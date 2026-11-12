<script setup>
import { computed } from 'vue'
import { useBlueprint } from '@/config/blueprint'
import { GAP } from './geometry'

// The house from above as cards: every room a softly rounded surface and a thin line of the
// page's background between rooms for the walls. Each door is a break in that line, so the
// house reads as one shape with its rooms marked off, not as loose pieces.
const map = useBlueprint()

const RADIUS = 6

const rooms = computed(() => (map.storage.rooms ?? []).map(r => ({
    id: r.id,
    x: r.x + GAP / 2, y: r.y + GAP / 2,
    w: r.w - GAP, h: r.h - GAP,
    rx: Math.min(RADIUS, (r.w - GAP) / 4, (r.h - GAP) / 4),
})))

// A door between two rooms fills the wall line along its length, a little past the rooms' rounded
// corners, clipped to the house. The front door (rooms on one side only) needs nothing.
const inRoom = (x, y) => (map.storage.rooms ?? []).some(r => x > r.x && x < r.x + r.w && y > r.y && y < r.y + r.h)
const between = d => {
    const mx = (d.x1 + d.x2) / 2, my = (d.y1 + d.y2) / 2
    return d.y1 === d.y2 ? inRoom(mx, my - 1) && inRoom(mx, my + 1) : inRoom(mx - 1, my) && inRoom(mx + 1, my)
}
const doors = computed(() => (map.storage.doors ?? []).filter(between).map(d => {
    const across = GAP + RADIUS * 2
    if (d.y1 === d.y2) {
        const a = Math.min(d.x1, d.x2), b = Math.max(d.x1, d.x2)
        return { x: a, y: d.y1 - across / 2, w: b - a, h: across }
    }
    const a = Math.min(d.y1, d.y2), b = Math.max(d.y1, d.y2)
    return { x: d.x1 - across / 2, y: a, w: across, h: b - a }
}))
</script>

<template>
    <defs>
        <clipPath id="plan-house">
            <!-- A hair larger than each room, so the clip has no seam where two rooms touch -->
            <rect v-for="r in map.storage.rooms" :key="r.id" :x="r.x - 0.5" :y="r.y - 0.5" :width="r.w + 1" :height="r.h + 1" />
        </clipPath>
    </defs>

    <g class="house">
        <rect v-for="(d, i) in doors" :key="i" :x="d.x" :y="d.y" :width="d.w" :height="d.h" class="room" clip-path="url(#plan-house)" />
        <rect v-for="r in rooms" :key="r.id" :x="r.x" :y="r.y" :width="r.w" :height="r.h" :rx="r.rx" class="room" />
    </g>
</template>

<style scoped>
.room { fill: var(--plan-room); }
</style>
