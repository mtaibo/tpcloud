<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/banner/dark.svg">
  <img src="../images/banner/light.svg" alt="TPHome" width="100%" />
</picture>

<br/>
<br/>

**Home automation system — from the chip to the browser.**

<br/>

![Status](https://img.shields.io/badge/status-active-brightgreen)
![License](https://img.shields.io/badge/license-MIT-blue)
![Made with](https://img.shields.io/badge/made%20with-C%2B%2B%20%7C%20Python%20%7C%20Vue-lightgrey)

</div>

---

I'm a first-year computer engineering student and this is the frontend layer of a home automation system I built from scratch. The full system lives in this monorepo: firmware (C++ for ESP8266/BK7231N switches), API (FastAPI on Raspberry Pi), and this frontend (Vue 3, also on the Pi).

This module contains the frontend layer that lets you easily interact with all devices, customize your house through JSON files, and host the web panel wherever you want.

## How it started

I got tired of needing different apps to control my house. Every brand has its own cloud and its own account, they hardly talk to each other, and if you want a specific device behaviour — like closing at 20% first and then going fully closed — there was no way to do it. So I started building my own system — one that lives entirely on my local network.

The whole project is made of three parts: the C++ firmware inside the switches, a Python backend hosted on a Raspberry Pi, and the Vue frontend in this repo (also hosted on the same Pi). I wrote every layer myself to get what I needed: a fast, personalized way to control my devices.

Everything here is:
- **Local** — no cloud needed, everything runs on my home network
- **Understandable** — no black boxes, I designed and built each piece, all fully documented
- **Customizable** — for example, the blind stops at 20% on first press instead of 0%
- **Unified** — one screen shows every device in the house

## Architecture

Three layers, one monorepo:

```
┌─────────────────────────────────────────────────────┐
│                  frontend/                          │
│           Vue 3 frontend · Tailwind CSS             │
│         Hosted via Docker on Raspberry Pi           │
└────────────────────────┬────────────────────────────┘
                         │ HTTP / WebSocket
┌────────────────────────▼────────────────────────────┐
│                     api/                            │
│    FastAPI backend · MQTT orchestration · SQLite    │
│          Hosted via Docker on Raspberry Pi          │
└────────────────────────┬────────────────────────────┘
                         │ MQTT (binary protocol)
┌────────────────────────▼────────────────────────────┐
│                  firmware/                          │
│   C++ firmware for ESP8266 / BK7231N smart switches │
│           Flashing over factory firmware            │
└─────────────────────────────────────────────────────┘
```

Each layer lives in its own subdirectory of this monorepo. The firmware runs on the chips inside the switches. The backend is hosted on a Raspberry Pi and communicates with the firmware via MQTT using a custom protocol. The frontend also runs on the Pi and talks to the backend through HTTP requests and WebSockets.

## Monorepo structure

<table>
<tr>
<td width="33%">

### [firmware/](../firmware/)

Custom C++ firmware for proprietary chips, built with PlatformIO.

Replaces factory software on commercial blind controllers and light switches with a fully local MQTT-based control layer.

**C++ · PlatformIO · Arduino**

</td>
<td width="33%">

### [api/](../api/)

FastAPI backend running on a Raspberry Pi alongside a Mosquitto MQTT broker, managed with Docker.

Handles device management, state persistence, OTA firmware serving and real-time WebSocket events.

**Python · FastAPI · Docker · SQLite**

</td>
<td width="33%">

### frontend/ (this module)

Vue 3 frontend served by Nginx behind a Caddy reverse proxy, also hosted on the Raspberry Pi.

You can manage devices in multiple ways — through simple cards, or by rendering an interactive SVG floor plan of my house where I can see and control every light and blind in real time. You can also configure chip firmwares from here.

**Vue 3 · Tailwind CSS · Nginx**

</td>
</tr>
</table>

## What's managed here

This repository is the **frontend layer** of TPHome — the single interface to control every device in the house:

| What | How |
|---|---|
| **Blueprint view** | Interactive SVG floor plan of the house with rooms, labels, and doors |
| **Lights** | Click any light fixture on the plan to toggle it on/off |
| **Blinds** | Click a blind to open a control panel with position slider, quick buttons, and presets |
| **Live updates** | WebSocket connection receives device state changes and updates the UI in real time |
| **Device management** | Pinia store syncs device configuration and state from the API |
| **API communication** | Axios client for REST calls, WebSocket for real-time events |
| **Docker deployment** | Nginx serves the static build, Caddy handles routing |

## How it works

A typical interaction — pressing "down" on a blind from the frontend:

```
Frontend sends POST /commands/B0101/down
        │
        ▼
API publishes command via MQTT
        │
        ▼
ESP8266 chip receives command, activates relay, starts motor
        │
        ▼
Chip publishes position + motor state every second
        │
        ▼
API receives state, updates database, pushes WebSocket event
        │
        ▼
Frontend updates position in real time
```

No cloud involved at any point. The entire system works on the local network.

## Tech stack

| Layer | Technology |
|---|---|
| **Framework** | Vue 3 with Composition API |
| **Build** | Vite 8 |
| **Styling** | Tailwind CSS 4 |
| **State** | Pinia |
| **Routing** | Vue Router 4 |
| **HTTP** | Axios |
| **Icons** | Lucide |
| **Reverse proxy** | Caddy |
| **Container** | Docker + docker-compose |

## How to run

**Production** — run from the monorepo root with a single command:

```bash
# From tphome/
docker compose up --build -d
```

**Development** — run the frontend standalone:

```bash
cd frontend
npm install
npm run dev
```

> **Note:** `package-lock.json` is committed to ensure reproducible installs. Requires **npm >= 10**. If you use a different npm version the lockfile may be reformatted — just commit the result.

## License

MIT — see [LICENSE](LICENSE)
