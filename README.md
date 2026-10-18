<div align="center">

# TPCloud

**The gateway to my personal cloud.**

<br/>

![Status](https://img.shields.io/badge/status-active-brightgreen)
![Made with](https://img.shields.io/badge/made%20with-Python%20%7C%20Vue-blue)
![Runs on](https://img.shields.io/badge/runs%20on-Docker%20%7C%20Cloudflare-lightgrey)

</div>

---

TPCloud is the core of everything I self-host under `migueltaibo.com`. It's the front door that every request goes through: one gateway exposed to the internet through a Cloudflare Tunnel, one auth service that signs me in once for every subdomain, and the small web apps around it to log in, manage accounts and jump between services. The rest of my projects plug into it as modules.

## Why I built it

Every service I added to my server needed its own login, its own port and its own way of being reached from outside. I wanted it to feel like a single cloud instead:

- **One login** — sign in once at `login.migueltaibo.com` and every subdomain knows who you are
- **No open ports** — the server is only reachable through a Cloudflare Tunnel, my router stays closed
- **Plug in a module** — a new service only needs a container on the shared network and a block in the gateway
- **Modern auth** — passkeys on the web, password + TOTP for the native app, and sessions I can see and revoke

## The ecosystem

<table>
<tr>
<td width="50%" valign="top">

### 🔐 [`auth`](auth/)
The authentication service behind everything. Passkeys (WebAuthn), password + TOTP two-factor, a session cookie shared across all subdomains, and the forward-auth endpoint the gateway calls before letting a request through.

**Python · FastAPI · SQLModel · webauthn · pyotp**

</td>
<td width="50%" valign="top">

### 🌐 [`gateway`](gateway/)
Caddy routes each subdomain to its container on a shared Docker network, puts the protected ones behind `auth`, and sits behind `cloudflared`, so nothing on the server is exposed directly.

**Caddy · Cloudflare Tunnel · Docker Compose**

</td>
</tr>
<tr>
<td width="50%" valign="top">

### 🔑 [`login`](login/) · [`accounts`](accounts/) · [`cloud`](cloud/)
The three web apps of the core. `login` signs you in with a passkey or password + 2FA, `accounts` manages your profile, passkeys, TOTP and active sessions (plus users for admins), and `cloud` is the dashboard that links every service.

**Vue 3 · Vite · Nginx**

</td>
<td width="50%" valign="top">

### 🧩 Modules
Separate repos that run next to tpcloud and use its gateway and auth:

- [**tpstore**](https://github.com/mtaibo/tpstore) — storage, photos and share links
- [**tphome**](https://github.com/mtaibo/tphome) — home automation
- [**tpapp**](https://github.com/mtaibo/tpapp) — native iOS and macOS app for all of it

</td>
</tr>
</table>

## Architecture

```
                              Internet
                                 │
                    ┌────────────▼────────────┐
                    │    Cloudflare Tunnel     │   no open ports on the router
                    └────────────┬────────────┘
                                 │
                    ┌────────────▼────────────┐       ┌──────────────────┐
                    │     gateway (Caddy)      │──────▶│       auth       │
                    │  routes by subdomain     │ check │ passkeys · TOTP  │
                    └────────────┬────────────┘session│ sessions · admin │
                                 │                     └──────────────────┘
     ┌──────────┬──────────┬─────┴─────┬───────────┬───────────┬──────────┐
     ▼          ▼          ▼           ▼           ▼           ▼          ▼
   login     accounts    cloud      tpstore      share      tphome     admin
                                   store.*     share.*     tphome.*   (private)
   └──────── tpcloud ────────┘   └───── tpstore ─────┘    └ tphome ┘
```

| Subdomain | Service | Login required |
|---|---|---|
| `login.migueltaibo.com` | Sign-in page | — |
| `cloud.migueltaibo.com` | Dashboard | ✅ |
| `accounts.migueltaibo.com` | Account management | ✅ |
| `store.migueltaibo.com` | [tpstore](https://github.com/mtaibo/tpstore) file browser | ✅ |
| `share.migueltaibo.com` | [tpstore](https://github.com/mtaibo/tpstore) public share links | — |
| `tphome.migueltaibo.com` | [tphome](https://github.com/mtaibo/tphome) control panel | ✅ |
| `admin.migueltaibo.com` | Server admin API (private) | ✅ admin |

## How it works

Opening a protected service without being signed in:

```
Browser opens store.migueltaibo.com
        │
        ▼
Caddy asks auth: is this session cookie valid?  (forward_auth)
        │
        ├── no ──▶ redirect to login.migueltaibo.com
        │                 │
        │                 ▼
        │          passkey, or password + TOTP code
        │                 │
        │                 ▼
        │          auth sets a session cookie on .migueltaibo.com
        │                 │
        │                 ▼
        │          back to store.migueltaibo.com
        │
        └── yes ─▶ request goes through to the service
```

### Sign-in methods

- **Passkeys** — WebAuthn with Face ID, Touch ID or a security key, registered from `accounts`. The default on the web.
- **Password + TOTP** — bcrypt-hashed password, plus a 6-digit code from an authenticator app once 2FA is set up. This is what the native app uses.

### Sessions

Every session is stored server-side with the device (parsed from the user agent), the IP's city and country, and the sign-in method used, so `accounts` can show where I'm signed in and revoke any of them. Web sessions last a fixed hour; sessions from the native app last 24 hours and slide forward while the app is in use, so it doesn't ask me to log in every day.

### Users and admins

The owner account is created from `.env` on first boot. From there, admins can create users, change roles and revoke anyone's sessions from `accounts`, and services like tpstore check the `is_admin` flag to unlock admin-only features.

## Tech stack

| Layer | Technology |
|---|---|
| **Auth** | FastAPI · Python · SQLModel · SQLite |
| **Security** | WebAuthn (`webauthn`) · TOTP (`pyotp`) · bcrypt |
| **Frontends** | Vue 3 · Vite · Nginx |
| **Gateway** | Caddy · `forward_auth` |
| **Exposure** | Cloudflare Tunnel (`cloudflared`) |
| **Hosting** | Docker Compose on my home server |

## How to run

You need Docker and a Cloudflare account with a tunnel pointing your subdomains at the `gateway` container.

```bash
git clone https://github.com/mtaibo/tpcloud
cd tpcloud
cp .env.example .env    # tunnel token, owner email and password, WebAuthn origin
docker compose up -d
docker compose exec auth python scripts/bootstrap.py   # create the owner account
```

Start tpcloud before any module: it creates the `tpcloud_gateway` network that [tpstore](https://github.com/mtaibo/tpstore) joins, and it attaches to [tphome](https://github.com/mtaibo/tphome)'s network to reach its containers.
