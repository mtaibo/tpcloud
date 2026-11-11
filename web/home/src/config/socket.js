import { ref } from 'vue'

import { useDevices } from './devices'
import { useBlueprint } from './blueprint'

const WS_URL = `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/home/api/ws`

let socket = null
let reconnectTimer = null

// Red until the first socket opens, so the sidebar's light never claims a connection it doesn't have.
export const apiOnline = ref(false)

const _listeners = {}

export function onEvent(type, fn) {
    if (!_listeners[type]) _listeners[type] = []
    _listeners[type].push(fn)
    return () => { _listeners[type] = _listeners[type].filter(f => f !== fn) }
}

function handle(store, type, data) {
    if (_listeners[type]) _listeners[type].forEach(fn => fn(data))

    const category = Object.keys(store.storage).find(cat => data.id in store.storage[cat])

    switch (type) {

        case 'device_state':
            if (!category) return
            store.storage[category][data.id].state = {
                position:    data.state.position,
                motor_state: data.state.motor_state,
            }
            break

        case 'device_online':
        case 'device_offline':
            if (!category) return
            store.storage[category][data.id].connection.online = type === 'device_online'
            break

        case 'device_info':
            if (!category) return
            store.storage[category][data.id].prefs = { ...data.prefs }
            break

        case 'device_lock':
            if (!category) return
            store.storage[category][data.id].locked = data.locked
            break
    }
}

function connect() {
    if (reconnectTimer) { clearTimeout(reconnectTimer); reconnectTimer = null }

    socket = new WebSocket(WS_URL)

    socket.onopen = async () => {
        console.log('[WS] Connected')
        apiOnline.value = true
        const devices = useDevices()
        const map = useBlueprint()
        // After a reconnect, catch up with whatever moved while the socket was down.
        if (devices.unconfigured) await devices.setup()
        else await devices.update()
        if (map.unconfigured) await map.setup()
    }

    socket.onmessage = (event) => {
        const { event: type, data } = JSON.parse(event.data)
        handle(useDevices(), type, data)
    }

    socket.onclose = () => {
        console.log('[WS] Disconnected, reconnecting in 3s...')
        apiOnline.value = false
        reconnectTimer = setTimeout(connect, 3000)
    }

    socket.onerror = (error) => console.error('[WS] Error:', error)
}

export const socket_manager = { connect }
