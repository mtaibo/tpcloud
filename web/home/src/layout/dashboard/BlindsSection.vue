<script setup>
import { computed, ref } from 'vue'
import { Blinds, ChevronDown, ChevronUp, Lock, Pause } from 'lucide-vue-next'

import Sheet from '@/ui/Sheet.vue'
import BlindsControl from './BlindsControl.vue'

import { useDevices } from '@/config/devices'
import { positions } from '@/config/useBlindAnimations'
import { blindStatus, moveBlind, stopBlind } from '@/config/blindControl'

// Every blind as a tile, like Home's accessories: how open it is, and up / stop / down right there.
// Tapping the tile opens its sheet with the big slider.
const store = useDevices()
const blinds = computed(() => Object.entries(store.blinds).map(([id, d]) => ({ id, ...d })))

const selectedId = ref(null)
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
                        <span class="tile-icon">
                            <Blinds :size="22" :stroke-width="2" />
                            <i class="level" :style="{ height: `${100 - pos(b.id)}%` }" />
                        </span>
                        <Lock v-if="b.locked" :size="15" :stroke-width="2.2" class="lock" />
                    </div>
                    <div class="tile-text">
                        <span class="headline name">{{ b.name }}</span>
                        <span class="footnote secondary">{{ b.locked ? 'Bloqueada' : blindStatus(b, positions[b.id]) }}</span>
                    </div>
                    <div class="tile-actions" @click.stop>
                        <button type="button" aria-label="Subir" :disabled="b.locked" @click="moveBlind(b.id, 'up')"><ChevronUp :size="20" :stroke-width="2.4" /></button>
                        <button type="button" aria-label="Parar" :disabled="b.locked" @click="stopBlind(b.id)"><Pause :size="16" :stroke-width="2.4" fill="currentColor" /></button>
                        <button type="button" aria-label="Bajar" :disabled="b.locked" @click="moveBlind(b.id, 'down')"><ChevronDown :size="20" :stroke-width="2.4" /></button>
                    </div>
                </div>
            </div>
        </div>

        <Sheet v-model:open="open" :title="selectedId ? store.blinds[selectedId]?.name : ''">
            <BlindsControl v-if="selectedId" :id="selectedId" @close="selectedId = null" />
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
.tile-top { display: flex; align-items: flex-start; justify-content: space-between; }
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
.lock { color: var(--label-3); }
.tile-text { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.name { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.tile-actions { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; }
.tile-actions button {
    height: 36px;
    display: grid;
    place-items: center;
    border-radius: 12px;
    color: var(--label);
    background: var(--fill-3);
    transition: transform 0.2s var(--spring), background 0.2s;
}
.tile-actions button:hover:not(:disabled) { background: var(--fill-2); }
.tile-actions button:active:not(:disabled) { transform: scale(0.9); }
.tile-actions button:disabled { opacity: 0.35; }
</style>
