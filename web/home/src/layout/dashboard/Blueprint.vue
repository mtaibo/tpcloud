<script setup>
import { computed, ref } from 'vue'
import { Map, Settings } from 'lucide-vue-next'

import Sheet from '@/ui/Sheet.vue'
import Segmented from '@/ui/Segmented.vue'
import Plan from '@/components/blueprint/Plan.vue'
import BlindsControl from './BlindsControl.vue'

import { useDevices } from '@/config/devices'
import { useBlueprint } from '@/config/blueprint'
import { roomName, useRooms } from '@/config/rooms'
import { navigateToDevice } from '@/config/sections'
import { t } from '@/ui/i18n.js'

// The house from above: rooms as cards, warmer the more daylight their blinds let in, and each
// blind on its window. Tap a room (or a blind) to open its sheet and move its blinds.
const store = useDevices()
const map = useBlueprint()
const rooms = useRooms()

const roomId = ref(null)
const blindId = ref(null)

const room = computed(() => rooms.value.find(r => r.id === roomId.value) || null)
const roomBlinds = computed(() => (room.value ? room.value.blinds : blindId.value ? [blindId.value] : []))

function select({ room: r, blind }) {
    roomId.value = r ?? null
    const list = r ? rooms.value.find(x => x.id === r)?.blinds ?? [] : []
    blindId.value = blind ?? list[0] ?? null
}

// "Salón Grande" and "Salón Pequeña" in the Salón: the picker only needs "Grande" and "Pequeña".
function shortNames(ids) {
    const names = ids.map(id => store.blinds[id]?.name ?? id)
    const words = names.map(n => n.split(' '))
    let common = 0
    while (words.every(w => w.length > common + 1 && w[common] === words[0][common])) common++
    return ids.map((id, i) => ({ value: id, label: words[i].slice(common).join(' ') }))
}
const options = computed(() => shortNames(roomBlinds.value))

const title = computed(() => {
    if (roomBlinds.value.length === 1) return store.blinds[roomBlinds.value[0]]?.name ?? ''
    return room.value ? roomName(room.value.label.text) : ''
})

function settings() {
    const id = blindId.value
    roomId.value = blindId.value = null
    navigateToDevice(id)
}
const open = computed({
    get: () => !!blindId.value && !!store.blinds[blindId.value],
    set: v => { if (!v) roomId.value = blindId.value = null },
})
</script>

<template>
    <div class="view">
        <div class="page plan-page">
            <header class="page-header">
                <h1 class="large-title">{{ t('Map', 'Plano') }}</h1>
            </header>

            <div v-if="map.storage.viewBox" class="plan-card">
                <Plan :selected="open ? roomId : null" @select="select" />
            </div>
            <div v-else class="empty">
                <Map :size="44" :stroke-width="1.5" class="empty-icon" />
                <h3>{{ t('No map', 'Sin plano') }}</h3>
                <p class="subheadline">{{ t('The API doesn’t have the house’s map yet.', 'La API todavía no tiene el plano de la casa.') }}</p>
            </div>
        </div>

        <Sheet v-model:open="open" :title="title">
            <template #trailing>
                <button class="icon-btn" type="button" :aria-label="t('Device settings', 'Ajustes del dispositivo')" @click="settings">
                    <Settings :size="20" :stroke-width="2" />
                </button>
            </template>
            <Segmented v-if="options.length > 1" v-model="blindId" :options="options" class="picker" />
            <BlindsControl v-if="blindId" :key="blindId" :id="blindId" />
        </Sheet>
    </div>
</template>

<style scoped>
/* The plan takes the rest of the screen under the title and sits in its middle. */
.plan-page { min-height: calc(100dvh - 24px - env(safe-area-inset-top)); display: flex; flex-direction: column; }
.plan-card { flex: 1; display: flex; align-items: center; justify-content: center; padding: 8px 0 40px; }
.plan-card :deep(.plan) { width: min(100%, calc((100dvh - 190px) * var(--ratio))); }
.picker { margin-bottom: 18px; }
@media (max-width: 767px) {
    .plan-page { min-height: calc(100dvh - 24px - env(safe-area-inset-top) - 110px - env(safe-area-inset-bottom)); padding-bottom: 0 !important; }
    .plan-card :deep(.plan) { width: 100%; }
}
</style>
