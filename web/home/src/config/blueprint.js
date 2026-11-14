import { defineStore } from 'pinia'
import { reactive, computed } from 'vue'

import { api } from './api'

export const useBlueprint = defineStore('blueprint', () => {

    const storage = reactive({})
    const unconfigured = computed(() => Object.keys(storage).length === 0)

    async function setup() {

        try {

            const config = await api.getConfig('map')
            if (Object.keys(config).length === 0) return

            for (const key of Object.keys(storage)) delete storage[key]

            storage.viewBox = config.viewBox
            storage.rooms   = config.rooms
            storage.labels  = config.labels
            storage.doors   = config.doors

        } catch (error) { console.error('TPHome - Map setup error:', error) }
    }

    return { storage, unconfigured, setup }
})