import { watch } from 'vue'

import { api } from './api'
import { useDevices } from './devices'
import { positions, handlePositions, getAnim, setPending, isPending } from './useBlindAnimations'

// Moving a blind from any screen (its tile, its sheet): starts the predicted animation right
// away and sends the command; the WebSocket's state updates then correct it.

const speed = (device, dir) => 100 / ((device?.prefs?.[dir === 'up' ? 'up_time' : 'down_time'] ?? 3000) * 10)
const dragging = {}

// Called once by the app: keeps every blind's animation in step with the states the API sends.
export function syncBlinds() {
    const store = useDevices()
    watch(() => store.blinds, blinds => {
        for (const [id, device] of Object.entries(blinds)) {
            if (dragging[id]) continue
            const newPos     = device.state?.position ?? 0
            const motorState = device.state?.motor_state ?? 0
            const a = getAnim(id, newPos)
            const prevRealPos = a.realPos
            a.realPos = newPos

            if (motorState === 0) {
                // Motor idle: snap everything
                setPending(id, false)
                a.velocity = 0; a.displayPos = newPos; a.handlePos = newPos
                a.moveStartPos = newPos; a.moveStartTime = Date.now()
                positions[id] = newPos; handlePositions[id] = newPos
            } else if (a.velocity !== 0) {
                // Already animating: just recalibrate the prediction anchor
                a.moveStartPos = newPos; a.moveStartTime = Date.now()
            } else if (isPending(id)) {
                // Waiting for a slider set to confirm: don't touch the display
            } else if (prevRealPos !== newPos) {
                // Moving but started elsewhere (its buttons, another screen): infer the direction
                a.velocity = newPos > prevRealPos ? speed(device, 'up') : -speed(device, 'down')
                a.displayPos = newPos; a.handlePos = newPos
                a.moveStartPos = newPos; a.moveStartTime = Date.now()
                positions[id] = newPos; handlePositions[id] = newPos
            } else {
                a.displayPos = newPos; a.handlePos = newPos
                positions[id] = newPos; handlePositions[id] = newPos
            }
        }
    }, { immediate: true, deep: true })
}

function send(id, command, value = null) {
    return api.sendCommand(id, command, value).catch(e => console.error('TPHome - command error:', e))
}

export function moveBlind(id, dir) {
    const a = getAnim(id, positions[id])
    setPending(id, false)
    a.velocity = (dir === 'up' ? 1 : -1) * speed(useDevices().blinds[id], dir)
    a.moveStartPos = a.displayPos; a.moveStartTime = Date.now()
    return send(id, dir)
}

export function stopBlind(id) {
    const a = getAnim(id)
    a.velocity = 0; a.displayPos = a.realPos; a.handlePos = a.realPos
    a.moveStartPos = a.realPos
    positions[id] = a.realPos; handlePositions[id] = a.realPos
    setPending(id, false)
    return send(id, 'stop')
}

export function dragStart(id, pos) {
    dragging[id] = true
    const a = getAnim(id, pos)
    a.velocity = 0; setPending(id, false)
    a.displayPos = pos; a.handlePos = pos
    positions[id] = pos; handlePositions[id] = pos
}

export function dragMove(id, pos) {
    if (!dragging[id]) return
    const a = getAnim(id)
    a.displayPos = pos; a.handlePos = pos
    positions[id] = pos; handlePositions[id] = pos
}

// Also used to send a position straight away (the presets in the sheet).
export function setBlind(id, pos) {
    dragging[id] = false
    const a = getAnim(id)
    a.handlePos = pos; handlePositions[id] = pos
    a.displayPos = a.realPos; a.moveStartPos = a.realPos; a.moveStartTime = Date.now()
    positions[id] = a.realPos

    const device = useDevices().blinds[id]
    if (pos < a.realPos)      { a.velocity = -speed(device, 'down'); setPending(id, true) }
    else if (pos > a.realPos) { a.velocity = speed(device, 'up');    setPending(id, true) }
    else                      { a.velocity = 0; setPending(id, false) }

    return send(id, 'set', pos)
}

// "Abierta 60 %", "Cerrada", "Sin conexión"…: what a tile or the sheet says under the name.
export function blindStatus(device, pos) {
    if (!device.connection?.online) return 'Sin conexión'
    if (device.state?.motor_state) return 'Moviéndose…'
    const p = Math.round(pos ?? device.state?.position ?? 0)
    if (p <= 0) return 'Cerrada'
    if (p >= 100) return 'Abierta'
    return `Abierta ${p} %`
}
