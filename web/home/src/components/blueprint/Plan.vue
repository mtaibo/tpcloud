<script setup>
import { useBlueprint } from '@/config/blueprint'
import HouseLayer  from './Map.vue'
import LightsLayer from './Lights.vue'
import BlindsLayer from './Blinds.vue'

// The house drawn from /config/map with its devices on top. Used to control them (the Plan
// screen) and to say which one a new switch is (placing it).
defineProps({
    mode:   { type: String, default: 'control' },
    picked: { type: String, default: null },
})
const emit = defineEmits(['select', 'pick'])
const map = useBlueprint()
</script>

<template>
    <svg
        v-if="map.storage.viewBox"
        :viewBox="map.storage.viewBox"
        class="plan"
        xmlns="http://www.w3.org/2000/svg"
    >
        <HouseLayer />
        <LightsLayer :mode="mode" :picked="picked" @pick="emit('pick', $event)" />
        <BlindsLayer :mode="mode" :picked="picked" @select="emit('select', $event)" @pick="emit('pick', $event)" />
    </svg>
</template>

<style scoped>
.plan {
    display: block;
    width: 100%;
    height: auto;
    --plan-wall: color-mix(in srgb, var(--label) 80%, transparent);
    --plan-label: var(--label-2);
    --plan-track: var(--bg);
    --plan-slat: color-mix(in srgb, var(--label) 82%, transparent);
    --plan-slat-edge: color-mix(in srgb, var(--cell) 40%, transparent);
}
</style>
