<div align="center">

# TPStore

**My own storage and photo cloud, on my own disk.**

<br/>

![Status](https://img.shields.io/badge/status-active-brightgreen)
![Made with](https://img.shields.io/badge/made%20with-Python%20%7C%20Vue-blue)
![Runs on](https://img.shields.io/badge/runs%20on-Docker-lightgrey)

</div>

---

TPStore is the storage module of my personal cloud, [tpcloud](https://github.com/mtaibo/tpcloud). It's my replacement for Google Drive and iCloud Photos: a file browser at `store.migueltaibo.com`, public share links at `share.migueltaibo.com`, and the backend that my native app, [tpapp](https://github.com/mtaibo/tpapp), backs up my iPhone photos to. Everything lives on an external disk plugged into my home server.

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

### ⚙️ [`api`](api/)
FastAPI backend for everything: file operations, resumable uploads, thumbnails, ZIP compression and extraction, share links, photo backup and server backups. Validates every request against tpcloud's auth service.

**Python · FastAPI · SQLModel · Pillow**

</td>
<td width="33%" valign="top">

### 🗂️ [`frontend`](frontend/)
The storage browser at `store.migueltaibo.com`. Folders with custom icons, favourites, drag-and-drop uploads with a transfer panel, image, video and audio viewers, share management, and a mobile layout.

**Vue 3 · Vite · Nginx**

</td>
<td width="33%" valign="top">

### 🔗 [`share-frontend`](share-frontend/)
The public viewer at `share.migueltaibo.com`. Opens a share link, asks for its password if it has one, and lets anyone browse, preview and download it, or upload into it if it's editable.

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
│                tpcloud gateway · Caddy + Cloudflare Tunnel                  │
│        store.migueltaibo.com (login required) · share.migueltaibo.com       │
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

tpstore isn't a standalone deployment. It joins tpcloud's `tpcloud_gateway` Docker network, the gateway routes both subdomains to it, and the API asks tpcloud's `auth` service who's behind every request. `share-frontend` has no backend of its own: every request it makes is served by the same API, against the same database.

## How it works

### Storage layout

The disk is split into areas, and the API enforces who can touch each one:

| Path | Who can access it |
|---|---|
| `users/<email>/` | Only that user — their personal files |
| `photos/<email>/YYYY/MM/` | Only that user — their backed-up photos |
| `shared/` | Every signed-in user |
| Server drives (`/mnt` on the host) | Admins only |

### Uploads

Small files go up in one request. Anything over 10 MB is split into 32 MB chunks, and the server keeps track of every pending upload, so if the connection drops or the tab closes, the upload resumes from the last chunk instead of starting over.

### Share links

Any file or folder can get a link. Each one can have a password (hashed with bcrypt), be read-only or editable, use a readable slug or a random token, and be listed on the public landing page of `share.migueltaibo.com`. Opening a protected link gives the visitor a short session, and every file inside is served through short-lived tokens, so the real path never leaves the server.

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

- **Thumbnails** are generated on demand with Pillow (HEIC included) and cached on disk
- **ZIP** — download any folder as a ZIP, compress files, or extract an archive as a background job with progress
- **Server backup** — an admin button runs `rsync` from the main storage drive to a backup drive and keeps the last result

## Tech stack

| Layer | Technology |
|---|---|
| **API** | FastAPI · Python 3.12 · Uvicorn |
| **Database** | SQLite via SQLModel |
| **Images** | Pillow · pillow-heif |
| **Frontends** | Vue 3 · Vite |
| **Auth** | tpcloud's auth service · bcrypt for share passwords |
| **Hosting** | Docker Compose · Nginx · behind the tpcloud gateway |

## How to run

[tpcloud](https://github.com/mtaibo/tpcloud) has to be running on the same host first, since it creates the `tpcloud_gateway` network and the `auth` service that tpstore depends on.

```bash
git clone https://github.com/mtaibo/tpstore
cd tpstore
cp .env.example .env    # set EXTERNAL_DISK_PATH to the disk that holds the files
docker compose up --build -d
```

No routing to configure here: the tpcloud gateway reaches `store-api`, `store-frontend` and `share-frontend` by their service names on the shared network.
