<div align="center">

<img src="App/Sources/Assets.xcassets/AppIcon.appiconset/icon_256x256.png" alt="TPCloud app icon" width="128" />

# TPApp

**One native app for my whole personal cloud.**

<br/>

![Status](https://img.shields.io/badge/status-active-brightgreen)
![Platforms](https://img.shields.io/badge/platforms-iOS%2027%20%7C%20macOS%2027-lightgrey)
![Made with](https://img.shields.io/badge/made%20with-Swift%20%7C%20SwiftUI-orange)

</div>

---

tpapp is the native iOS and macOS client for everything I self-host with [tpcloud](../). It replaces the Vue web apps under [`web/`](../web/) and [tphome](https://github.com/mtaibo/tphome)'s with a single SwiftUI app (installed as **TPCloud**), talking to the same FastAPI backends without changing a line on the server.

## Why a native app

The web frontends work, but a browser tab can't do what the phone is actually good at. I wanted my files, my photos and my house in one place, with the things only a native app can do:

- **Photo backup** — the iPhone library uploads to my own server in the background, like iCloud Photos but on my disk
- **Native feel** — Liquid Glass, system tab bar, Photos-app gestures, VoiceOver and Keychain AutoFill
- **Instant** — photo list and thumbnails live on the device, so the gallery opens without waiting for the network
- **One login** — password + TOTP once, and the session stays on the device

## The app

Four tabs, each backed by one of my services:

<table>
<tr>
<td width="50%" valign="top">

### 🗂️ Store
Files-style browser for my storage on [Store](../services/store/): my Home and Shared folders, uploads from Files or the photo library with a progress bar that survives leaving the app, photo and video thumbnails, rename, delete, ZIP download, favorites, custom folder icons and public share links. Folders load ahead of time so they open instantly, and anything can be dragged into another folder or out to another app.

**`TPStoreFeature`**

</td>
<td width="50%" valign="top">

### 🖼️ Photos
A gallery built to feel like the Photos app: pinch-to-zoom between 1, 3, 5, 15 and 31 columns, a `UICollectionView` grid drawn from on-device thumbnails, albums, and full-resolution originals fetched on open. Backs up the iPhone library to tpstore, even while the app is suspended.

**`TPPhotosFeature`**

</td>
</tr>
<tr>
<td width="50%" valign="top">

### 🏠 Home
Interactive floor plan of my house from [tphome](https://github.com/mtaibo/tphome), with every light and blind drawn on top. Tap to toggle, slide to position a blind, and see state change live over WebSocket.

**`TPHomeFeature`**

</td>
<td width="50%" valign="top">

### ⚙️ More
Account management against tpcloud's [auth service](../services/auth/) (profile, password, TOTP setup with QR code, active sessions).

**`TPCloudFeature`**

</td>
</tr>
</table>

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    App/  (XcodeGen)                      │
│        TPApp-iOS · TPApp-macOS · login · tab root        │
└────────────────────────────┬─────────────────────────────┘
                             │
┌────────────────────────────▼─────────────────────────────┐
│                   Feature modules                        │
│   TPStoreFeature · TPPhotosFeature · TPHomeFeature       │
│                     TPCloudFeature                       │
└──────────────┬──────────────────────────────┬────────────┘
               │                              │
┌──────────────▼──────────────┐ ┌─────────────▼────────────┐
│           TPUI              │ │          TPCore          │
│  badges · alerts · images   │ │  HTTP · auth · clients   │
└─────────────────────────────┘ │  models · WebSocket      │
                                └─────────────┬────────────┘
                                              │ HTTPS · WSS
           ┌──────────────────────┬───────────┴────────────┐
           ▼                      ▼                        ▼
        /auth               /store/api               /home/api
     tpcloud auth           tpstore api              tphome api
```

The whole thing is a Swift package first. Every module builds and tests with `swift build` / `swift test`, no Xcode needed. The `App/` folder is just a thin XcodeGen shell that turns those modules into the iOS and macOS apps, so Xcode only matters for signing and running on a device.

## Modules

| Module | Description |
|---|---|
| [`TPCore`](Sources/TPCore/) | Networking, password + TOTP authentication, cookie session persistence, Keychain credentials, and typed clients and models for every backend (Cloud, Store, Photos, Home) |
| [`TPUI`](Sources/TPUI/) | Shared SwiftUI building blocks: status badges, error alerts, cached remote images |
| [`TPStoreFeature`](Sources/TPStoreFeature/) | File browser, favorites and share-link management |
| [`TPPhotosFeature`](Sources/TPPhotosFeature/) | Gallery, thumbnail cache, photo backup and the background upload engine |
| [`TPHomeFeature`](Sources/TPHomeFeature/) | Floor-plan rendering (walls, lights, blinds) and live device control |
| [`TPCloudFeature`](Sources/TPCloudFeature/) | Account, password, TOTP and sessions |
| [`App/`](App/) | XcodeGen project with the iOS and macOS targets, login screen and tab root |

## How photo backup works

```
Photos in the iPhone library
        │
        ▼
Backup planner picks what tpstore doesn't have yet, newest first
        │
        ▼
Uploads are queued in a background URLSession
(keeps going while the app is suspended or closed)
        │
        ▼
BGProcessingTask refills the queue overnight,
BGContinuedProcessingTask keeps a user-started backup alive
        │
        ▼
Each upload writes a 256px thumbnail to the device cache
        │
        ▼
Gallery draws from the local cache, originals load on open
```

Thumbnails are kept as ready-to-draw bitmaps in memory-mapped tables, so even the 31-column grid scrolls without decoding a single JPEG on the main thread.

## Tech stack

| Layer | Technology |
|---|---|
| **Language** | Swift · Swift Concurrency |
| **UI** | SwiftUI · Liquid Glass · UIKit (`UICollectionView` for the photo grid) |
| **Photos** | PhotoKit · background `URLSession` · BackgroundTasks |
| **Auth** | Password + TOTP · `HTTPCookieStorage` sessions · Keychain |
| **Real time** | `URLSessionWebSocketTask` |
| **Build** | Swift Package Manager · XcodeGen |
| **Platforms** | iOS 27 · macOS 27 |

## How to run

You need Xcode with the iOS 27 / macOS 27 SDKs, and [XcodeGen](https://github.com/yonaskolb/XcodeGen) for the app targets.

```bash
git clone https://github.com/mtaibo/tpcloud
cd tpcloud/apple
make build    # swift build, all modules
make test     # swift test
```

To run the app:

```bash
cp App/Config/Local.xcconfig.example App/Config/Local.xcconfig   # set your DEVELOPMENT_TEAM
make open     # generate the Xcode project and open it
make app      # or: build the macOS app, install it to /Applications and launch it
make device   # or: build the iOS app, install it on the connected iPhone and launch it
```

Every module is a path under one server address (`TPServer`, in TPCore), for now my own, so it needs an account on [tpcloud](../) to sign in. `make testflight` archives the iOS app and uploads it to App Store Connect.
