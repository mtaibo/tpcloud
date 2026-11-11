<script setup>
import { computed, ref } from 'vue'
import { Blinds, ChevronDown, ChevronUp, Lock, Pause, Settings } from 'lucide-vue-next'

import Sheet from '@/ui/Sheet.vue'
import BlindsControl from './BlindsControl.vue'
import BlindSlider from '@/components/BlindSlider.vue'

import { useDevices } from '@/config/devices'
import { positions, handlePositions } from '@/config/useBlindAnimations'
import { dragMove, dragStart, moveBlind, setBlind, stopBlind } from '@/config/blindControl'
import { navigateToDevice } from '@/config/sections'

// Every blind as a tile, like Home's accessories: its bar and up / stop / down right there, as in
// its sheet. Tapping the rest of the tile opens that sheet.
const store = useDevices()
const blinds = computed(() => Object.entries(store.blinds).map(([id, d]) => ({ id, ...d })))

const selectedId = ref(null)
function settings() {
    const id = selectedId.value
    selectedId.value = null
    navigateToDevice(id)
}
const open = computed({
    get: () => !!selectedId.value && !!store.blinds[selectedId.value],
    set: v => { if (!v) selectedId.value = null },
})

const pos = id => Math.round(positions[id] ?? store.blinds[id]?.state?.position ?? 0)
</script>

<template>
    <div class="view">
        <div class="page">
            <header class="page-header">
                <h1 class="large-title">Persianas</h1>
            </header>

            <div v-if="!blinds.length" class="empty">
                <Blinds :size="44" :stroke-width="1.5" class="empty-icon" />
                <h3>No hay persianas</h3>
                <p class="subheadline">Cuando configures una en Dispositivos aparecerá aquí.</p>
            </div>

            <div v-else class="grid">
                <div
                    v-for="b in blinds"
                    :key="b.id"
                    class="tile"
                    :class="{ open: pos(b.id) > 0, offline: !b.connection?.online }"
                    role="button"
                    tabindex="0"
                    @click="selectedId = b.id"
                    @keydown.enter="selectedId = b.id"
                >
                    <div class="tile-top">
                        <div class="tile-side">
                            <span class="tile-icon">
                                <Blinds :size="22" :stroke-width="2" />
                                <i class="level" :style="{ height: `${100 - pos(b.id)}%` }" />
                            </span>
                        </div>
                        <div class="stage" @click.stop>
                            <BlindSlider
                                class="slider"
                                :fill-pos="positions[b.id] ?? 0"
                                :handle-pos="handlePositions[b.id] ?? 0"
                                :disabled="b.locked"
                                @drag-start="dragStart(b.id, $event)"
                                @drag-move="dragMove(b.id, $event)"
                                @drag-end="setBlind(b.id, $event)"
                            />
                            <div class="buttons">
                                <button type="button" aria-label="Subir" :disabled="b.locked" @click="moveBlind(b.id, 'up')"><ChevronUp :size="20" :stroke-width="2.4" /></button>
                                <button type="button" aria-label="Parar" :disabled="b.locked" @click="stopBlind(b.id)"><Pause :size="15" :stroke-width="2.4" fill="currentColor" /></button>
                                <button type="button" aria-label="Bajar" :disabled="b.locked" @click="moveBlind(b.id, 'down')"><ChevronDown :size="20" :stroke-width="2.4" /></button>
                            </div>
                        </div>
                    </div>
                    <div class="name-row">
                        <span class="headline name">{{ b.name }}</span>
                        <Lock v-if="b.locked" :size="15" :stroke-width="2.2" class="lock" />
                    </div>
                </div>
            </div>
        </div>

        <Sheet v-model:open="open" :title="selectedId ? store.blinds[selectedId]?.name : ''">
            <template #trailing>
                <button class="icon-btn" type="button" aria-label="Ajustes del dispositivo" @click="settings">
                    <Settings :size="20" :stroke-width="2" />
                </button>
            </template>
            <BlindsControl v-if="selectedId" :id="selectedId" />
        </Sheet>
    </div>
</template>

<style scoped>
.grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 190px), 1fr));
    gap: 14px;
}
@media (max-width: 480px) { .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; } }
.tile {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px;
    border-radius: var(--radius-card);
    background: var(--cell);
    cursor: pointer;
    transition: transform 0.3s var(--spring), background 0.3s, opacity 0.3s;
}
.tile:active { transform: scale(0.97); }
@media (hover: hover) { .tile:hover { transform: translateY(-2px); } }
.tile.offline { opacity: 0.55; }
.tile-top { display: flex; align-items: flex-start; justify-content: space-between; gap: 10px; }
.tile-side { display: flex; flex-direction: column; align-items: center; gap: 10px; }
.tile-icon {
    position: relative;
    width: 44px;
    height: 44px;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: 50%;
    color: var(--label-2);
    background: var(--fill-3);
    transition: color 0.3s, background 0.3s;
}
.tile.open .tile-icon { color: #fff; background: var(--accent); }
.tile-icon svg { position: relative; z-index: 1; }
.level { position: absolute; inset: 0 0 auto; background: rgba(0, 0, 0, 0.18); transition: height 0.2s linear; }
.tile:not(.open) .level { display: none; }
.lock { flex-shrink: 0; color: var(--label-3); }
.name-row { display: flex; align-items: center; gap: 6px; min-width: 0; }
/* Two tiles a row on a phone: no room for the round icon, the bar and buttons take the width. */
@media (max-width: 480px) {
    .tile-side { display: none; }
    .tile-top { justify-content: center; }
    .stage { --round: 36px; }
}
.name { min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
/* The bar and its buttons, as in the sheet: the bar as tall as the three and twice as wide as one. */
.stage { --round: 40px; --gap: 8px; display: flex; gap: 10px; cursor: default; }
.slider { width: calc(2 * var(--round)); height: calc(3 * var(--round) + 2 * var(--gap)); border-radius: calc(var(--round) / 2); }
.buttons { display: flex; flex-direction: column; gap: var(--gap); }
.buttons button {
    width: var(--round);
    height: var(--round);
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: var(--label);
    background: var(--fill-3);
    transition: transform 0.2s var(--spring), background 0.2s;
}
.buttons button:hover:not(:disabled) { background: var(--fill-2); }
.buttons button:active:not(:disabled) { transform: scale(0.9); }
.buttons button:disabled { opacity: 0.35; }
</style>
