# Everything a TPCloud server runs, without Docker on the server: packed into runtime.tar.zst,
# which the tpcloud binary carries inside. Built by cli/build-runtime.sh from a context holding the
# tracked files of tpcloud/ and tphome/ next to this file. Docker is only needed here, to build.
#
#   runtime/
#     python/            a portable CPython (python-build-standalone)
#     py/{auth,store,home}/   each service's pinned packages (pip --target)
#     code/{auth,store,home}/ each service's code
#     web/<app>/         the seven web apps, built
#     bin/               caddy, mosquitto, cloudflared, ffmpeg, ffprobe, rsync (static)
#     Caddyfile, mosquitto.conf

ARG PY_RELEASE=20261110
ARG PY_VERSION=3.12.16
ARG CADDY_VERSION=2.11.9
ARG CLOUDFLARED_VERSION=2026.11.0
ARG MOSQUITTO_VERSION=2.1.2
ARG RSYNC_VERSION=3.5.2
ARG FFMPEG_VERSION=8.0
ARG ZIMG_VERSION=3.0.6

# ── Mosquitto, static (no TLS, websockets or plugins: the switches speak plain MQTT on the LAN) ──
FROM alpine:3.22 AS mosquitto
ARG MOSQUITTO_VERSION
RUN apk add --no-cache build-base cmake curl linux-headers cjson-dev cjson-static
RUN curl -fsSL https://mosquitto.org/files/source/mosquitto-${MOSQUITTO_VERSION}.tar.gz | tar -xz -C /tmp
WORKDIR /tmp/mosquitto-${MOSQUITTO_VERSION}
# musl has getrandom(), but 2.1 only looks for it through glibc's version macro.
RUN cmake -S . -B build -DCMAKE_BUILD_TYPE=Release \
      -DWITH_TLS=OFF -DWITH_TLS_PSK=OFF -DWITH_WEBSOCKETS=OFF -DWITH_STATIC_LIBRARIES=ON \
      -DWITH_CLIENTS=OFF -DWITH_APPS=OFF -DWITH_PLUGINS=OFF -DWITH_DOCS=OFF -DWITH_TESTS=OFF \
      -DWITH_CTRL_SHELL=OFF -DWITH_SRV=OFF -DCJSON_LIBRARY=/usr/lib/libcjson.a \
      "-DCMAKE_C_FLAGS=-DHAVE_GETRANDOM=1 -include sys/random.h" -DCMAKE_EXE_LINKER_FLAGS=-static \
 && cmake --build build --target mosquitto -j"$(nproc)" \
 && strip build/src/mosquitto && cp build/src/mosquitto /mosquitto \
 && /mosquitto -h | head -1

# ── rsync, static (Store's backup) ──
FROM alpine:3.22 AS rsync
ARG RSYNC_VERSION
RUN apk add --no-cache build-base curl perl acl-dev acl-static attr-dev popt-dev popt-static zlib-dev zlib-static
RUN curl -fsSL https://download.samba.org/pub/rsync/src/rsync-${RSYNC_VERSION}.tar.gz | tar -xz -C /tmp
WORKDIR /tmp/rsync-${RSYNC_VERSION}
RUN ./configure --disable-openssl --disable-xxhash --disable-zstd --disable-lz4 --disable-md2man --disable-idn \
      LDFLAGS="-static" >/dev/null \
 && make -j"$(nproc)" >/dev/null && strip rsync && cp rsync /rsync && /rsync --version | head -1

# ── ffmpeg and ffprobe, static, with only what Store uses (~10 MB each instead of ~125): read
#    iPhone HEVC/H.264 clips, tone-map HDR with zscale, write JPEG posters and H.264/AAC previews ──
FROM alpine:3.22 AS ffmpeg
ARG FFMPEG_VERSION
ARG ZIMG_VERSION
RUN apk add --no-cache build-base nasm curl pkgconf autoconf automake libtool linux-headers bash xz
WORKDIR /tmp
RUN curl -fsSL https://github.com/sekrit-twc/zimg/archive/refs/tags/release-${ZIMG_VERSION}.tar.gz | tar -xz \
 && cd zimg-release-${ZIMG_VERSION} && ./autogen.sh >/dev/null 2>&1 \
 && ./configure --enable-static --disable-shared --prefix=/usr/local >/dev/null && make -j"$(nproc)" >/dev/null && make install >/dev/null
RUN curl -fsSL https://code.videolan.org/videolan/x264/-/archive/stable/x264-stable.tar.gz | tar -xz \
 && cd x264-stable && ./configure --enable-static --disable-cli --enable-pic --prefix=/usr/local >/dev/null \
 && make -j"$(nproc)" >/dev/null && make install >/dev/null
RUN curl -fsSL https://ffmpeg.org/releases/ffmpeg-${FFMPEG_VERSION}.tar.xz | tar -xJ && cd ffmpeg-${FFMPEG_VERSION} \
 && PKG_CONFIG_PATH=/usr/local/lib/pkgconfig ./configure --prefix=/usr/local \
      --disable-everything --disable-autodetect --disable-doc --disable-network --disable-debug --disable-ffplay \
      --enable-static --disable-shared --pkg-config-flags=--static --extra-ldflags=-static \
      --enable-gpl --enable-libx264 --enable-libzimg \
      --enable-protocol=file,pipe \
      --enable-demuxer=mov,matroska,avi,mpegts,image2,mjpeg,mp3,wav,aac \
      --enable-decoder=hevc,h264,mpeg4,vp8,vp9,mjpeg,png,prores,aac,mp3,pcm_s16le,pcm_s24le,ac3,eac3,opus,alac \
      --enable-parser=hevc,h264,aac,mjpeg,mpeg4video,vp8,vp9,ac3,opus \
      --enable-encoder=libx264,aac,mjpeg \
      --enable-muxer=mp4,image2,mjpeg,null \
      --enable-filter=zscale,tonemap,scale,format,fps,null,anull,aformat,aresample,setsar,setpts,copy \
      --enable-bsf=hevc_mp4toannexb,h264_mp4toannexb,aac_adtstoasc >/dev/null \
 && make -j"$(nproc)" >/dev/null 2>&1 && strip ffmpeg ffprobe && cp ffmpeg ffprobe / && /ffmpeg -version | head -1

# ── The web apps ──
FROM node:20-alpine AS web
WORKDIR /src
COPY tpcloud/web ./
RUN for app in login accounts cloud home store photos share; do \
      (cd $app && npm install --no-audit --no-fund --loglevel=error && npm run build >/dev/null) || exit 1; \
    done

# ── Python and every service's packages ──
FROM debian:bookworm-slim AS python
ARG TARGETARCH
ARG PY_RELEASE
ARG PY_VERSION
RUN apt-get update -qq && apt-get install -qq -y --no-install-recommends curl ca-certificates >/dev/null
RUN arch=$([ "$TARGETARCH" = arm64 ] && echo aarch64 || echo x86_64) \
 && curl -fsSL "https://github.com/astral-sh/python-build-standalone/releases/download/${PY_RELEASE}/cpython-${PY_VERSION}+${PY_RELEASE}-${arch}-unknown-linux-gnu-install_only_stripped.tar.gz" \
    | tar -xz -C /opt
COPY tpcloud/services/auth/requirements.txt /req/auth.txt
COPY tpcloud/services/store/requirements.txt /req/store.txt
COPY tphome/api/requirements.txt /req/home.txt
# Binary wheels only: nothing gets compiled on the way, so what's packed is what pip downloaded.
RUN for svc in auth store home; do \
      /opt/python/bin/python3 -m pip install -q --no-compile --only-binary=:all: --target /opt/py/$svc -r /req/$svc.txt || exit 1; \
    done && find /opt/py -name "__pycache__" -prune -exec rm -rf {} +

# ── Assemble ──
FROM debian:bookworm-slim AS assemble
ARG TARGETARCH
ARG CADDY_VERSION
ARG CLOUDFLARED_VERSION
RUN apt-get update -qq && apt-get install -qq -y --no-install-recommends curl ca-certificates zstd binutils python3 >/dev/null
WORKDIR /runtime/bin
RUN curl -fsSL "https://github.com/caddyserver/caddy/releases/download/v${CADDY_VERSION}/caddy_${CADDY_VERSION}_linux_${TARGETARCH}.tar.gz" | tar -xz caddy \
 && curl -fsSL -o cloudflared "https://github.com/cloudflare/cloudflared/releases/download/${CLOUDFLARED_VERSION}/cloudflared-linux-${TARGETARCH}" && chmod +x cloudflared
COPY --from=ffmpeg /ffmpeg /ffprobe ./
COPY --from=mosquitto /mosquitto ./mosquitto
COPY --from=rsync /rsync ./rsync
COPY --from=python /opt/python /runtime/python
COPY --from=python /opt/py /runtime/py
COPY --from=web /src/login/dist /runtime/web/login
COPY --from=web /src/accounts/dist /runtime/web/accounts
COPY --from=web /src/cloud/dist /runtime/web/cloud
COPY --from=web /src/home/dist /runtime/web/home
COPY --from=web /src/store/dist /runtime/web/store
COPY --from=web /src/photos/dist /runtime/web/photos
COPY --from=web /src/share/dist /runtime/web/share
COPY tpcloud/services/auth/app /runtime/code/auth
COPY tpcloud/services/store/app /runtime/code/store/app
COPY tphome/api/app /runtime/code/home
COPY tpcloud/services/gateway/Caddyfile /runtime/Caddyfile
# Slimmer: stdlib parts no service uses, unneeded symbols in the packages' native libraries, and
# every file the three services' packages share stored once (hard links, which tar keeps).
RUN rm -rf /runtime/python/lib/python3.12/ensurepip /runtime/python/lib/python3.12/idlelib \
      /runtime/python/lib/python3.12/tkinter /runtime/python/lib/python3.12/turtledemo \
      /runtime/python/lib/python3.12/test /runtime/python/lib/python3.12/lib2to3 \
      /runtime/python/lib/python3.12/site-packages/pip* /runtime/python/lib/libtcl* /runtime/python/lib/libtk* \
      /runtime/python/lib/tcl* /runtime/python/lib/tk* \
 && find /runtime/py -name "*.so*" -type f -exec strip --strip-unneeded {} + 2>/dev/null; \
    python3 - <<'PY'
import hashlib, os
seen = {}
for root, _, files in os.walk("/runtime/py"):
    for name in sorted(files):
        path = os.path.join(root, name)
        if os.path.islink(path):
            continue
        with open(path, "rb") as f:
            key = (os.path.getsize(path), hashlib.sha256(f.read()).hexdigest())
        if key in seen:
            os.remove(path)
            os.link(seen[key], path)
        else:
            seen[key] = path
PY
RUN find /runtime/code -name "__pycache__" -prune -exec rm -rf {} + \
 && cd / && tar --sort=name --owner=0 --group=0 --numeric-owner --mtime='2026-01-01 00:00Z' -cf - runtime \
    | zstd -19 -T0 --long=27 -q -o /runtime.tar.zst \
 && ls -la /runtime.tar.zst && du -sh /runtime /runtime/*

FROM scratch
COPY --from=assemble /runtime.tar.zst /
