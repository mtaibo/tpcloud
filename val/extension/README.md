# TPVal Sync

Extensión de navegador (Manifest V3) que sincroniza las cookies de sesión de Riot con el backend de `val.migueltaibo.com` para desbloquear el acceso a tienda, inventario, loadout y wallet.

## Instalación (dev, Chrome / Brave / Edge)

1. Ir a `chrome://extensions`.
2. Activar "Developer mode" (arriba a la derecha).
3. "Load unpacked" → seleccionar la carpeta `val/extension/`.
4. Fijar el icono en la barra.

## Instalación (dev, Firefox)

1. Ir a `about:debugging#/runtime/this-firefox`.
2. "Load Temporary Add-on" → seleccionar `val/extension/manifest.json`.

> En Firefox Manifest V3 aún tiene diferencias con Chrome; si algo del `background.js` no arranca, comprobar `about:debugging` → "Inspect".

## Configuración

1. En `val.migueltaibo.com`, vincular una cuenta Riot.
2. Pulsar en el modal "Generar pair token" → copiar el token.
3. Abrir el popup de la extensión (click en el icono).
4. Pegar el token en "Pair token" → "Guardar configuración".
5. Ir a `https://auth.riotgames.com` y hacer login con tu cuenta Riot.
6. Volver al popup → "Sync now".

Ese flujo se hace una sola vez. La extensión sincroniza cada 72h automáticamente con `chrome.alarms`.

## Iconos

Añadir `icons/icon16.png`, `icons/icon48.png`, `icons/icon128.png` (rojo `#FF4655` con la letra "V" funciona). Se pueden generar desde `icon.svg` con:

```
rsvg-convert -w 16 icon.svg  > icons/icon16.png
rsvg-convert -w 48 icon.svg  > icons/icon48.png
rsvg-convert -w 128 icon.svg > icons/icon128.png
```

Sin iconos la extensión igualmente carga pero Chrome muestra un placeholder gris.

## Debug

- Service worker: `chrome://extensions` → TPVal Sync → "Inspect views: service worker".
- Storage: dentro del inspector, `chrome.storage.local.get(console.log)`.
- Cookies: `chrome.cookies.getAll({domain: '.riotgames.com'}, console.log)`.

## Empaquetar para release

```
cd val/extension
zip -r tpval-sync-<version>.zip . -x '*.DS_Store' -x 'README.md'
```

Subirlo a Chrome Web Store dev dashboard (o cargarlo unpacked en producción).
