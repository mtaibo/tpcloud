<div align="center">

# TPCloud

**My own cloud in one file: sign-in, files, photos and native apps, at home and from anywhere.**

<br/>

![Status](https://img.shields.io/badge/status-active-brightgreen)
![Made with](https://img.shields.io/badge/made%20with-Python%20%7C%20Vue%20%7C%20Swift%20%7C%20Rust-blue)
![Runs on](https://img.shields.io/badge/runs%20on-Linux-lightgrey)

</div>

---

TPCloud is everything I self-host, in one repo. It's the front door that every request goes through: one gateway at a single address, where every module is a path (`/store`, `/photos`, `/home`, `/login`…), one auth service that signs you in once for all of them, my storage and photo backup, the web apps for all of it, the native apps for iPhone, Mac and Linux, and the `tpcloud` command that runs the server and, on any other computer, signs you in to it. Home automation lives in its own repo, [tphome](https://github.com/mtaibo/tphome), because it's a project of its own with firmware and hardware, and plugs in as one more module. Anyone can run their own from one file, downloaded from [tpcloud.migueltaibo.com](https://tpcloud.migueltaibo.com), and nothing in it depends on me: no account, no relay, no server of mine in between.

## Why I built it

Every service I added to my server needed its own login, its own port and its own way of being reached from outside. I wanted it to feel like a single cloud instead:

- **One login** — sign in once at `/login` and every module knows who you are
- **Reached your way** — at home, found on the network by itself with its own certificate; privately from anywhere through Tailscale; at your own domain through a Cloudflare Tunnel the setup makes for you. Any mix of them, and nothing is opened unless you choose to
- **Fast at home** — the apps find the server on the Wi-Fi by themselves and go straight to it (about 16 ms a request from my Mac, against about 180 ms around the internet through the tunnel), and switch to your own address when you're out
- **Apps everywhere** — iPhone, Mac and Linux apps and a `tpcloud` command, each keeping a list of networks like Mail keeps accounts
- **Plug in a module** — a new service is one more process the server runs and one more path in the gateway
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
Caddy routes each path to its service, puts the protected ones behind `auth`, serves every web app's files itself and adds strict security headers to every answer. It answers on every address the server has at once: the tunnel's, Tailscale's (with Tailscale's certificate) and the home network's (with its own certificate authority, announced over Bonjour), and `/.well-known/tpcloud` tells the apps all of them.

**Caddy · Cloudflare Tunnel · Tailscale**

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
Every web app, drawn after iOS (grouped lists, large titles, sheets, light and dark) from one shared [`web/ui`](web/ui/): `login`, `accounts` (profile, passkeys, 2FA, devices, activity, users for admins), `cloud` (the dashboard), `store` (a Finder-like file browser, with its own PDF viewer: selectable text, search, thumbnails, the outline and pinch zoom, the same in every browser), `photos` (the gallery), `share` (public share links) and `home` (the house's control panel, with a live floor plan).

**Vue 3 · Vite · pdf.js · served by the gateway**

</td>
</tr>
<tr>
<td width="50%" valign="top">

### 📱 [`apple/`](apple/)
The native iOS and macOS app, installed as **TPCloud**: files, photos with background backup of the iPhone library, the house's floor plan and the account, against the same backends as the web. It keeps a list of networks (found on the Wi-Fi or added by address), sets up new hosts, updates the server for admins, and trusts a host's own certificate only once it checks out. On TestFlight for now.

**Swift · SwiftUI · Swift Package Manager · XcodeGen**

</td>
<td width="50%" valign="top">

### 🐧 [`desktop/`](desktop/)
The native Linux app, `tpcloud-app`: the same networks, files, photos, the house and the server's admin, in a window of its own on any desktop, Wayland or X11. It shares its sign-ins with the `tpcloud` command, and can turn the computer it runs on into the server.

**Rust · iced · rustls · mdns-sd**

</td>
</tr>
<tr>
<td width="50%" valign="top">

### 🏠 [tphome](https://github.com/mtaibo/tphome)
Home automation with my own Wi-Fi switches for lights and blinds: firmware, MQTT broker and the API behind `/home/api`. A separate repo whose API ships inside the `tpcloud` file and runs when Home is turned on; its control panel is [`web/home`](web/home/), here.

**C++ · ESP8266 · MQTT · FastAPI · Vue 3**

</td>
</tr>
<tr>
<td width="50%" valign="top">

### 🧰 [`cli/`](cli/)
The `tpcloud` command. On the server, one static Linux file that carries everything above, its own Python included, and installs, sets up, runs, updates, backs up and uninstalls the whole server with nothing else on the machine; it also makes and watches mirrored, optionally encrypted disk pools. On any Mac or Linux computer, a 2–4 MB build of the same program is the client: sign in (approving it in the browser), accounts, sessions, updating the server, and `tphome` for the house. On its own it opens a terminal app.

**Rust · ratatui · rustls · systemd · Btrfs · LUKS**

</td>
</tr>
<tr>
<td colspan="2" valign="top">

### ⬇️ [`site/`](site/)
[tpcloud.migueltaibo.com](https://tpcloud.migueltaibo.com), where TPCloud is downloaded: a file of points that opens into the machine, your devices and the race between home and the internet, then the right download for the visitor's device and the two one-line installers it serves (`/install` for the server, `/get` for every other computer). Every tagged release builds the server, the client and the Linux app on GitHub Actions, and the site's links always point at the latest.

**HTML · CSS · canvas · Cloudflare Workers · GitHub Actions**

</td>
</tr>
</table>

## Repository layout

```
tpcloud/
├── apple/              native iOS and macOS app (Swift package + XcodeGen project)
├── desktop/            native Linux app (Rust, iced): networks, Files, Photos, Home, Server
├── services/
│   ├── auth/           sign-in, sessions, users
│   ├── gateway/        Caddyfile (+ extra/ for blocks of one server only)
│   └── store/          files, photos, share links
├── web/
│   ├── ui/             shared design system for every web app
│   ├── login/  accounts/  cloud/
│   ├── store/  photos/  share/
│   └── home/           tphome's control panel
├── cli/                the `tpcloud` command: the server in one file, and the client for any computer
├── site/               tpcloud.migueltaibo.com, where it's downloaded (+ the /install and /get scripts)
├── data/               what the services write when run with Docker (not in git)
├── docker-compose.yml  the same services as containers, for running with Docker instead
└── install.sh          the installer for that Docker setup
```

## Architecture

```
                    Internet · Tailscale · home network
                                 │
                    ┌────────────▼────────────┐   your domain (Cloudflare Tunnel or port 443),
                    │  cloud.example.com       │   Tailscale's name, and <host>.local at home,
                    │  box.tailnet.ts.net …    │   all at once
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

The gateway strips each module's prefix, so the services behind it still see the `/api/...` paths they always had. On a server everything above runs as plain processes under one systemd service: the services listen only on `127.0.0.1`, and the gateway is the only way in (with Home, the switches also reach the MQTT broker on the local network).

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
- **From the terminal** — `tpcloud login` asks how, one key each: in the browser (the same approval page, passkey included), with a phone (a QR code drawn in the terminal) or with email, password and code right there.

### Sessions

Every session is stored server-side with the device (parsed from the user agent), the IP it signed in from and the one it was last used from (and, if `TPCLOUD_GEOIP=1`, their city and country), when it was last used, and how it signed in (for a QR code, which device approved it), so `accounts` can show where I'm signed in and revoke any of them, or all but this one. Sessions are revoked by their id; the session token itself never goes back to the browser.

Sessions slide forward while they're used: with "keep me signed in" (asked right after signing in, on by default) a browser stays signed in 30 days after it was last used; without it, until the browser closes or 12 hours without use. The native apps and the `tpcloud` command get 90 days, and send their session as a bearer token, so one sign-in works on every address of the network. Every sign-in, failed attempt and revoked session is also kept for six months as the account's activity, with device, place and IP.

### Security

- **One host, one cookie** — the session cookie is `__Host-` prefixed: Secure, HttpOnly, SameSite=Lax, host-only, so no other site or subdomain can read, set or overwrite it.
- **Limited attempts** — 5 wrong passwords per account (or 20 per address) in 15 minutes lock sign-in for a while; a half-signed-in session is thrown away after 5 wrong TOTP codes, and each code works once.
- **Nothing to enumerate** — unknown emails get the same answers, in the same time, as real ones.
- **Strict headers** — HSTS, a Content-Security-Policy that only allows this host's own scripts, `nosniff`, no framing by other sites; files users upload are served sandboxed, so an uploaded HTML page can't run scripts.
- **Little exposed** — every service listens only on `127.0.0.1`; the gateway listens only on what the chosen way in needs (and, with Home, the MQTT broker and `/firmware` on the local network for the switches); the APIs have no docs or schema online.

### Users and admins

The owner account is created by the guided setup. From there, admins can create users, change roles and revoke anyone's sessions from `accounts`, the apps or `tpcloud`, update the server to the latest release from any of them (the update runs apart from the service, so the restart at its end can't cut it), and services like Store check the `is_admin` flag to unlock admin-only features.

## Tech stack

| Layer | Technology |
|---|---|
| **Auth** | FastAPI · Python · SQLModel · SQLite |
| **Security** | WebAuthn (`webauthn`) · TOTP (`pyotp`) · bcrypt |
| **Storage** | FastAPI · SQLModel · Pillow · ffmpeg |
| **Frontends** | Vue 3 · Vite, built into the `tpcloud` file and served by Caddy |
| **Apps** | Swift · SwiftUI (iPhone, Mac) · Rust · iced (Linux) |
| **Gateway** | Caddy · `forward_auth` |
| **Exposure** | Cloudflare Tunnel (`cloudflared`) · Tailscale · Caddy's own TLS |
| **Server** | One static Rust binary (`cli/`) run by systemd, supervising every process · Btrfs pools with LUKS · Docker Compose as an alternative |
| **Client** | The same Rust program without the server (macOS universal, Linux static) · rustls with the host's own CA pinned |
| **Releases** | GitHub Actions (server, client and Linux app for arm64 and x86-64, client for macOS) · GitHub Releases |
| **Site** | Cloudflare Workers static assets · canvas |

## How to run

### The server: one file

On a Linux machine (a Raspberry Pi, an old PC, a VPS; arm64 or x86-64):

```bash
curl -fsSL https://tpcloud.migueltaibo.com/install | sh
```

That downloads the right file for the machine and runs `sudo tpcloud install`. The file can also be downloaded by hand from [tpcloud.migueltaibo.com](https://tpcloud.migueltaibo.com) or the [releases](https://github.com/mtaibo/tpcloud/releases), then `sudo ./tpcloud-linux-arm64 install`.

It's a single static binary (68 MB for arm64, 76 MB for x86-64) and needs nothing else: no Docker, no Python, nothing from the package manager. Inside it carries its own Python with every service, the built web apps, [tphome](https://github.com/mtaibo/tphome)'s API, and static builds of Caddy, Mosquitto, ffmpeg, rsync, cloudflared, btrfs-progs, cryptsetup and smartctl. Installing unpacks them to `/opt/tpcloud`, copies itself to `/usr/local/bin` and adds a systemd service, `tpcloud serve`, that runs each process and restarts it if it stops. The setup is done right there in the terminal, or from another device: the server then waits on the home network, announced over Bonjour, and the TPCloud app («Set up a new host») or a browser at `http://tpcloud.local` finishes it with the pairing code it prints. Either way it asks the same questions, nothing is applied until the end, and it creates your admin account:

- **Files** — a folder on any disk that's already mounted, or empty disks it formats as one Btrfs pool: a single disk, a mirror (RAID1) or mirrored pairs (RAID10), optionally encrypted with LUKS (opened by itself at boot, with a recovery key shown once). `tpcloud storage` shows each disk's health, a scrub runs every month, and `tpcloud storage add <disk>` grows the pool later.
- **Modules** — Store (files, photo backup, share links) and Home (the house, with [tphome](https://github.com/mtaibo/tphome)'s switches).
- **How it's reached** — at home as `https://tpcloud.local` (its own certificate authority, which the apps trust when they add the network), privately through Tailscale, at your own domain through a Cloudflare Tunnel it makes by signing you in to Cloudflare, or on port 443; any mix of them at once.

On NixOS the service goes in `configuration.nix` instead of `/etc/systemd/system`: the installer stops before installing anything and prints the lines to add.

Settings, databases and caches live in `/var/lib/tpcloud` (`tpcloud.env`, `data/`, `home/`, `gateway-extra/`, `logs/`) and an update never touches them. `sudo tpcloud update` gets the latest release from GitHub (checked against its SHA-256) and installs it; `install` run from an older file does the same, unless it's given `--this-file`. The service looks for a new release once a day, and `tpcloud status` says when there is one. Admins can also update it from the apps, the web (Accounts → Administration → Server) or `tpcloud host update` on their own computer. `install --adopt <folder>` takes over a server that ran with Docker from tpcloud and tphome clones in that folder, copying their settings and data, with about a second down.

**Deleting it.** `sudo tpcloud delete` asks what should go: only the program (every account, file, setting and backup stays, and installing again carries on), everything after a last backup to a folder you choose (`tpcloud restore` brings it back), or everything with nothing kept, the Cloudflare tunnel it made included. Everyone's files are asked about on their own, and nothing is touched until you type `delete`.

**Backups.** Every night the service copies TPCloud's own data (settings, every database: accounts, the photo index, albums, share links, the house; Home's firmware) to the folder set by the guided setup or `sudo tpcloud backup --to <folder>`, best on another disk. They're a few MB each, made with SQLite's online backup while everything keeps running, and the last 14 are kept. Your files aren't in them (they're on their own disk, with Store's own mirror to a backup disk), nor are thumbnails and previews, which are made again. If the folder is inside that mirror's destination, the mirror leaves it alone. `sudo tpcloud restore <backup>` puts one back, after backing up what's there.

| Command | What it does |
|---|---|
| `tpcloud` | A terminal app: status, Home, accounts, modules, logs and setup |
| `tpcloud install` | Install (or reinstall) from this file; asks how to set it up |
| `tpcloud status` | What's running, the address, the modules and where files and data are |
| `tpcloud setup` | The guided configuration again |
| `tpcloud update [--force]` | Get the latest release from GitHub and install it |
| `tpcloud backup [--to <folder>]` · `--list` · `--off` | Back up now (and set where), list the backups, or stop the nightly ones |
| `tpcloud restore <backup>` | Put a backup's settings and databases back |
| `tpcloud start` · `stop` · `restart` | Start everything, stop it (data is kept) or restart it |
| `tpcloud logs [process] [-f]` | A process's log, or every one's |
| `tpcloud module list` · `enable home` · `disable home` | Turn Store or Home on and off |
| `tpcloud users` · `users add <email> [--admin]` | List the accounts or add one |
| `tpcloud storage` · `storage add <disk>` · `scrub` | The disks and the pool, grow it, check every block now |
| `tpcloud delete` | Remove TPCloud, choosing what to keep |
| `tphome` | Every blind, its position and whether it's connected (there while Home is on) |
| `tphome --blind B0301 up` | Raise a blind; also `down`, `stop`, or a position from 0 (closed) to 100 (open) |

`tphome` talks to home-api on its local port from the server itself, so it needs no session: whoever can run commands there already owns the server. Commands that change something need `sudo`.

### Your computers: the apps and the `tpcloud` command

On a Mac or a Linux computer:

```bash
curl -fsSL https://tpcloud.migueltaibo.com/get | sh
```

On Linux it first asks whether this computer signs in to a cloud or *is* one (the second is the same as `/install`); signed in, it installs the command and the TPCloud app, with its entry in the applications menu, and goes straight on to `tpcloud login` (or «later»). On a Mac it installs the `tpcloud` command (the app for Mac and iPhone is on TestFlight for now). It always writes to the same places (`/usr/local/bin/tpcloud`, `tphome` beside it, `tpcloud-app`), so running it again updates instead of duplicating; on a TPCloud server it leaves the server's own `tpcloud`, which is a client too.

| Command | What it does |
|---|---|
| `tpcloud login [address]` | Find the server on this network (or at an address) and sign in, approving it in the browser or with email, password and code |
| `tpcloud` | The network in a terminal app: overview, your files (browse, download, upload, share), synced folders, share links, the house, accounts, sessions and the server |
| `tpcloud sync add [folder] [--full]` | Your files in a folder here (`~/TPCloud` by default): on demand, or a full two-way copy with `--full` |
| `tpcloud sync keep <path>` · `free <path>` | Keep a folder on demand here for good, or give back the room what was fetched takes |
| `tpcloud sync mode <folder> demand\|full` · `sync` | Switch a folder between on demand and a full copy; how each one is doing |
| `tpcloud networks` · `networks use <name>` | The networks this computer knows, and which one is in use |
| `tpcloud users [add\|remove\|admin]` | Accounts (admins) |
| `tpcloud sessions [--all] [revoke <id>]` | Your sessions, or everyone's (admins), and close one |
| `tpcloud host [check\|update]` | The server's version, and update it (admins) |
| `tphome` | The house, over the network |
| `tpcloud update` · `delete` | Update this command, or remove every trace of it, sign-ins included |

**Folders on demand** (Linux, through FUSE): the folder shows every file in the cloud with its name, size and date, but they take no room until one is opened; then it's fetched and kept for a day (or for good, inside what `tpcloud sync keep` marks), and whatever is written there goes up when it's closed. On a Mac, and anywhere without FUSE, a synced folder is a full two-way copy, like `--full`: changes go both ways every few seconds, and a file changed on both sides keeps both versions. A small user service (systemd, or launchd on a Mac) keeps them going after a restart.

The first time it reaches a server at home it checks the server's own certificate authority against the fingerprint the server publishes, and from then on trusts that authority only for that network's local names, like the apps do. Sign-ins are kept in `~/.config/tpcloud` (on a Mac, `~/Library/Application Support/tpcloud`), shared with the Linux app.

To build it: `cli/build.sh` writes the server, `cli/dist/tpcloud-linux-<arch>`, and `cli/build-client.sh` the Linux client, `cli/dist/tpcloud-cli-linux-<arch>`. It needs Docker on the building machine only (the runtime and the Rust binary are both built in containers) and tphome cloned next to this folder. Pushing a `v*` tag builds everything on GitHub Actions (the server, the client and the Linux app for both architectures, and a universal client for macOS) and publishes it as a release, which is what the site's download links point to.

### With Docker instead

The same services also run as Docker containers, if you'd rather manage them that way. The installer script asks the same questions:

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

The app is built from [`apple/`](apple/) with Xcode; see its [README](apple/README.md). The Linux desktop app is `desktop/build.sh` (or `tpcloud-app-linux-<arch>` from a release): one file, nothing to install; on NixOS it needs nix-ld.
