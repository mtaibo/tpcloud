# AGENTS.md — TPHome Frontend

## Project overview

TPHome is a full-stack home automation system. This repository is the **frontend layer** — a Vue 3 SPA that renders an interactive SVG floor plan for controlling lights and blinds in real time over WebSocket.

Three repos make up the whole system:
- `tphome` — Vue 3 frontend (this one)
- `tphome-api` — FastAPI backend + MQTT broker on Raspberry Pi
- `tphome-firmware` — C++ firmware for ESP8266 / BK7231N chips

## Tech stack

- **Vue 3** (Composition API, `<script setup>`)
- **Vite 8** (build tool)
- **Tailwind CSS 4** (`@tailwindcss/vite` plugin, `@import "tailwindcss"`)
- **Pinia** (state management)
- **Vue Router 4** (history mode, two routes: `/` and `/settings`)
- **Axios** (HTTP client to `/api/*`)
- **Lucide Vue Next** (icons)
- **Nginx** (production static serving)
- **Caddy** (reverse proxy, routes `/api/*` and `/ws/*` to backend)

## Project structure

```
tphome/
├── src/
│   ├── main.js              # App entry: creates Vue app, mounts Pinia + Router
│   ├── App.vue              # Root component, initializes device + map stores
│   ├── main.css             # Tailwind import, custom theme colors, component classes
│   ├── router.js            # Vue Router config (/, /settings)
│   ├── config/
│   │   ├── api.js           # Axios client (getDevices, getConfig, sendCommand, sendPrefs)
│   │   ├── socket.js        # WebSocket client (auto-reconnect, state updates)
│   │   ├── devices.js       # Pinia store: devices storage, blinds/lights computed, setup/update
│   │   └── map.js           # Pinia store: map config (viewBox, rooms, labels, doors)
│   ├── views/
│   │   ├── Dashboard.vue    # Main page: Sidebar + Topbar + Blueprint
│   │   └── Settings.vue     # Placeholder settings page
│   ├── layout/
│   │   ├── Sidebar.vue      # Collapsible sidebar (nav items, user card, collapse toggle)
│   │   ├── Topbar.vue       # API status indicator, active devices count, refresh button
│   │   └── BlindsControl.vue # Blind control panel (position slider, up/stop/down, presets, precise input)
│   └── components/
│       ├── Blueprint.vue        # SVG blueprint container + BlindsControl sidebar
│       └── blueprint-layers/
│           ├── HouseLayer.vue   # SVG: rooms (rects), labels (text), doors (lines)
│           ├── BlindsLayer.vue  # SVG: blind rectangles with cover animation + click to select
│           └── LightsLayer.vue  # SVG: light circles with on/off state + click to toggle
├── src/db/
│   ├── devices.json          # Device config (lights, blinds with IDs, names, map positions, prefs)
│   └── map.json              # Floor plan config (viewBox, rooms, labels, doors)
├── docs/
│   ├── banner.svg            # README banner
│   ├── ai-reference.html     # First prototype (vanilla JS, not used by the Vue app)
│   └── images/               # Title SVGs
├── docker-compose.yml        # Caddy + frontend (external network for api)
├── Dockerfile                # Multi-stage: node build → nginx serve
├── nginx.conf                # SPA fallback to index.html
├── Caddyfile                 # Reverse proxy: /api/* and /ws/* → tphome-api:8000
├── vite.config.js            # Vite + Vue + Tailwind plugins
└── package.json              # Scripts: dev, build, preview
```

## Key data flow

1. App mounts → `App.vue` calls `devices.setup()` and `map.setup()` on mount
2. `devices.setup()` fetches device config from `GET /api/config/devices`, then calls `update()` to sync state from `GET /api/devices`
3. `map.setup()` fetches map config from `GET /api/config/map`
4. WebSocket connects to `ws://<host>/api/ws` — receives `device_state`, `device_online`, `device_offline`, `device_info` events
5. User clicks a blind → `BlindsLayer` emits `select` → `Blueprint` sets `selectedId` → `BlindsControl` panel opens
6. `BlindsControl` calls `POST /commands/<id>/<action>` (up/down/stop/set)
7. `LightsLayer` calls `POST /commands/<id>/toggle` on click

## Conventions

- **Vue**: Composition API with `<script setup>`
- **Imports**: Single quotes, relative paths within src/
- **Indentation**: 4 spaces
- **CSS**: Tailwind utility classes + custom `@theme` tokens in main.css
- **Device IDs**: `L####` for lights, `B####` for blinds, `S####` for switches
- **API paths**: `/api/devices`, `/api/config/{subject}`, `/commands/{id}/{action}`, `/admin/{id}/prefs`
- **State management**: Pinia stores in `src/config/` (not `src/stores/`)
- **Database**: Static JSON files in `src/db/`, served by the API

## API routes (from the frontend's perspective)

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/devices` | List all devices with current state |
| GET | `/api/config/{subject}` | Get config JSON (devices or map) |
| PUT | `/api/config/{subject}` | Update config JSON |
| POST | `/commands/{id}/{action}` | Send command to device |
| POST | `/commands/{id}/set/{value}` | Set blind position |
| POST | `/admin/{id}/prefs` | Update device preferences |
| WS | `/api/ws` | Real-time device events |

## Device store (`devices.js`)

Reactive `storage` object keyed by category (`lights`, `blinds`, `switches`). Each device has:
- `name`, `map` (SVG coordinates), `prefs` (timings, relay config)
- `state` (position, motor_state for blinds; on for lights)
- `connection` (online status)

Computed properties: `blinds` (filters out null-state), `lights` (filters out null-state), `active` (online count).

## Build & deploy

```bash
npm run dev      # Development server with HMR
npm run build    # Production build to dist/
docker compose up -d  # Full deployment with Caddy
```

The Docker network `tphome-network` is expected to be created externally (shared with `tphome-api`).
