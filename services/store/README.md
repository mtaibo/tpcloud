<div align="center">

# TPStore

**My own storage and photo cloud, on my own disk.**

<br/>

![Status](https://img.shields.io/badge/status-active-brightgreen)
![Made with](https://img.shields.io/badge/made%20with-Python%20%7C%20Vue-blue)
![Runs on](https://img.shields.io/badge/runs%20on-Docker-lightgrey)

</div>

---

TPStore is the storage module of my personal cloud, [tpcloud](../../). It's my replacement for Google Drive and iCloud Photos: a file browser at `/store`, a gallery of my iPhone photos at `/photos`, public share links at `/share`, and the backend that my native app, [tpapp](../../apple/), backs up my iPhone photos to. Everything lives on an external disk plugged into my home server.

## Why I built it

I wanted my files and photos on hardware I own, without monthly storage plans or a company deciding what happens to them, but still with the comfort of a real cloud:

- **One place** — files, photos and shared folders behind the same login as the rest of my services
- **Big files** — chunked uploads that resume where they left off if the connection drops
- **Sharing that works** — links with a password, read-only or editable, for anyone without an account
- **Photo backup** — every photo from my iPhone, deduplicated and organised by date, with its metadata

## The modules

<table>
<tr>
<td width="33%" valign="top">

### ⚙️ [`services/store`](./)
FastAPI backend for everything: file operations, resumable uploads, thumbnails, light versions of photos and videos for share pages, ZIP compression and extraction, share links, photo backup and server backups. Validates every request against tpcloud's auth service.

**Python · FastAPI · SQLModel · Pillow · ffmpeg**

</td>
<td width="33%" valign="top">

### 🗂️ [`web/store`](../../web/store/)
The storage browser at `/store`, drawn after Finder on the desktop and Files on the phone, in light and dark: icon and list views with sortable columns, keyboard selection, Quick Look for photos and videos, folders with custom symbols, favourites, drag-and-drop uploads with a transfer panel, and share links managed from sheets. Its look comes from tpcloud's shared `ui/`.

**Vue 3 · Vite · Nginx**

</td>
<td width="33%" valign="top">

### 🔗 [`web/share`](../../web/share/)
The public viewer at `/share`, in the same look as the browser: the shared folder up top with Download all and Save to Photos, photos and videos in a grid like Photos, other files in a list like Files. It asks for the link's password if it has one, steps through photos and videos full screen, saves them straight to the phone's gallery and downloads them.

### 🖼️ [`web/photos`](../../web/photos/)
The gallery at `/photos`, a section of its own next to the store: every photo backed up from the iPhone, newest first and grouped by month, filtered to the camera's or screenshots, full screen with download and delete. Its tiles and previews are the same light versions as a share page, and the API makes every photo's thumbnail ahead of time so scrolling back years doesn't wait for them.

**Vue 3 · Vite · Nginx**

</td>
</tr>
</table>

## Architecture

```
       browser                 tpapp (iOS · macOS)            anyone with a link
          │                            │                              │
          └─────────────┬──────────────┘                              │
                        │ HTTPS                                       │ HTTPS
┌───────────────────────▼─────────────────────────────────────────────▼───────┐
│                       tpcloud gateway · Caddy                               │
│              /store and /store/api (login required) · /share                │
└──────────┬──────────────────────────┬─────────────────────────┬─────────────┘
           │                          │                         │
┌──────────▼──────────┐   ┌───────────▼───────────┐  ┌──────────▼──────────┐
│     frontend/       │   │         api/          │  │   share-frontend/   │
│  Vue 3 · Nginx      │   │  FastAPI · SQLite     │  │   Vue 3 · Nginx     │
└─────────────────────┘   └─────┬───────────┬─────┘  └─────────────────────┘
                                │           │
                   session check│           │ files
                                ▼           ▼
                       tpcloud auth    external disk
```

tpstore isn't a standalone deployment. It joins tpcloud's `tpcloud_gateway` Docker network, the gateway routes `/store`, `/share` and `/photos` to it (stripping the prefix, so the API still serves `/api/...`), and the API asks tpcloud's `auth` service who's behind every request. `share-frontend` and `photos-frontend` have no backend of their own: every request it makes is served by the same API, against the same database.

## How it works

### Storage layout

The disk is split into areas, and the API enforces who can touch each one. Admins get no wider access: nobody can browse the disk root or the server's own drives.

| Path | Who can access it |
|---|---|
| `users/<email>/` | Only that user — their personal files |
| `photos/<email>/YYYY/MM/` | Only that user, and only through the photo endpoints (upload, thumbnails, viewing), not the file browser |
| `shared/` | Every signed-in user |

### Uploads

Small files go up in one request. Anything over 10 MB is split into 32 MB chunks, and the server keeps track of every pending upload, so if the connection drops or the tab closes, the upload resumes from the last chunk instead of starting over.

### Share links

Any file or folder can get a link. Each one can have a password (hashed with bcrypt), be read-only or editable, use a readable slug or a random token, and be listed on the public landing page at `/share`. Only the share endpoints of the API are reachable from `/share`; everything else needs a session. Opening a protected link gives the visitor a short session, and every file inside is served through short-lived tokens, so the real path never leaves the server.

Phone media is too heavy to show as it is: an iPhone video is 4K 60 fps HDR HEVC at 80–100 Mbit/s, and HEIC photos don't open in most browsers. So share pages get light versions, made once and cached on disk:

| Original | Shown on the page | Made |
|---|---|---|
| Photo (HEIC, JPEG, PNG…) | Square JPEG thumbnail, and a 2048 px JPEG in the viewer | On request, in about a second |
| Video | Thumbnail cut from a poster frame, with its length | On request, in about a second |
| Video | 720p H.264 SDR MP4 (about 30× smaller) in the viewer | In the background, one at a time, as soon as the folder is opened; the original plays until it's ready |

Their URLs carry the file's version, so browsers keep them for good. Downloads, ZIPs (streamed, not built in memory) and **Save to Photos** always get the originals. Save to Photos hands the files to the phone's share sheet in parts of up to 300 MB, since a mobile browser can't hold a whole folder of videos at once.

### Photo backup

```
tpapp finds new photos in the iPhone library
        │
        ▼
Asks the API which ones it already has (by checksum and asset ID)
        │
        ▼
Uploads only the missing ones, in chunks
        │
        ▼
API verifies the checksum, reads the EXIF date and saves the metadata
(camera, location, dimensions, screenshot or original)
        │
        ▼
Photo is stored at photos/<email>/YYYY/MM/<checksum>.<ext>
```

Each photo is unique per user by checksum, so the same picture is never stored twice. An admin sanitize job, which runs in the background, moves old photos into the canonical layout and cleans up rows whose file has gone missing.

### Thumbnails and extras

- **Thumbnails** are generated on demand with Pillow (HEIC included), or from a poster frame with ffmpeg for videos, and cached on disk
- **ZIP** — download any folder as a ZIP, compress files, or extract an archive as a background job with progress
- **Server backup** — an admin button runs `rsync` from the main storage drive to a backup drive and keeps the last result

## Tech stack

| Layer | Technology |
|---|---|
| **API** | FastAPI · Python 3.12 · Uvicorn |
| **Database** | SQLite via SQLModel |
| **Images and video** | Pillow · pillow-heif · ffmpeg |
| **Frontends** | Vue 3 · Vite |
| **Auth** | tpcloud's auth service · bcrypt for share passwords |
| **Hosting** | Docker Compose · Nginx · behind the tpcloud gateway |

## How to run

Store is part of tpcloud: it runs from the [compose file at the root](../../docker-compose.yml), behind the `store` profile (`COMPOSE_PROFILES=store` in `.env`, which the installer sets), with its database and thumbnails in `data/store/` and users' files in `EXTERNAL_DISK_PATH`. See tpcloud's [How to run](../../README.md#how-to-run).

No routing to configure here: the tpcloud gateway reaches `store-api`, `store-frontend`, `share-frontend` and `photos-frontend` by their service names on the shared network.
