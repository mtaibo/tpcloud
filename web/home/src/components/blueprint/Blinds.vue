<script setup>
import { computed } from 'vue'
import { useDevices } from '@/config/devices'
import { positions } from '@/config/useBlindAnimations'

// The blinds on the plan, each one drawn as covered as it really is. In "config" mode (placing a
// new switch) they're all shown flat and the picked one is highlighted.
const props = defineProps({
    mode:   { type: String, default: 'control' },
    picked: { type: String, default: null },
})
const emit = defineEmits(['select', 'pick'])

const store = useDevices()

const blinds = computed(() => (props.mode === 'config' ? (store.storage.blinds ?? {}) : store.blinds))

const isHorizontal = blind => blind.map.width > blind.map.height

const cover = (id, blind) => {
    if (props.mode === 'config' || !blind.state) return { w: blind.map.width, h: blind.map.height }
    const t = (100 - (positions[id] ?? blind.state.position)) / 100
    return isHorizontal(blind)
        ? { w: blind.map.width * t, h: blind.map.height }
        : { w: blind.map.width,     h: blind.map.height * t }
}

const onClick = id => emit(props.mode === 'config' ? 'pick' : 'select', id)
</script>

<template>
    <g
        v-for="(blind, id) in blinds"
        :key="id"
        class="blind"
        :class="{ offline: mode === 'control' && !blind.connection?.online, picked: picked === id }"
        @click="onClick(id)"
    >
        <!-- Hit area, a little larger than the blind -->
        <rect
            :x="blind.map.x - 6" :y="blind.map.y - 6"
            :width="blind.map.width + 12" :height="blind.map.height + 12"
            fill="transparent" rx="4"
        />

        <rect
            :x="blind.map.x" :y="blind.map.y" rx="1.5"
            :width="blind.map.width" :height="blind.map.height"
            class="track"
        />

        <rect
            v-if="mode === 'config'"
            :x="blind.map.x" :y="blind.map.y" rx="1.5"
            :width="blind.map.width" :height="blind.map.height"
            class="config-fill"
        />
        <rect
            v-else
            :x="blind.map.x" :y="blind.map.y" rx="1.5"
            :width="cover(id, blind).w" :height="cover(id, blind).h"
            :fill="isHorizontal(blind) ? 'url(#blind-slat-h)' : 'url(#blind-slat-v)'"
        />
    </g>

    <defs>
        <!-- Wide blind seen from above: one stripe per slat, filling left to right. -->
        <pattern id="blind-slat-h" x="0" y="0" width="5" height="7" patternUnits="userSpaceOnUse">
            <rect width="5" height="7" style="fill: var(--plan-slat)" />
            <rect x="4.2" width="0.8" height="7" style="fill: var(--plan-slat-edge)" />
        </pattern>
        <!-- Tall blind seen from the side: one stripe per slat, filling top to bottom. -->
        <pattern id="blind-slat-v" x="0" y="0" width="7" height="4" patternUnits="userSpaceOnUse">
            <rect width="7" height="4" style="fill: var(--plan-slat)" />
            <rect y="3.4" width="7" height="0.6" style="fill: var(--plan-slat-edge)" />
        </pattern>
    </defs>
</template>

<style scoped>
.blind { cursor: pointer; pointer-events: all; transition: opacity 0.3s; }
.blind.offline { opacity: 0.35; }
.track { fill: var(--plan-track); stroke: var(--plan-wall); stroke-width: 0.5; }
.config-fill { fill: color-mix(in srgb, var(--accent) 25%, transparent); transition: fill 0.2s; }
.picked .config-fill { fill: var(--accent); }
</style>
