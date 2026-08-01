<script setup>

    import { computed } from 'vue'
    import { useBlueprint } from '@/config/blueprint'

    const map = useBlueprint()

    const vb = computed(() => {
        const parts = (map.storage.viewBox ?? '0 0 0 0').split(' ').map(Number)
        return { w: parts[2] ?? 0, h: parts[3] ?? 0 }
    })

    const roomRx = computed(() => Math.round(Math.min(vb.value.w || 100, vb.value.h || 100) * 0.08))

    // Classify wall segments by testing each boundary only within the room that owns it.
    // This avoids generating phantom walls where a room-corner y/x passes through another room's interior.
    const wallSegments = computed(() => {
        const rooms = map.storage.rooms
        if (!rooms) return { exterior: [], interiorPath: '' }
        const xs = [...new Set(rooms.flatMap(r => [r.x, r.x + r.w]))].sort((a, b) => a - b)
        const ys = [...new Set(rooms.flatMap(r => [r.y, r.y + r.h]))].sort((a, b) => a - b)
        const inRoom = (x, y) => rooms.some(r => x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h)
        const sk = (x1, y1, x2, y2) => `${x1},${y1},${x2},${y2}`

        const exteriorSet = new Set(), interiorSet = new Set()
        const exterior = [], interiorParts = []

        // Horizontal boundaries: only test x-intervals within the room that has that y-edge
        for (const y of ys) {
            for (const room of rooms.filter(r => r.y === y || r.y + r.h === y)) {
                const rxs = xs.filter(x => x >= room.x && x <= room.x + room.w)
                for (let i = 0; i < rxs.length - 1; i++) {
                    const xm = (rxs[i] + rxs[i + 1]) / 2
                    const above = inRoom(xm, y - 0.5), below = inRoom(xm, y + 0.5)
                    const key = sk(rxs[i], y, rxs[i + 1], y)
                    if (above !== below && !exteriorSet.has(key)) {
                        exteriorSet.add(key)
                        exterior.push({ x1: rxs[i], y1: y, x2: rxs[i + 1], y2: y })
                    } else if (above && below && !interiorSet.has(key)) {
                        interiorSet.add(key)
                        interiorParts.push(`M${rxs[i]},${y}L${rxs[i + 1]},${y}`)
                    }
                }
            }
        }

        // Vertical boundaries: only test y-intervals within the room that has that x-edge
        for (const x of xs) {
            for (const room of rooms.filter(r => r.x === x || r.x + r.w === x)) {
                const rys = ys.filter(y => y >= room.y && y <= room.y + room.h)
                for (let j = 0; j < rys.length - 1; j++) {
                    const ym = (rys[j] + rys[j + 1]) / 2
                    const left = inRoom(x - 0.5, ym), right = inRoom(x + 0.5, ym)
                    const key = sk(x, rys[j], x, rys[j + 1])
                    if (left !== right && !exteriorSet.has(key)) {
                        exteriorSet.add(key)
                        exterior.push({ x1: x, y1: rys[j], x2: x, y2: rys[j + 1] })
                    } else if (left && right && !interiorSet.has(key)) {
                        interiorSet.add(key)
                        interiorParts.push(`M${x},${rys[j]}L${x},${rys[j + 1]}`)
                    }
                }
            }
        }

        return { exterior, interiorPath: interiorParts.join('') }
    })

    // Trace exterior segments into a closed polygon with rounded corners
    const exteriorOutlinePath = computed(() => {
        const segs = wallSegments.value.exterior
        if (!segs.length) return ''

        const adj = new Map()
        const k = (x, y) => `${x},${y}`
        for (const seg of segs) {
            const k1 = k(seg.x1, seg.y1), k2 = k(seg.x2, seg.y2)
            if (!adj.has(k1)) adj.set(k1, { x: seg.x1, y: seg.y1, nb: [] })
            if (!adj.has(k2)) adj.set(k2, { x: seg.x2, y: seg.y2, nb: [] })
            adj.get(k1).nb.push(k2)
            adj.get(k2).nb.push(k1)
        }

        const startKey = [...adj.keys()].reduce((a, b) => {
            const na = adj.get(a), nb = adj.get(b)
            return nb.y < na.y || (nb.y === na.y && nb.x < na.x) ? b : a
        })

        const raw = []
        let cur = startKey, prev = null
        do {
            const node = adj.get(cur)
            raw.push({ x: node.x, y: node.y })
            const next = node.nb.find(n => n !== prev) ?? node.nb[0]
            prev = cur
            cur = next
        } while (cur !== startKey && raw.length <= segs.length)

        // Drop collinear vertices
        const n = raw.length
        const poly = raw.filter((c, i) => {
            const p = raw[(i - 1 + n) % n], nx = raw[(i + 1) % n]
            return (c.x - p.x) * (nx.y - p.y) - (c.y - p.y) * (nx.x - p.x) !== 0
        })

        if (poly.length < 3) return ''

        // Quadratic bezier arcs only at convex corners (cross product > 0).
        // Concave corners (inward notches) get a sharp angle to avoid visual gaps.
        const r = 12
        const m = poly.length
        let d = ''

        for (let i = 0; i < m; i++) {
            const pv = poly[(i - 1 + m) % m]
            const cv = poly[i]
            const nx = poly[(i + 1) % m]

            const dx1 = cv.x - pv.x, dy1 = cv.y - pv.y
            const len1 = Math.hypot(dx1, dy1)
            const dx2 = nx.x - cv.x, dy2 = nx.y - cv.y
            const len2 = Math.hypot(dx2, dy2)

            const cross = dx1 * dy2 - dy1 * dx2

            if (cross > 0) {
                const cr = Math.min(r, len1 / 2, len2 / 2)
                const p1x = cv.x - (dx1 / len1) * cr, p1y = cv.y - (dy1 / len1) * cr
                const p2x = cv.x + (dx2 / len2) * cr, p2y = cv.y + (dy2 / len2) * cr
                d += i === 0 ? `M${p1x},${p1y}` : `L${p1x},${p1y}`
                d += `Q${cv.x},${cv.y} ${p2x},${p2y}`
            } else {
                d += i === 0 ? `M${cv.x},${cv.y}` : `L${cv.x},${cv.y}`
            }
        }

        return d + 'Z'
    })

</script>

<template>

    <defs>
        <mask id="door-mask">
            <rect :width="vb.w" :height="vb.h" fill="white"/>
            <line
                v-for="(door, i) in map.storage.doors"
                :key="i"
                :x1="door.x1" :y1="door.y1"
                :x2="door.x2" :y2="door.y2"
                stroke="black"
                stroke-width="8"
            />
        </mask>
    </defs>

    <!-- Rooms (fill only) -->
    <g>
        <rect
            v-for="room in map.storage.rooms"
            :key="room.id"
            :x="room.x" :y="room.y"
            :width="room.w" :height="room.h"
            :rx="roomRx" :ry="roomRx"
            fill="none"
        />
    </g>

    <!-- Interior walls (shared walls between rooms) -->
    <path
        :d="wallSegments.interiorPath"
        fill="none"
        class="stroke-2"
        style="stroke: rgba(255,255,255,0.80);"
        stroke-linecap="round"
        mask="url(#door-mask)"
    />

    <!-- Exterior outline with rounded corners -->
    <path
        :d="exteriorOutlinePath"
        fill="none"
        class="stroke-2"
        style="stroke: rgba(255,255,255,0.80);"
        stroke-linecap="round"
        mask="url(#door-mask)"
    />

    <!-- Room labels -->
    <g font-family="-apple-system, BlinkMacSystemFont, 'SF Pro Text', 'SF Pro Display', sans-serif"
       font-size="11"
       font-weight="500"
       fill="rgba(255,255,255,0.68)"
       letter-spacing="1.4"
       class="uppercase pointer-events-none">
        <text
            v-for="(label, i) in map.storage.labels"
            :key="i"
            :x="label.x" :y="label.y"
            text-anchor="middle"
        >{{ label.text }}</text>
    </g>

</template>
