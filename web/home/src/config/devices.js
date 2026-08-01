import { defineStore } from 'pinia'
import { reactive, computed } from 'vue'

import { api } from './api'

export const useDevices = defineStore('devices', () => {

    const storage        = reactive({})
    const unconfigured   = computed(() =>
        Object.keys(storage.blinds ?? {}).length === 0 &&
        Object.keys(storage.lights ?? {}).length === 0
    )
    const blinds         = computed(() =>
        Object.fromEntries(
            Object.entries(storage.blinds ?? {}).filter(([, d]) => d.state != null)
        )
    )
    const lights         = computed(() =>
        Object.fromEntries(
            Object.entries(storage.lights ?? {}).filter(([, d]) => d.state != null)
        )
    )
    const active         = computed(() =>
        Object.values(storage.blinds ?? {}).filter(d => d.connection?.online).length
    )

    async function setup() {
        try {
            const [blindsData, lightsData] = await Promise.all([
                api.getDevicesByCategory('blinds'),
                api.getDevicesByCategory('lights'),
            ])

            for (const key of Object.keys(storage)) delete storage[key]

            storage.blinds = {}
            for (const device of blindsData) {
                storage.blinds[device.id] = {
                    name:       device.name,
                    map:        device.map,
                    prefs:      device.prefs,
                    state:      device.state,
                    connection: device.connection,
                }
            }

            storage.lights = {}
            for (const device of lightsData) {
                storage.lights[device.id] = {
                    name:       device.name,
                    map:        device.map,
                    state:      device.state,
                    connection: device.connection,
                }
            }
        } catch (error) { console.error('TPHome - Setup error:', error) }
    }

    async function update() {
        try {
            const [blindsData, lightsData] = await Promise.all([
                api.getDevicesByCategory('blinds'),
                api.getDevicesByCategory('lights'),
            ])

            for (const device of blindsData) {
                if (!storage.blinds?.[device.id]) continue
                storage.blinds[device.id].connection = device.connection
                storage.blinds[device.id].state      = device.state
            }

            for (const device of lightsData) {
                if (!storage.lights?.[device.id]) continue
                storage.lights[device.id].connection = device.connection
                storage.lights[device.id].state      = device.state
            }
        } catch (error) { console.error('TPHome - Update error:', error) }
    }

    return { storage, unconfigured, blinds, lights, active, setup, update }
})
