<script setup>
import { computed } from 'vue'
import { useBlueprint } from '@/config/blueprint'
import { useDevices } from '@/config/devices'
import { roomName, useRooms } from '@/config/rooms'
import { positions } from '@/config/useBlindAnimations'
import { t } from '@/ui/i18n.js'
import HouseLayer  from './Map.vue'
import LightsLayer from './Lights.vue'
import BlindsLayer from './Blinds.vue'

// The house drawn from /config/map with its devices on top. Used to control them (the Plan
// screen: tap a room or a blind) and to say which one a new switch is (placing it). Room names
// are HTML over the drawing, so they keep a readable size however small the plan gets.
const props = defineProps({
    mode:     { type: String, default: 'control' },
    picked:   { type: String, default: null },
    selected: { type: String, default: null }, // room id whose sheet is open
})
const emit = defineEmits(['select', 'pick'])

const map = useBlueprint()
const store = useDevices()
const rooms = useRooms()

const size = computed(() => {
    const [, , w, h] = (map.storage.viewBox ?? '0 0 1 1').split(' ').map(Number)
    return { w, h }
})

const control = computed(() => props.mode === 'control')
const openOf = id => Math.min(100, Math.max(0, Math.round(positions[id] ?? store.blinds[id]?.state?.position ?? 0)))
const usable = id => store.blinds[id]?.connection?.online && !store.blinds[id]?.locked

// How much daylight each room gets: the average of how open its working blinds are.
const light = computed(() => {
    if (!control.value) return {}
    const out = {}
    for (const r of rooms.value) {
        const ids = r.blinds.filter(usable)
        if (!ids.length) continue
        // The rooms that open onto it (the lounge's gallery) share its light.
        const value = 0.12 * ids.reduce((s, id) => s + openOf(id), 0) / ids.length / 100
        for (const m of r.members) out[m] = value
    }
    return out
})
const clickable = computed(() => (control.value ? Object.fromEntries(rooms.value.filter(r => r.blinds.length).map(r => [r.id, true])) : {}))

function summary(r) {
    if (!control.value || !r.blinds.length) return ''
    if (!r.blinds.some(usable)) return r.blinds.some(id => store.blinds[id]?.locked) ? t('Locked', 'Bloqueada') : t('Offline', 'Sin conexión')
    const values = r.blinds.filter(usable).map(openOf)
    const plural = values.length > 1
    if (values.every(v => v >= 100)) return plural ? t('Open', 'Abiertas') : t('Open', 'Abierta')
    if (values.every(v => v <= 0)) return plural ? t('Closed', 'Cerradas') : t('Closed', 'Cerrada')
    const avg = Math.round(values.reduce((s, v) => s + v, 0) / values.length)
    return t(`${avg}%`, `${avg} %`)
}

const pos = l => ({ left: `${(l.x / size.value.w) * 100}%`, top: `${(l.y / size.value.h) * 100}%` })

// A blind tapped on the plan opens its room's sheet with that blind chosen.
function selectBlind(id) {
    const room = rooms.value.find(r => r.blinds.includes(id))
    emit('select', room ? { room: room.id, blind: id } : { room: null, blind: id })
}
</script>

<template>
    <div v-if="map.storage.viewBox" class="plan" :style="{ '--ratio': size.w / size.h }">
        <svg :viewBox="map.storage.viewBox" xmlns="http://www.w3.org/2000/svg">
            <HouseLayer :light="light" :selected="selected" :clickable="clickable" @select="emit('select', { room: $event })" />
            <LightsLayer :mode="mode" :picked="picked" @pick="emit('pick', $event)" />
            <BlindsLayer :mode="mode" :picked="picked" @select="selectBlind" @pick="emit('pick', $event)" />
        </svg>
        <div class="labels">
            <div v-for="r in rooms" :key="r.id" class="label" :style="pos(r.label)">
                <span class="name">{{ roomName(r.label.text) }}</span>
                <span v-if="summary(r)" class="state">{{ summary(r) }}</span>
            </div>
        </div>
    </div>
</template>

<style scoped>
.plan {
    position: relative;
    width: 100%;
    aspect-ratio: var(--ratio);
    container-type: inline-size;
    --plan-bg: var(--bg);
    --plan-room: var(--cell);
    --plan-sun: var(--yellow);
    --plan-track: color-mix(in srgb, var(--label) 14%, var(--plan-room));
    --plan-wall: color-mix(in srgb, var(--label) 30%, transparent);
}
svg { position: absolute; inset: 0; width: 100%; height: 100%; display: block; overflow: visible; }
.labels { position: absolute; inset: 0; pointer-events: none; }
.label {
    position: absolute;
    transform: translate(-50%, -50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    text-align: center;
    white-space: nowrap;
}
.name { font-size: clamp(11px, 2.6cqw, 15px); font-weight: 600; letter-spacing: -0.2px; color: var(--label); }
.state { font-size: clamp(10px, 2.2cqw, 13px); color: var(--label-2); font-variant-numeric: tabular-nums; }
</style>
