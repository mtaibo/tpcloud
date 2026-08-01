import { reactive } from 'vue'

// Module-level state — persists for the app lifetime across any component mount/unmount
const _anim    = {}   // id -> { realPos, velocity, moveStartPos, moveStartTime, displayPos, handlePos }
const _pending = {}   // id -> boolean

export const positions       = reactive({})
export const handlePositions = reactive({})

export function getAnim(id, initialPos = 0) {
    if (!_anim[id]) {
        positions[id]       = initialPos
        handlePositions[id] = initialPos
        _anim[id] = {
            realPos:       initialPos,
            velocity:      0,
            moveStartPos:  initialPos,
            moveStartTime: Date.now(),
            displayPos:    initialPos,
            handlePos:     initialPos,
        }
    }
    return _anim[id]
}

export function setPending(id, val) { _pending[id] = !!val }
export function isPending(id)       { return !!_pending[id] }

// Single shared rAF loop — always running, cheap when no blinds are moving
;(function _loop() {
    const now = Date.now()
    for (const [id, a] of Object.entries(_anim)) {
        if (a.velocity === 0) continue
        a.displayPos = Math.max(0, Math.min(100, a.moveStartPos + a.velocity * (now - a.moveStartTime)))
        positions[id] = a.displayPos
        if (!_pending[id]) {
            a.handlePos = a.displayPos
            handlePositions[id] = a.handlePos
        }
    }
    requestAnimationFrame(_loop)
})()
