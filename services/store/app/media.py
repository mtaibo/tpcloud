"""Light versions of photos and videos for share pages, built once and kept in DATA_DIR/media.

Phone media is far too heavy to show as-is to someone opening a share link: an iPhone video is
4K 60 fps HEVC 10-bit HDR at 80-100 Mbit/s, which no connection streams smoothly from a home
uplink and most browsers outside Apple's can't even decode, and a HEIC photo doesn't open at all
in Chrome. So each one gets:

- a poster frame (videos), which square thumbnails are cut from instead of loading the video;
- a 2048 px JPEG (photos), for the viewer;
- a 720p H.264 SDR MP4 with faststart (videos), about 30x smaller than the original.

Thumbnails, posters and photo previews take a second and are made on request. Video previews
take about 6x the clip's length on the Pi, so they're made one at a time in the background,
queued when a share folder is listed; until one is ready the original is served instead.
Downloads always get the untouched original.
"""

import asyncio
import hashlib
import json
import logging
import os
import shutil
from pathlib import Path

from fastapi.concurrency import run_in_threadpool

from app.database import DATA_DIR

log = logging.getLogger(__name__)

MEDIA_DIR = DATA_DIR / "media"

IMAGE_EXTS = {".jpg", ".jpeg", ".png", ".gif", ".webp", ".bmp", ".tiff", ".heic", ".heif"}
VIDEO_EXTS = {".mp4", ".mov", ".m4v", ".mkv", ".avi", ".webm"}

FFMPEG = shutil.which("ffmpeg")
FFPROBE = shutil.which("ffprobe")

PREVIEW_IMAGE_SIZE = 2048
POSTER_SIZE = 1280

# Posters and photo previews someone is waiting for: a couple at a time, so a burst of requests
# from a gallery can't take every core away from the API.
_image_sem = asyncio.Semaphore(2)
# Warming a folder up goes one file at a time and at low priority, behind those requests: it
# used to start all of a folder's files at once, which kept the Pi's cores busy for minutes and
# left a visitor's own photo waiting behind the whole folder.
_warm_lane = asyncio.Semaphore(1)
_warming: set[Path] = set()
_in_flight: dict[str, asyncio.Future] = {}

_video_queue: list[Path] = []
_video_queued: set[str] = set()
_video_wakeup: asyncio.Event | None = None
_video_worker: asyncio.Task | None = None
_background: set[asyncio.Task] = set()

# Tone-maps HLG/PQ (what iPhones record) down to SDR, after scaling so the float maths runs on
# 720p rather than 4K. Without it HDR clips come out grey and washed out.
_TONEMAP = (
    "zscale=t=linear:npl=100,format=gbrpf32le,zscale=p=bt709,"
    "tonemap=hable:desat=0,zscale=t=bt709:m=bt709:r=tv"
)
_HDR_TRANSFERS = {"arib-std-b67", "smpte2084"}


def is_image(path: Path) -> bool:
    return path.suffix.lower() in IMAGE_EXTS


def is_video(path: Path) -> bool:
    return path.suffix.lower() in VIDEO_EXTS


def _cache_path(file_path: Path, kind: str, ext: str) -> Path:
    mtime = int(file_path.stat().st_mtime)
    key = hashlib.sha256(f"{file_path}|{mtime}|{kind}".encode()).hexdigest()[:24]
    return MEDIA_DIR / f"{key}.{ext}"


async def _once(target: Path, build) -> Path:
    """Builds `target` with `build()` unless it exists, sharing a build already running for it."""
    if target.exists():
        return target
    key = str(target)
    if key in _in_flight:
        await _in_flight[key]
        return target
    future = asyncio.get_running_loop().create_future()
    _in_flight[key] = future
    try:
        MEDIA_DIR.mkdir(parents=True, exist_ok=True)
        await build()
        future.set_result(None)
    except Exception as error:
        future.set_exception(error)
        future.exception()  # marks it retrieved when nobody else was waiting on it
        raise
    finally:
        _in_flight.pop(key, None)
    return target


async def _run(*args: str) -> bytes:
    process = await asyncio.create_subprocess_exec(
        *args, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
    )
    stdout, stderr = await process.communicate()
    if process.returncode != 0:
        raise RuntimeError(f"{Path(args[0]).name} failed: {stderr.decode(errors='replace')[-500:]}")
    return stdout


# ── Video info ────────────────────────────────────────────────────────────────

async def video_info(file_path: Path) -> dict:
    """Duration, display size and whether it's HDR, from ffprobe; cached next to the poster."""
    info_path = _cache_path(file_path, "info", "json")

    async def build():
        out = await _run(
            FFPROBE, "-v", "error", "-select_streams", "v:0",
            "-show_entries", "stream=width,height,color_transfer:stream_side_data=rotation:format=duration",
            "-of", "json", str(file_path),
        )
        data = json.loads(out)
        stream = (data.get("streams") or [{}])[0]
        width, height = stream.get("width") or 0, stream.get("height") or 0
        rotation = next((int(s.get("rotation", 0)) for s in stream.get("side_data_list", []) if "rotation" in s), 0)
        if abs(rotation) % 180 == 90:
            width, height = height, width
        info = {
            "duration": float(data.get("format", {}).get("duration") or 0),
            "width": width,
            "height": height,
            "hdr": stream.get("color_transfer") in _HDR_TRANSFERS,
        }
        tmp = info_path.with_suffix(".tmp")
        tmp.write_text(json.dumps(info))
        tmp.replace(info_path)

    await _once(info_path, build)
    return json.loads(info_path.read_text())


def cached_video_info(file_path: Path) -> dict | None:
    """The info if it's been worked out already, without probing (for listings)."""
    try:
        return json.loads(_cache_path(file_path, "info", "json").read_text())
    except (OSError, ValueError):
        return None


# ── Posters and photo previews ────────────────────────────────────────────────

async def ensure_video_poster(file_path: Path, background: bool = False) -> Path:
    if not FFMPEG:
        raise RuntimeError("ffmpeg isn't installed")
    poster = _cache_path(file_path, f"poster{POSTER_SIZE}", "jpg")

    async def build():
        info = await video_info(file_path)
        # A second in skips the black first frame many clips start with; short ones use their middle.
        at = min(1.0, info["duration"] / 2) if info["duration"] else 0
        scale = f"scale=w={POSTER_SIZE}:h={POSTER_SIZE}:force_original_aspect_ratio=decrease"
        vf = f"{scale},{_TONEMAP},format=yuvj420p" if info["hdr"] else f"{scale},format=yuvj420p"
        tmp = poster.with_suffix(".tmp.jpg")
        nice = ["nice", "-n", "10"] if background else []
        async with _warm_lane if background else _image_sem:
            await _run(*nice, FFMPEG, "-v", "error", "-threads", "2", "-ss", f"{at:.2f}", "-i", str(file_path),
                       "-frames:v", "1", "-vf", vf, "-q:v", "3", "-y", str(tmp))
        tmp.replace(poster)

    return await _once(poster, build)


def _build_image_preview(file_path: Path, target: Path) -> None:
    from PIL import Image, ImageOps
    with Image.open(file_path) as img:
        img = ImageOps.exif_transpose(img).convert("RGB")
        img.thumbnail((PREVIEW_IMAGE_SIZE, PREVIEW_IMAGE_SIZE), Image.LANCZOS)
        tmp = target.with_suffix(".tmp.jpg")
        img.save(str(tmp), format="JPEG", quality=82, optimize=True, progressive=True)
        tmp.replace(target)


async def ensure_image_preview(file_path: Path, background: bool = False) -> Path:
    target = _cache_path(file_path, f"preview{PREVIEW_IMAGE_SIZE}", "jpg")

    async def build():
        async with _warm_lane if background else _image_sem:
            await run_in_threadpool(_build_image_preview, file_path, target)

    return await _once(target, build)


# ── Video previews ────────────────────────────────────────────────────────────

def video_preview_path(file_path: Path) -> Path:
    return _cache_path(file_path, "preview720", "mp4")


def ready_video_preview(file_path: Path) -> Path | None:
    path = video_preview_path(file_path)
    return path if path.exists() else None


def queue_video_preview(file_path: Path, first: bool = False) -> None:
    """Queues the 720p version; `first` moves it to the front (someone is waiting to watch it)."""
    global _video_worker, _video_wakeup
    if not FFMPEG or video_preview_path(file_path).exists():
        return
    key = str(file_path)
    if key in _video_queued:
        if first and file_path in _video_queue:
            _video_queue.remove(file_path)
            _video_queue.insert(0, file_path)
        return
    _video_queued.add(key)
    if first:
        _video_queue.insert(0, file_path)
    else:
        _video_queue.append(file_path)
    if _video_wakeup is None:
        _video_wakeup = asyncio.Event()
    if _video_worker is None or _video_worker.done():
        _video_worker = asyncio.get_running_loop().create_task(_video_loop())
    _video_wakeup.set()


async def _video_loop() -> None:
    while True:
        if not _video_queue:
            _video_wakeup.clear()
            await _video_wakeup.wait()
            continue
        file_path = _video_queue.pop(0)
        try:
            if file_path.exists():
                await _transcode(file_path)
        except Exception:
            log.exception("Couldn't make a preview of %s", file_path)
        finally:
            _video_queued.discard(str(file_path))


async def _transcode(file_path: Path) -> None:
    target = video_preview_path(file_path)
    if target.exists():
        return
    MEDIA_DIR.mkdir(parents=True, exist_ok=True)
    info = await video_info(file_path)
    scale = "fps=30,scale=w=1280:h=1280:force_original_aspect_ratio=decrease:force_divisible_by=2"
    vf = f"{scale},{_TONEMAP},format=yuv420p" if info["hdr"] else f"{scale},format=yuv420p"
    tmp = target.with_suffix(".tmp.mp4")
    # Lowest priority and three of the four cores, so the API and everything else on the Pi stay
    # responsive while a batch of clips is converted.
    await _run(
        "nice", "-n", "19", FFMPEG, "-v", "error", "-threads", "3", "-i", str(file_path),
        "-map", "0:v:0", "-map", "0:a:0?", "-vf", vf,
        "-c:v", "libx264", "-preset", "veryfast", "-crf", "24", "-threads", "3",
        "-c:a", "aac", "-b:a", "128k", "-movflags", "+faststart", "-y", str(tmp),
    )
    tmp.replace(target)


# ── Warming up a folder ───────────────────────────────────────────────────────

def warm_up(files: list[Path], thumbnail_sizes: list[int]) -> None:
    """Starts making everything a share page will ask for, so it's ready by the time it does.
    A folder already warming up isn't started again when someone else opens it."""
    if not files or files[0].parent in _warming:
        return
    folder = files[0].parent
    _warming.add(folder)
    task = asyncio.get_running_loop().create_task(_warm_up(files, thumbnail_sizes))
    _background.add(task)
    task.add_done_callback(lambda t: (_background.discard(t), _warming.discard(folder)))


async def _warm_up(files: list[Path], thumbnail_sizes: list[int]) -> None:
    from app.routes.files import _thumb_cache_path, _ensure_thumbnail, _THUMB_DIR
    _THUMB_DIR.mkdir(parents=True, exist_ok=True)

    # In page order, one file at a time: every thumbnail first (what the page shows straight
    # away), then the photos' viewer versions. Videos join the transcode queue as they go.
    for file_path in files:
        try:
            if is_video(file_path) and FFMPEG:
                poster = await ensure_video_poster(file_path, background=True)
                for size in thumbnail_sizes:
                    async with _warm_lane:
                        await _ensure_thumbnail(poster, _thumb_cache_path(poster, size), size)
                queue_video_preview(file_path)
            elif is_image(file_path):
                for size in thumbnail_sizes:
                    async with _warm_lane:
                        await _ensure_thumbnail(file_path, _thumb_cache_path(file_path, size), size)
        except Exception:
            log.exception("Couldn't prepare %s", file_path)
    for file_path in files:
        if is_image(file_path):
            try:
                await ensure_image_preview(file_path, background=True)
            except Exception:
                log.exception("Couldn't prepare %s", file_path)


def cache_headers(private: bool) -> dict:
    """For responses whose URL carries the file's version (`v=`), so they never go stale."""
    return {"Cache-Control": f"{'private' if private else 'public'}, max-age=2592000, immutable"}
