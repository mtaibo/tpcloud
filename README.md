<div align="center">

# TPCloud

**My personal cloud: sign-in, files, photos and a native app, in one repo.**

<br/>

![Status](https://img.shields.io/badge/status-active-brightgreen)
![Made with](https://img.shields.io/badge/made%20with-Python%20%7C%20Vue%20%7C%20Swift-blue)
![Runs on](https://img.shields.io/badge/runs%20on-Docker-lightgrey)

</div>

---

TPCloud is everything I self-host, in one repo. It's the front door that every request goes through: one gateway at a single address, where every module is a path (`/store`, `/photos`, `/home`, `/login`…), one auth service that signs you in once for all of them, my storage and photo backup, the web apps for all of it and the native iOS and macOS app. Home automation lives in its own repo, [tphome](https://github.com/mtaibo/tphome), because it's a project of its own with firmware and hardware, and plugs in as one more module. Anyone can run their own with one installer.

## Why I built it

Every service I added to my server needed its own login, its own port and its own way of being reached from outside. I wanted it to feel like a single cloud instead:

- **One login** — sign in once at `/login` and every module knows who you are
- **Reached your way** — only at home, privately from anywhere through Tailscale, or at your own domain through a Cloudflare Tunnel; nothing is opened unless you choose to
- **Plug in a module** — a new service only needs a container on the shared network and a path in the gateway
- **Modern auth** — passkeys on the web, password + TOTP for the native app, and sessions you can see and revoke

## What's inside

<table>
<tr>
<td width="50%" valign="top">

### 🔐 [`services/auth`](services/auth/)
The authentication service behind everything. Passkeys (WebAuthn), password + TOTP two-factor, one host-only session cookie, limits on failed attempts, and the forward-auth endpoint the gateway calls before letting a request through.

**Python · FastAPI · SQLModel · webauthn · pyotp**

</td>
<td width="50%" valign="top">

### 🌐 [`services/gateway`](services/gateway/)
Caddy routes each path on a shared Docker network, puts the protected ones behind `auth`, serves every web app's files itself (they're built into its image) and adds strict security headers to every answer. In front of it sits whatever way in was chosen: `cloudflared`, `tailscale serve`, or Caddy's own certificate.

**Caddy · Cloudflare Tunnel · Tailscale · Docker Compose**

</td>
</tr>
<tr>
<td width="50%" valign="top">

### 🗄️ [`services/store`](services/store/)
My replacement for Google Drive and iCloud Photos, on my own disk: file operations, resumable uploads, thumbnails, share links, ZIP files, the photo backup the app uploads to, albums and backups to a second disk.

**Python · FastAPI · SQLModel · Pillow · ffmpeg**

</td>
<td width="50%" valign="top">

### 🔑 [`web/`](web/)
Every web app, drawn after iOS (grouped lists, large titles, sheets, light and dark) from one shared [`web/ui`](web/ui/): `login`, `accounts` (profile, passkeys, 2FA, devices, activity, users for admins), `cloud` (the dashboard), `store` (a Finder-like file browser), `photos` (the gallery), `share` (public share links) and `home` (the house's control panel, with a live floor plan).

**Vue 3 · Vite · served by the gateway**

</td>
</tr>
<tr>
<td width="50%" valign="top">

### 📱 [`apple/`](apple/)
The native iOS and macOS app, installed as **TPCloud**: files, photos with background backup of the iPhone library, the house's floor plan and the account, against the same backends as the web.

**Swift · SwiftUI · Swift Package Manager · XcodeGen**

</td>
<td width="50%" valign="top">

### 🏠 [tphome](https://github.com/mtaibo/tphome)
Home automation with my own Wi-Fi switches for lights and blinds: firmware, MQTT broker and the API behind `/home/api`. A separate repo that joins the gateway's network when it's installed; its control panel is [`web/home`](web/home/), here.

**C++ · ESP8266 · MQTT · FastAPI · Vue 3**

</td>
</tr>
</table>

## Repository layout

```
tpcloud/
├── apple/              native iOS and macOS app (Swift package + XcodeGen project)
├── services/
│   ├── auth/           sign-in, sessions, users
│   ├── gateway/        Caddyfile (+ extra/ for blocks of one server only)
│   └── store/          files, photos, share links
├── web/
│   ├── ui/             shared design system for every web app
│   ├── login/  accounts/  cloud/
│   ├── store/  photos/  share/
│   └── home/           tphome's control panel
├── data/               what the services write while they run (not in git)
│   ├── auth/           users and sessions
│   └── store/          share links, photo index, thumbnails
├── docker-compose.yml  everything above, one command
└── install.sh          step-by-step installer, and updates
```

## Architecture

```
                    Internet · Tailscale · home network
                                 │
                    ┌────────────▼────────────┐   Cloudflare Tunnel, tailscale serve
                    │   one address, e.g.      │   or port 443 with Caddy's certificate
                    │   cloud.example.com      │
                    └────────────┬────────────┘
                                 │
                    ┌────────────▼────────────┐       ┌──────────────────┐
                    │     gateway (Caddy)      │──────▶│       auth       │
                    │      routes by path      │ check │ passkeys · TOTP  │
                    └────────────┬────────────┘session│ sessions · admin │
                                 │                     └──────────────────┘
     ┌──────────┬──────────┬─────┴─────┬───────────┬───────────┬──────────┐
     ▼          ▼          ▼           ▼           ▼           ▼          ▼
   login     accounts    cloud       store       photos      share     tphome
   /login    /accounts     /         /store      /photos     /share     /home
   └─────────────────────── this repo ──────────────────────────┘   └ its own ┘
```

| Path | Service | Login required |
|---|---|---|
| `/login` | Sign-in page | — |
| `/` | Dashboard | ✅ |
| `/accounts` | Account management | ✅ |
| `/store` | [Store](web/store/) file browser, API at `/store/api` | ✅ |
| `/share` | [Public share links](web/share/) (only the share endpoints of the Store API) | — |
| `/photos` | [Gallery](web/photos/) of the photos backed up from the iPhone (data from `/store/api`) | ✅ |
| `/home` | [Home control panel](web/home/), API at `/home/api` from [tphome](https://github.com/mtaibo/tphome) | ✅ |
| `/auth` | The auth API every client signs in against | — |

The gateway strips each module's prefix, so the services behind it still see the `/api/...` paths they always had.

## How it works

Opening a protected service without being signed in:

```
Browser opens /store/photos
        │
        ▼
Caddy asks auth: is this session cookie valid?  (forward_auth)
        │
        ├── no ──▶ redirect to /login  (/store/photos kept in a short-lived cookie)
        │                 │
        │                 ▼
        │          passkey, password + TOTP code, or a QR code approved elsewhere
        │                 │
        │                 ▼
        │          auth sets a host-only __Host- session cookie
        │                 │
        │                 ▼
        │          back to /store/photos (only ever a path on this host)
        │
        └── yes ─▶ request goes through to the service
```

### Sign-in methods

- **Passkeys** — WebAuthn with Face ID, Touch ID or a security key, registered from `accounts`. The default on the web, and no email needed: the browser offers the passkey it has for the site, also as autofill on the email field.
- **Password + TOTP** — bcrypt-hashed password, plus a 6-digit code from an authenticator app once 2FA is set up. This is what the native app uses.
- **QR code** — the login page shows a code that a signed-in phone scans; the phone sees which device asks and from where, and approves or rejects it. Only the browser that showed the code can collect the session, and each code lasts two minutes.

### Sessions

Every session is stored server-side with the device (parsed from the user agent), the IP it signed in from and the one it was last used from (and, if `TPCLOUD_GEOIP=1`, their city and country), when it was last used, and how it signed in (for a QR code, which device approved it), so `accounts` can show where I'm signed in and revoke any of them, or all but this one. Sessions are revoked by their id; the session token itself never goes back to the browser.

Sessions slide forward while they're used: with "keep me signed in" (asked right after signing in, on by default) a browser stays signed in 30 days after it was last used; without it, until the browser closes or 12 hours without use. The native app gets 90 days. Every sign-in, failed attempt and revoked session is also kept for six months as the account's activity, with device, place and IP.

### Security

- **One host, one cookie** — the session cookie is `__Host-` prefixed: Secure, HttpOnly, SameSite=Lax, host-only, so no other site or subdomain can read, set or overwrite it.
- **Limited attempts** — 5 wrong passwords per account (or 20 per address) in 15 minutes lock sign-in for a while; a half-signed-in session is thrown away after 5 wrong TOTP codes, and each code works once.
- **Nothing to enumerate** — unknown emails get the same answers, in the same time, as real ones.
- **Strict headers** — HSTS, a Content-Security-Policy that only allows this host's own scripts, `nosniff`, no framing by other sites; files users upload are served sandboxed, so an uploaded HTML page can't run scripts.
- **Little exposed** — only the gateway listens beyond the Docker network, and only on what the chosen way in needs; the APIs have no docs or schema online.

### Users and admins

The owner account is created by the installer. From there, admins can create users, change roles and revoke anyone's sessions from `accounts`, and services like Store check the `is_admin` flag to unlock admin-only features.

## Tech stack

| Layer | Technology |
|---|---|
| **Auth** | FastAPI · Python · SQLModel · SQLite |
| **Security** | WebAuthn (`webauthn`) · TOTP (`pyotp`) · bcrypt |
| **Storage** | FastAPI · SQLModel · Pillow · ffmpeg |
| **Frontends** | Vue 3 · Vite, built into the gateway's image and served by Caddy |
| **App** | Swift · SwiftUI · Swift Package Manager · XcodeGen |
| **Gateway** | Caddy · `forward_auth` |
| **Exposure** | Cloudflare Tunnel (`cloudflared`) · Tailscale · Caddy's own TLS |
| **Hosting** | Docker Compose, set up by `install.sh` |

## How to run

On a Linux machine (a Raspberry Pi, an old PC, a VPS), run the installer and answer its questions:

```bash
curl -fsSL https://raw.githubusercontent.com/mtaibo/tpcloud/main/install.sh | bash
```

It asks whether you want Store and [tphome](https://github.com/mtaibo/tphome), where Store keeps your files and how you'll reach it (only at home, Tailscale, or your own domain through a Cloudflare Tunnel or an open port), creates your admin account, installs Docker if it's missing and starts everything. Run it again to change any of that; `./install.sh --update` gets the latest version of everything and restarts it without asking anything, and `--dry-run` shows what it would do without changing anything.

By hand:

```bash
git clone https://github.com/mtaibo/tpcloud
cd tpcloud
cp .env.example .env    # address, how it's reached, modules, where Store keeps files
docker compose up -d --build
OWNER_EMAIL=you@example.com OWNER_PASSWORD=... docker compose run --rm -e OWNER_EMAIL -e OWNER_PASSWORD auth python scripts/bootstrap.py
```

Store runs behind the `store` profile and Home's control panel behind `home` (`COMPOSE_PROFILES=store,home`, plus `tunnel` for a Cloudflare Tunnel). tphome, if you want it, is cloned next to this folder and started after it, since tpcloud creates the `tpcloud_gateway` network its API joins.

Beyond localhost the gateway only publishes what `.env` asks for: port 443 for your own domain or the home network, and, where tphome is installed, port 80 on the local network for nothing but `/firmware`, so its switches can download OTA updates.

The app is built from [`apple/`](apple/) with Xcode; see its [README](apple/README.md).
