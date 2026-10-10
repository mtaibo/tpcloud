# tpstore

Personal cloud storage and file-sharing module, extracted from the [tpcloud](https://github.com/mtaibo/tpcloud) monorepo. It is **not** a standalone deployment — it runs alongside `tpcloud` and depends on it for authentication and network access.

## Architecture

```
┌─────────────────────────┐   ┌──────────────────────────┐
│       frontend/          │   │     share-frontend/        │
│   Vue 3 · storage UI      │   │   Vue 3 · public share UI   │
└────────────┬─────────────┘   └────────────┬───────────────┘
             │ HTTP                          │ HTTP
             └──────────────┬────────────────┘
                             ▼
                   ┌───────────────────┐
                   │       api/         │
                   │  FastAPI · SQLite  │
                   └─────────┬──────────┘
                             │ HTTP (session validation)
                             ▼
                 tpcloud's `auth` service
                 (shared `tpcloud_gateway` network)
```

`share-frontend` has no backend of its own — every request it makes (`/api/share/*`, `/api/open/*`) is served by `api/`, against the same database as the main storage UI.

## Modules

| Module | Description |
|---|---|
| [`api/`](api/) | FastAPI backend. File browser, uploads, thumbnails, ZIP extraction, and share-link management. Validates sessions against `tpcloud`'s `auth` service over HTTP. |
| [`frontend/`](frontend/) | Vue 3 SPA — the main storage browser, served by Nginx. |
| [`share-frontend/`](share-frontend/) | Vue 3 SPA — public share-link viewer, served by Nginx. No authentication required. |

## Requirements

- `tpcloud` already running on the same host — this creates the `tpcloud_gateway` Docker network and the `auth` service that `store-api` depends on.
- Docker and Docker Compose.

## How to run

```bash
cp .env.example .env
# fill in EXTERNAL_DISK_PATH to match the value used in tpcloud's .env
docker compose up --build -d
```

Start `tpcloud` first — the `tpcloud_gateway` network and `auth` service must already exist, otherwise `docker compose up` will fail to attach the external network, and `store-api` won't be able to validate sessions.

Routing (`store.migueltaibo.com`, `share.migueltaibo.com`) is handled by `tpcloud`'s Caddy gateway, which reaches these containers by their Compose service names (`store-api`, `store-frontend`, `share-frontend`) on the shared network — no configuration needed here.

## Tech stack

| Layer | Technology |
|---|---|
| **API framework** | FastAPI · Python 3.12 |
| **API database** | SQLite via SQLModel |
| **Frontend framework** | Vue 3 |
| **Hosting** | Docker + Nginx, behind tpcloud's Caddy gateway |
