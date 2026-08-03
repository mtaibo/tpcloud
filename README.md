# tpcloud

Personal infrastructure for migueltaibo.com. Monorepo grouping all services running on the server, orchestrated with Docker Compose and exposed to the internet via Cloudflare Tunnel.

## Structure

```
tpcloud/
├── auth/               # Passkey authentication service (FastAPI)
│   ├── app/
│   │   ├── main.py
│   │   ├── models.py
│   │   ├── database.py
│   │   ├── dependencies.py
│   │   ├── test.html           # Test page for registration/login
│   │   ├── routes/
│   │   │   ├── passkey.py      # WebAuthn endpoints (register, login, validate)
│   │   │   └── auth.py
│   │   └── scripts/
│   │       └── bootstrap.py    # Adds the owner email to allowed_emails
│   ├── Dockerfile
│   └── requirements.txt
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
| `OWNER_EMAIL` | Administrator email, automatically added to `allowed_emails` on bootstrap |
| `RP_ID` | WebAuthn Relying Party ID (e.g. `migueltaibo.com`) |
| `RP_NAME` | Display name of the passkey shown in the user's keychain |
| `ORIGIN` | Full URL where `navigator.credentials` is called (e.g. `https://login.migueltaibo.com`) |

## Services

### auth

Passkey-based authentication API (WebAuthn). Acts as a forward-auth provider for Caddy/Cloudflare: any protected service redirects the user to `login.migueltaibo.com`, where they authenticate with their passkey and receive a session cookie valid across all `migueltaibo.com` subdomains.

**Main endpoints:**

| Method | Path | Description |
|---|---|---|
| `GET` | `/health` | Service health check |
| `GET` | `/auth/passkey/validate` | Validates the session cookie (used as forward-auth) |
| `POST` | `/auth/passkey/register/begin` | Starts passkey registration |
| `POST` | `/auth/passkey/register/complete` | Completes registration |
| `POST` | `/auth/passkey/login/begin` | Starts login |
| `POST` | `/auth/passkey/login/complete` | Completes login and issues the session cookie |

**Bootstrap** — add the first authorized email:

```bash
docker compose exec auth python scripts/bootstrap.py
```

### cloudflared

Cloudflare Tunnel that exposes services to the internet without opening ports on the router. Tunnel routing (which subdomain points to which service) is managed from the Cloudflare dashboard.
