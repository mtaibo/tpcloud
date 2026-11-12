import { computed } from 'vue'
import { useBlueprint } from './blueprint'
import { useDevices } from './devices'
import { t } from '@/ui/i18n.js'

// The plan's rooms as the screen uses them: each named room (one with a label in /config/map)
// with the blinds that belong to it. A blind sits on a wall, so it belongs to the room it's
// drawn in; when that room has no name (the gallery, the entrance) it goes to the named room
// that room opens onto through its widest door.

// /config/map names the rooms in Spanish.
const NAMES_EN = { Pasillo: 'Hall', Cocina: 'Kitchen', 'Salón': 'Lounge', 'Baño': 'Bath', 'Hab.': 'Room' }
export const roomName = text => t(NAMES_EN[text] ?? text, text)

const inside = (r, x, y, pad = 0) => x >= r.x - pad && x <= r.x + r.w + pad && y >= r.y - pad && y <= r.y + r.h + pad

// How much of the edge two rooms share is a door: 1 means they're one open space.
function opening(a, b, doors) {
    let shared = 0, open = 0
    const overlap = (a1, a2, b1, b2) => Math.max(0, Math.min(a2, b2) - Math.max(a1, b1))
    for (const [x, ya1, ya2, yb1, yb2] of [[a.x + a.w, a.y, a.y + a.h, b.y, b.y + b.h], [a.x, a.y, a.y + a.h, b.y, b.y + b.h]]) {
        if (x !== b.x && x !== b.x + b.w) continue
        const len = overlap(ya1, ya2, yb1, yb2)
        shared += len
        for (const d of doors) if (d.x1 === d.x2 && d.x1 === x) open += overlap(Math.min(d.y1, d.y2), Math.max(d.y1, d.y2), Math.max(ya1, yb1), Math.min(ya2, yb2))
    }
    for (const [y, xa1, xa2, xb1, xb2] of [[a.y + a.h, a.x, a.x + a.w, b.x, b.x + b.w], [a.y, a.x, a.x + a.w, b.x, b.x + b.w]]) {
        if (y !== b.y && y !== b.y + b.h) continue
        const len = overlap(xa1, xa2, xb1, xb2)
        shared += len
        for (const d of doors) if (d.y1 === d.y2 && d.y1 === y) open += overlap(Math.min(d.x1, d.x2), Math.max(d.x1, d.x2), Math.max(xa1, xb1), Math.min(xa2, xb2))
    }
    return shared ? open / shared : 0
}

// Pairs of rooms that are really one space (a door along most of the edge they share), with
// that edge: the plan draws them joined.
export function openEdges(rooms, doors) {
    const out = []
    for (let i = 0; i < rooms.length; i++) {
        for (let j = i + 1; j < rooms.length; j++) {
            const a = rooms[i], b = rooms[j]
            if (opening(a, b, doors) < 0.7) continue
            const ox1 = Math.max(a.x, b.x), ox2 = Math.min(a.x + a.w, b.x + b.w)
            const oy1 = Math.max(a.y, b.y), oy2 = Math.min(a.y + a.h, b.y + b.h)
            if (ox1 === ox2) out.push({ a: a.id, b: b.id, vertical: true, x: ox1, y1: oy1, y2: oy2 })
            else if (oy1 === oy2) out.push({ a: a.id, b: b.id, vertical: false, y: oy1, x1: ox1, x2: ox2 })
        }
    }
    return out
}

export function useRooms() {
    const map = useBlueprint()
    const store = useDevices()

    return computed(() => {
        const rooms = map.storage.rooms ?? []
        const doors = map.storage.doors ?? []
        const labels = map.storage.labels ?? []

        const labelOf = new Map()
        for (const l of labels) {
            const r = rooms.find(r => inside(r, l.x, l.y))
            if (r && !labelOf.has(r.id)) labelOf.set(r.id, l)
        }
        const owner = r => {
            if (labelOf.has(r.id)) return r.id
            let best = null, bestOpen = 0
            for (const o of rooms) {
                if (o === r || !labelOf.has(o.id)) continue
                const open = opening(r, o, doors)
                if (open > bestOpen) { best = o.id; bestOpen = open }
            }
            return best
        }

        const named = new Map([...labelOf].map(([id, label]) => [id, { id, label, blinds: [], members: [id] }]))
        for (const r of rooms) if (!labelOf.has(r.id)) { const o = owner(r); if (o) named.get(o).members.push(r.id) }
        for (const [id, blind] of Object.entries(store.blinds)) {
            const m = blind.map
            if (!m) continue
            const cx = m.x + m.width / 2, cy = m.y + m.height / 2
            const room = rooms.find(r => inside(r, cx, cy)) ?? rooms.find(r => inside(r, cx, cy, 6))
            const target = room && owner(room)
            if (target) named.get(target).blinds.push(id)
        }
        return [...named.values()]
    })
}
