// Shared by the plan's layers: the wall line between rooms, in /config/map units.
export const GAP = 2.5

// A blind is drawn on its window, which is on a room's outside edge. /config/map places it
// roughly over the old wall line; this finds that edge and returns where the room's card
// actually ends there (the wall minus half the gap), so the capsule sits centred on it.
export function snapToWall(rooms, blind) {
    const horizontal = blind.width >= blind.height
    const centre = horizontal ? blind.y + blind.height / 2 : blind.x + blind.width / 2
    const from = horizontal ? blind.x : blind.y
    const to = from + (horizontal ? blind.width : blind.height)
    let best = null
    for (const r of rooms) {
        const [lo, hi] = horizontal ? [r.x, r.x + r.w] : [r.y, r.y + r.h]
        if (Math.min(to, hi) - Math.max(from, lo) <= 0) continue
        const edges = horizontal ? [[r.y, 1], [r.y + r.h, -1]] : [[r.x, 1], [r.x + r.w, -1]]
        for (const [edge, inward] of edges) {
            const d = Math.abs(edge - centre)
            if (!best || d < best.d) best = { d, line: edge + inward * GAP / 2 }
        }
    }
    return best && best.d < 12 ? best.line : centre
}
