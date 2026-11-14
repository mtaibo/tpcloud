<script setup>
import { computed } from 'vue'
import { useDevices } from '@/config/devices'
import { api } from '@/config/api'

// Lights on the plan. Their firmware is still in development, so in control mode this only shows
// what the API reports and sends a toggle.
const props = defineProps({
    mode:   { type: String, default: 'control' },
    picked: { type: String, default: null },
})
const emit = defineEmits(['pick'])

const store = useDevices()
const lights = computed(() => (props.mode === 'config' ? (store.storage.lights ?? {}) : store.lights))

const onClick = id => {
    if (props.mode === 'config') return emit('pick', id)
    api.sendCommand(id, 'toggle')
}
</script>

<template>
    <g
        v-for="(light, id) in lights"
        :key="id"
        :transform="`translate(${light.map.x}, ${light.map.y})`"
        class="light"
        :class="{ on: mode === 'control' && light.state?.on, config: mode === 'config', picked: picked === id }"
        @click="onClick(id)"
    >
        <circle r="14" fill="transparent" />
        <circle r="10" class="ring" />
        <circle r="3" class="dot" />
    </g>
</template>

<style scoped>
.light { cursor: pointer; }
.ring { fill: var(--plan-track); stroke: var(--plan-wall); stroke-width: 1.5; transition: fill 0.3s, stroke 0.3s; }
.dot { fill: var(--label-3); transition: fill 0.3s; }
.on .ring { fill: color-mix(in srgb, var(--yellow) 18%, transparent); stroke: var(--yellow); }
.on .dot, .config .dot { fill: var(--yellow); }
.config .ring { stroke: var(--accent); }
.picked .ring { fill: var(--accent); }
</style>
