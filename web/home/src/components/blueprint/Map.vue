<script setup>
import { computed } from 'vue'
import { useBlueprint } from '@/config/blueprint'
import { openEdges } from '@/config/rooms'

// The house from above as cards: every room a rounded surface and a thin gap between rooms for
// the walls. A room can be lit
// warm (how open its blinds are) and picked out while its sheet is open.
const props = defineProps({
    light:     { type: Object, default: () => ({}) },  // room id → 0..1
    selected:  { type: String, default: null },
    clickable: { type: Object, default: () => ({}) },  // room id → true
})
const emit = defineEmits(['select'])

const map = useBlueprint()

const GAP = 4
const RADIUS = 9

const rooms = computed(() => (map.storage.rooms ?? []).map(r => ({
    ...r,
    x: r.x + GAP / 2, y: r.y + GAP / 2,
    w: r.w - GAP, h: r.h - GAP,
    rx: Math.min(RADIUS, (r.w - GAP) / 4, (r.h - GAP) / 4),
})))

// Rooms that are one open space (the entrance and the hall, the lounge and its gallery) are
// joined across the gap along the whole edge they share, a little past the rounded corners and
// clipped to the house. Ordinary doors aren't drawn: the gap between cards is the wall.
const bridges = computed(() => openEdges(map.storage.rooms ?? [], map.storage.doors ?? []).map(e => {
    const across = GAP + RADIUS * 2
    return e.vertical
        ? { room: e.a, x: e.x - across / 2, y: e.y1 + GAP / 2, w: across, h: Math.max(0, e.y2 - e.y1 - GAP) }
        : { room: e.a, x: e.x1 + GAP / 2, y: e.y - across / 2, w: Math.max(0, e.x2 - e.x1 - GAP), h: across }
}))

const fill = id => `color-mix(in srgb, var(--plan-sun) ${Math.round((props.light[id] ?? 0) * 100)}%, var(--plan-room))`
</script>

<template>
    <defs>
        <clipPath id="plan-house">
            <!-- A hair larger than each room, so the clip has no seam where two rooms touch -->
            <rect v-for="r in map.storage.rooms" :key="r.id" :x="r.x - 0.5" :y="r.y - 0.5" :width="r.w + 1" :height="r.h + 1" />
        </clipPath>
    </defs>

    <g class="house">
        <rect v-for="b in bridges" :key="`${b.x},${b.y}`" :x="b.x" :y="b.y" :width="b.w" :height="b.h" class="bridge" :style="{ fill: fill(b.room) }" clip-path="url(#plan-house)" />
        <rect
            v-for="r in rooms"
            :key="r.id"
            :x="r.x" :y="r.y" :width="r.w" :height="r.h" :rx="r.rx"
            class="room"
            :class="{ selected: selected === r.id, clickable: clickable[r.id] }"
            :style="{ fill: fill(r.id) }"
            @click="clickable[r.id] && emit('select', r.id)"
        />
    </g>
</template>

<style scoped>
.room { transition: fill 0.6s var(--ease), stroke 0.2s; stroke: transparent; stroke-width: 2; }
.room.clickable { cursor: pointer; }
@media (hover: hover) { .room.clickable:hover { stroke: color-mix(in srgb, var(--label) 12%, transparent); } }
.room.selected { stroke: var(--accent); }
.bridge { transition: fill 0.6s var(--ease); }
</style>
