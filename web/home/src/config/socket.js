import { ref } from 'vue'

import { useDevices } from './devices'
import { useBlueprint } from './blueprint'

const WS_URL = `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/api/ws`

let socket = null
let reconnectTimer = null

export const apiOnline = ref(true)

function handle(store, type, data) {
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
        if (devices.unconfigured) await devices.setup()
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
