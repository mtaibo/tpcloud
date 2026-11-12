<script setup>
import { computed } from 'vue'
import { useBlueprint } from '@/config/blueprint'
import { t } from '@/ui/i18n.js'
import HouseLayer  from './Map.vue'
import LightsLayer from './Lights.vue'
import BlindsLayer from './Blinds.vue'

// The house drawn from /config/map with its devices on top. Used to control them (the Plan
// screen: tap a blind) and to say which one a new switch is (placing it). Room names are HTML
// over the drawing, so they keep a readable size however small the plan gets.
defineProps({
    mode:   { type: String, default: 'control' },
    picked: { type: String, default: null },
})
const emit = defineEmits(['select', 'pick'])

const map = useBlueprint()

const size = computed(() => {
    const [, , w, h] = (map.storage.viewBox ?? '0 0 1 1').split(' ').map(Number)
    return { w, h }
})

// /config/map names the rooms in Spanish.
const NAMES_EN = { Pasillo: 'Hall', Cocina: 'Kitchen', 'Salón': 'Lounge', 'Baño': 'Bath', 'Hab.': 'Room' }
const roomName = text => t(NAMES_EN[text] ?? text, text)

const pos = l => ({ left: `${(l.x / size.value.w) * 100}%`, top: `${(l.y / size.value.h) * 100}%` })
</script>

<template>
    <div v-if="map.storage.viewBox" class="plan" :style="{ '--ratio': size.w / size.h }">
        <svg :viewBox="map.storage.viewBox" xmlns="http://www.w3.org/2000/svg">
            <HouseLayer />
            <LightsLayer :mode="mode" :picked="picked" @pick="emit('pick', $event)" />
            <BlindsLayer :mode="mode" :picked="picked" @select="emit('select', $event)" @pick="emit('pick', $event)" />
        </svg>
        <div class="labels">
            <span v-for="(l, i) in map.storage.labels" :key="i" class="label" :style="pos(l)">{{ roomName(l.text) }}</span>
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
    --plan-blind: var(--orange);
    --plan-track: color-mix(in srgb, var(--label) 16%, var(--plan-room));
    --plan-wall: color-mix(in srgb, var(--label) 30%, transparent);
}
svg { position: absolute; inset: 0; width: 100%; height: 100%; display: block; overflow: visible; }
.labels { position: absolute; inset: 0; pointer-events: none; }
.label {
    position: absolute;
    transform: translate(-50%, -50%);
    white-space: nowrap;
    font-size: clamp(10px, 2.4cqw, 14px);
    font-weight: 600;
    letter-spacing: -0.1px;
    color: var(--label);
}
/* Apart from the sidebar: in light the house floats on a soft shadow; in dark its rooms are a
   step lighter than the sidebar's glass. */
svg :deep(.house) { filter: drop-shadow(0 1px 1.5px rgba(0, 0, 0, 0.05)) drop-shadow(0 6px 16px rgba(0, 0, 0, 0.06)); }
@media (prefers-color-scheme: dark) {
    .plan { --plan-room: #2c2c2e; }
    svg :deep(.house) { filter: none; }
}
</style>
