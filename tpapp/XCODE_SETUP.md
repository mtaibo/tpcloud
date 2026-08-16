# Configuración del proyecto en Xcode

## 1. Crear el proyecto

1. Abre Xcode → **File → New → Project**
2. Selecciona la plantilla **iOS → App** y pulsa **Next**
3. Rellena los campos:
   - **Product Name:** TPApp
   - **Bundle Identifier:** `com.migueltaibo.tpapp`
   - **Interface:** SwiftUI
   - **Language:** Swift
   - **Team:** Ninguno (para simulador) o tu cuenta de desarrollador (para dispositivo real)
4. Elige la carpeta de destino. Puedes guardar el `.xcodeproj` dentro de `tpcloud/tpapp/` o a un nivel superior.

---

## 2. Configurar el target

1. En el navigator lateral, selecciona el proyecto (icono azul) → selecciona el **target TPApp**
2. En la pestaña **General**:
   - **Minimum Deployments → iOS 17.0**
   - Elimina `ContentView.swift` y el asset catalog por defecto si no los vas a usar

---

## 3. Añadir los archivos Swift

1. En el navigator, haz clic derecho en el grupo **TPApp** → **Add Files to "TPApp"...**
2. Navega hasta `tpcloud/tpapp/Sources/TPApp/` y selecciona **todas las carpetas**:
   - `Models/`
   - `Networking/`
   - `WebSocket/`
   - `Auth/`
   - `Cloud/`
   - `Home/`
   - `App/`
3. Asegúrate de que **"Copy items if needed"** está desmarcado (los archivos ya están en el repo) y de que el target `TPApp` está marcado.
4. Pulsa **Add**.

> Xcode creará grupos que reflejan la estructura de carpetas.

---

## 4. Configurar el entitlement de Associated Domains (passkeys)

1. Selecciona el **target TPApp** → pestaña **Signing & Capabilities**
2. Pulsa **"+ Capability"** y añade **Associated Domains**
3. En el campo que aparece, escribe: `webcredentials:migueltaibo.com`

> Alternativamente, selecciona el archivo `TPApp.entitlements` del repo y arrástralo al proyecto, luego en la pestaña **Build Settings** del target busca `Code Signing Entitlements` y apunta a `TPApp.entitlements`.

---

## 5. Info.plist

No se necesitan claves especiales. El framework `AuthenticationServices` y las peticiones de red a HTTPS funcionan sin configuración adicional (App Transport Security permite HTTPS por defecto).

---

## 6. Compilar y ejecutar

- Selecciona el simulador **iPhone 15 (iOS 17+)** o superior
- Pulsa **Run (⌘R)**

El flujo de autenticación con passkey **no funciona en simulador** (el simulador no tiene soporte de passkeys reales). Para pruebas de UI sin passkey, puedes añadir un modo de bypass temporal.

---

## Notas importantes

### Cuenta de desarrollador de pago

Para probar passkeys en un **dispositivo físico** necesitas:
- Una cuenta de desarrollador de Apple de pago (99 $/año)
- El dispositivo registrado en tu cuenta
- El `webcredentials` entitlement firmado con tu equipo

### Archivo AASA en el servidor

Para que iOS reconozca el dominio `migueltaibo.com` como válido para passkeys, el servidor debe servir el archivo Apple App Site Association en:

```
https://migueltaibo.com/.well-known/apple-app-site-association
```

Con contenido similar a:

```json
{
  "webcredentials": {
    "apps": ["TEAMID.com.migueltaibo.tpapp"]
  }
}
```

Sustituye `TEAMID` por tu Apple Team ID (visible en https://developer.apple.com/account).

### Origen iOS en el backend

El backend ya está parcheado para aceptar el origen nativo de iOS:
`ios:bundle-id:com.migueltaibo.tpapp`

Esto permite que las passkeys registradas desde web también funcionen en la app nativa con el mismo `RP_ID = migueltaibo.com`.
