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

// A door between two rooms fills the wall line along its length, wide enough to square off the
// rooms' rounded corners beside it. A door that runs (nearly) the whole shared edge goes right to
// the rooms' ends, so the two read as one shape with no nick at the corners. The front door
// (rooms on one side only) needs nothing.
const roomAt = (x, y) => (map.storage.rooms ?? []).find(r => x > r.x && x < r.x + r.w && y > r.y && y < r.y + r.h)
const doors = computed(() => (map.storage.doors ?? []).flatMap(d => {
    const horizontal = d.y1 === d.y2
    const mx = (d.x1 + d.x2) / 2, my = (d.y1 + d.y2) / 2
    const a = horizontal ? roomAt(mx, my - 1) : roomAt(mx - 1, my)
    const b = horizontal ? roomAt(mx, my + 1) : roomAt(mx + 1, my)
    if (!a || !b) return []
    // The stretch of wall the two rooms share, and the door along it.
    const lo = horizontal ? Math.max(a.x, b.x) : Math.max(a.y, b.y)
    const hi = horizontal ? Math.min(a.x + a.w, b.x + b.w) : Math.min(a.y + a.h, b.y + b.h)
    let from = horizontal ? Math.min(d.x1, d.x2) : Math.min(d.y1, d.y2)
    let to = horizontal ? Math.max(d.x1, d.x2) : Math.max(d.y1, d.y2)
    from = from - lo < 3 ? lo + GAP / 2 : from
    to = hi - to < 3 ? hi - GAP / 2 : to
    const across = GAP + RADIUS * 2
    return horizontal
        ? [{ x: from, y: d.y1 - across / 2, w: to - from, h: across }]
        : [{ x: d.x1 - across / 2, y: from, w: across, h: to - from }]
}))
</script>

<template>
    <g class="house">
        <rect v-for="(d, i) in doors" :key="i" :x="d.x" :y="d.y" :width="d.w" :height="d.h" class="room" />
        <rect v-for="r in rooms" :key="r.id" :x="r.x" :y="r.y" :width="r.w" :height="r.h" :rx="r.rx" class="room" />
    </g>
</template>

<style scoped>
.room { fill: var(--plan-room); }
</style>
