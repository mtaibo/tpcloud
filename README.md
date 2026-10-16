# tpcloud

Personal infrastructure for migueltaibo.com. Monorepo grouping all services running on the server, orchestrated with Docker Compose and exposed to the internet via Cloudflare Tunnel.

## Structure

```
tpcloud/
├── auth/               # Passkey/password+TOTP authentication service (FastAPI)
│   ├── app/
│   │   ├── main.py
│   │   ├── models.py
│   │   ├── database.py
│   │   ├── dependencies.py
│   │   ├── test.html           # Test page for registration/login
│   │   ├── routes/
│   │   │   ├── passkey.py      # WebAuthn endpoints (register, login, validate)
│   │   │   ├── password_auth.py # Password + TOTP login
│   │   │   ├── account.py      # Self-service profile/passkeys/sessions
│   │   │   ├── totp.py         # TOTP setup/status
│   │   │   └── admin.py        # Admin-only user/session management
│   │   └── scripts/
│   │       └── bootstrap.py    # Creates (or promotes) the owner account
│   ├── Dockerfile
│   └── requirements.txt
├── login/              # Login UI (Vue 3) — login.migueltaibo.com
├── cloud/              # Authenticated landing hub (Vue 3) — cloud.migueltaibo.com
├── accounts/           # Account self-service UI (Vue 3) — accounts.migueltaibo.com
├── gateway/            # Caddy reverse proxy + forward-auth gate
│   └── Caddyfile
├── docker-compose.yml  # Service orchestration
├── .env.example        # Environment variables (copy to .env and fill in)
└── .gitignore
```

## Requirements

- Docker and Docker Compose
- A Cloudflare account with a configured tunnel

## Getting started

```bash
cp .env.example .env
# Edit .env with real values
docker compose up -d
```

## Environment variables

| Variable | Description |
|---|---|
| `TUNNEL_TOKEN` | Cloudflare tunnel token |
| `DATABASE_URL` | Database URL (defaults to local SQLite) |
| `OWNER_EMAIL` | Administrator email, created (or promoted to admin) on bootstrap |
| `OWNER_PASSWORD` | Password set for the owner account on bootstrap |
| `RP_ID` | WebAuthn Relying Party ID (e.g. `migueltaibo.com`) |
| `RP_NAME` | Display name of the passkey shown in the user's keychain |
| `ORIGIN` | Full URL where `navigator.credentials` is called for login (e.g. `https://login.migueltaibo.com`) |
| `ACCOUNTS_ORIGIN` | Full URL where `navigator.credentials` is called from the accounts UI (e.g. `https://accounts.migueltaibo.com`) |

## Services

### auth

Authentication API (WebAuthn passkeys, plus password+TOTP as a fallback login method). Acts as a forward-auth provider for Caddy: any protected service redirects the user to `login.migueltaibo.com`, where they authenticate and receive a session cookie valid across all `migueltaibo.com` subdomains. Sessions last 24h for native app clients (`X-TP-Client: ios/macos`) and 1h for web sessions.

**Main endpoints:**

| Method | Path | Description |
|---|---|---|
| `GET` | `/health` | Service health check |
| `GET` | `/auth/passkey/validate` | Validates the session cookie (used as forward-auth) |
| `GET` | `/auth/passkey/me` | Current session's user profile |
| `POST` | `/auth/passkey/register/begin` \| `/complete` | Passkey registration |
| `POST` | `/auth/passkey/login/begin` \| `/complete` | Passkey login |
| `POST` | `/auth/password/login` | Password login |
| `POST` | `/auth/password/totp/verify` | TOTP verification step |
| `GET`/`PATCH` | `/auth/account/profile` | View/update the current user's profile |
| `POST`/`DELETE` | `/auth/account/passkey/*` | Manage the current user's passkeys |
| `GET`/`DELETE` | `/auth/account/session/{id}` | List/revoke the current user's sessions |
| `*` | `/auth/account/totp/*` | TOTP setup/status for the current user |
| `GET`/`POST`/`PATCH`/`DELETE` | `/auth/admin/users` | Admin-only user management |
| `GET`/`DELETE` | `/auth/admin/sessions` | Admin-only session management |

**Bootstrap** — create or promote the owner account:

```bash
docker compose exec auth python scripts/bootstrap.py
```

This creates the `OWNER_EMAIL` user (with `OWNER_PASSWORD`) if it doesn't exist, or promotes it to admin if it does.

### login / cloud / accounts

Vue 3 single-page apps, served by nginx:

- **login** — password/TOTP/passkey login form; handles the `?redirect=` flow coming from the forward-auth gate.
- **cloud** — landing hub at `cloud.migueltaibo.com` linking out to the other services (Store, Home, Accounts).
- **accounts** — self-service profile, passkeys, active sessions, and (admin-only) user management, all against `auth`'s `/auth/account/*` and `/auth/admin/*` endpoints.

### gateway

Caddy reverse proxy and forward-auth gate (`gateway/Caddyfile`). Routes each `*.migueltaibo.com` subdomain to its container, including sibling projects that join the same `gateway` Docker network under well-known service names: `store-api`/`store-frontend`/`share-frontend` (from the separate `tpstore` repo) and `tphome-api`/`tphome-frontend`. `admin.migueltaibo.com` is proxied to a `tpadmin` process running on the host (outside Docker).

### cloudflared

Cloudflare Tunnel that exposes services to the internet without opening ports on the router. Tunnel routing (which subdomain points to which service) is managed from the Cloudflare dashboard.
