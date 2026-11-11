<script setup>
// A blind as a big control, like Home's: the slats come down from the top as far as the blind
// is closed, and dragging anywhere sets where it should go (the white bar), 100 = open.
defineProps({
    fillPos:   { type: Number, default: 0 },
    handlePos: { type: Number, default: 0 },
    disabled:  { type: Boolean, default: false },
})
const emit = defineEmits(['drag-start', 'drag-move', 'drag-end'])

function posFromY(el, clientY) {
    const rect = el.getBoundingClientRect()
    return Math.max(0, Math.min(100, Math.round((1 - (clientY - rect.top) / rect.height) * 100)))
}

function onPointerDown(e) {
    e.currentTarget.setPointerCapture(e.pointerId)
    emit('drag-start', posFromY(e.currentTarget, e.clientY))
}
function onPointerMove(e) {
    if (!e.currentTarget.hasPointerCapture(e.pointerId)) return
    emit('drag-move', posFromY(e.currentTarget, e.clientY))
}
function onPointerUp(e) {
    if (!e.currentTarget.hasPointerCapture(e.pointerId)) return
    emit('drag-end', posFromY(e.currentTarget, e.clientY))
}
</script>

<template>
    <div
        class="blind-slider"
        :class="{ disabled }"
        role="slider"
        aria-label="Posición de la persiana"
        :aria-valuenow="Math.round(handlePos)"
        aria-valuemin="0"
        aria-valuemax="100"
        @pointerdown="onPointerDown"
        @pointermove="onPointerMove"
        @pointerup="onPointerUp"
        @pointercancel="onPointerUp"
    >
        <div class="slats" :style="{ height: `${100 - fillPos}%` }">
            <i v-for="i in 40" :key="i" />
        </div>
        <div class="handle" :style="{ top: `clamp(6px, calc(${100 - handlePos}% - 3px), calc(100% - 12px))` }" />
    </div>
</template>

<style scoped>
.blind-slider {
    position: relative;
    overflow: hidden;
    border-radius: 30px;
    background: var(--fill-3);
    touch-action: none;
    cursor: grab;
}
.blind-slider:active { cursor: grabbing; }
.blind-slider.disabled { opacity: 0.4; pointer-events: none; }
.slats {
    position: absolute;
    inset: 0 0 auto;
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 5px 5px 0;
    overflow: hidden;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    box-shadow: inset 0 -1px 0 color-mix(in srgb, var(--accent) 45%, transparent);
}
.slats i {
    flex-shrink: 0;
    height: 6px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--accent) 55%, var(--cell));
}
.handle {
    position: absolute;
    left: 22%;
    right: 22%;
    height: 6px;
    border-radius: 3px;
    background: #fff;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.3);
    pointer-events: none;
}
</style>
