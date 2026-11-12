<script setup>
import { computed } from 'vue'
import { useDevices } from '@/config/devices'
import { positions } from '@/config/useBlindAnimations'

// The blinds on the plan: a capsule on the window's wall that fills in the label colour (black in
// light, white in dark) as far as the blind is down, like its slats in the sheet: empty when it's
// fully open, full when it's closed. Gray when it can't be moved, with a padlock while locked. In "config" mode (placing a new switch)
// they're all drawn the same and the picked one in the accent colour.
const props = defineProps({
    mode:   { type: String, default: 'control' },
    picked: { type: String, default: null },
})
const emit = defineEmits(['select', 'pick'])

const store = useDevices()

const THICK = 7

const blinds = computed(() => {
    const source = props.mode === 'config' ? (store.storage.blinds ?? {}) : store.blinds
    return Object.entries(source).filter(([, b]) => b.map).map(([id, b]) => {
        const m = b.map
        const horizontal = m.width >= m.height
        const len = horizontal ? m.width : m.height
        // Centred on the wall line it sits on.
        const x = horizontal ? m.x : m.x + m.width / 2 - THICK / 2
        const y = horizontal ? m.y + m.height / 2 - THICK / 2 : m.y
        return { id, blind: b, horizontal, len, x, y, w: horizontal ? len : THICK, h: horizontal ? THICK : len }
    })
})

// How much of the window it covers, 0..1 (tphome's position is how open it is), following the
// animation while it moves.
const cover = (id, b) => 1 - Math.min(1, Math.max(0, (positions[id] ?? b.state?.position ?? 0) / 100))
const usable = b => b.connection?.online && !b.locked

const onClick = id => emit(props.mode === 'config' ? 'pick' : 'select', id)
</script>

<template>
    <g
        v-for="c in blinds"
        :key="c.id"
        class="blind"
        :class="{ off: mode === 'control' && !usable(c.blind), picked: picked === c.id, config: mode === 'config' }"
        @click.stop="onClick(c.id)"
    >
        <!-- Hit area, well past the capsule -->
        <rect :x="c.x - 10" :y="c.y - 10" :width="c.w + 20" :height="c.h + 20" fill="transparent" />
        <rect :x="c.x" :y="c.y" :width="c.w" :height="c.h" :rx="THICK / 2" class="track" />
        <rect
            v-if="mode === 'control'"
            :x="c.x" :y="c.y" :rx="THICK / 2"
            :width="c.horizontal ? Math.max(THICK, c.len * cover(c.id, c.blind)) : c.w"
            :height="c.horizontal ? c.h : Math.max(THICK, c.len * cover(c.id, c.blind))"
            class="level"
            :class="{ empty: cover(c.id, c.blind) === 0 }"
        />
        <g v-if="mode === 'control' && c.blind.locked" :transform="`translate(${c.x + c.w / 2}, ${c.y + c.h / 2})`" class="lock">
            <circle r="7" />
            <g transform="translate(-4.2, -4.6) scale(0.35)">
                <rect x="5" y="10.5" width="14" height="10" rx="2.5" />
                <path d="M8 10.5V8a4 4 0 0 1 8 0v2.5" />
            </g>
        </g>
    </g>
</template>

<style scoped>
.blind { cursor: pointer; pointer-events: all; }
.track { fill: var(--plan-track); stroke: var(--plan-bg); stroke-width: 2.5; paint-order: stroke; transition: fill 0.2s; }
.level { fill: var(--plan-blind); transition: width 0.4s var(--ease), height 0.4s var(--ease), fill 0.3s; }
.level.empty { fill: transparent; }
.off .level { fill: var(--gray); }
.lock circle { fill: var(--plan-blind); stroke: var(--plan-bg); stroke-width: 1.5; }
.lock rect { fill: var(--plan-bg); }
.lock path { fill: none; stroke: var(--plan-bg); stroke-width: 3; stroke-linecap: round; }
.config .track { fill: color-mix(in srgb, var(--accent) 35%, var(--plan-bg)); }
.picked .track { fill: var(--accent); }
@media (hover: hover) { .blind:not(.config):hover .track { fill: color-mix(in srgb, var(--label) 22%, var(--plan-bg)); } }
</style>
