<script setup>
import { computed } from 'vue'
import { Blinds, ChevronDown, ChevronUp, Lock, Pause } from 'lucide-vue-next'

import BlindSlider from '@/components/BlindSlider.vue'

import { useDevices } from '@/config/devices'
import { positions, handlePositions } from '@/config/useBlindAnimations'
import { dragMove, dragStart, moveBlind, setBlind, stopBlind } from '@/config/blindControl'
import { t } from '@/ui/i18n.js'

// Every blind as a tile, like Home's accessories: its bar and up / stop / down right there.
// The tile itself does nothing when tapped; only its controls do.
const store = useDevices()
const blinds = computed(() => Object.entries(store.blinds).map(([id, d]) => ({ id, ...d })))

const pos = id => Math.round(positions[id] ?? store.blinds[id]?.state?.position ?? 0)
</script>

<template>
    <div class="view">
        <div class="page blinds-page">
            <header class="page-header">
                <h1 class="large-title">{{ t('Blinds', 'Persianas') }}</h1>
            </header>

            <div v-if="!blinds.length" class="empty">
                <Blinds :size="44" :stroke-width="1.5" class="empty-icon" />
                <h3>{{ t('No blinds', 'No hay persianas') }}</h3>
                <p class="subheadline">{{ t('When you set one up in Devices it will show up here.', 'Cuando configures una en Dispositivos aparecerá aquí.') }}</p>
            </div>

            <div v-else class="grid">
                <div
                    v-for="b in blinds"
                    :key="b.id"
                    class="tile"
                    :class="{ open: pos(b.id) > 0, offline: !b.connection?.online }"
                >
                    <div class="tile-side">
                        <span class="tile-icon">
                            <Blinds :size="22" :stroke-width="2" />
                            <i class="level" :style="{ height: `${100 - pos(b.id)}%` }" />
                        </span>
                        <div class="name-row">
                            <span class="headline name">{{ b.name }}</span>
                            <Lock v-if="b.locked" :size="15" :stroke-width="2.2" class="lock" />
                        </div>
                    </div>
                    <div class="stage">
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
                            <button type="button" :aria-label="t('Raise', 'Subir')" :disabled="b.locked" @click="moveBlind(b.id, 'up')"><ChevronUp :size="20" :stroke-width="2.4" /></button>
                            <button type="button" :aria-label="t('Stop', 'Parar')" :disabled="b.locked" @click="stopBlind(b.id)"><Pause :size="15" :stroke-width="2.4" fill="currentColor" /></button>
                            <button type="button" :aria-label="t('Lower', 'Bajar')" :disabled="b.locked" @click="moveBlind(b.id, 'down')"><ChevronDown :size="20" :stroke-width="2.4" /></button>
                        </div>
                    </div>
                </div>
            </div>
        </div>

    </div>
</template>

<style scoped>
.grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 260px), 1fr));
    gap: 14px;
}
/* A little wider than the other screens, so four tiles fit a row on a large display. */
.view .blinds-page { max-width: 1180px; }
/* Icon on top and name at the bottom on the left; the bar and its buttons on the right. */
.tile {
    min-width: 0;
    display: flex;
    gap: 16px;
    padding: 16px 16px 16px 18px;
    border-radius: var(--radius-card);
    background: var(--cell);
    transition: opacity 0.3s;
}
.tile.offline { opacity: 0.55; }
.tile-side { flex: 1; min-width: 0; display: flex; flex-direction: column; justify-content: space-between; gap: 12px; }
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
.name-row { display: flex; align-items: flex-end; gap: 6px; min-width: 0; }
.name { min-width: 0; overflow-wrap: anywhere; line-height: 22px; }
.lock { flex-shrink: 0; margin-bottom: 3px; color: var(--label-3); }

/* The bar as tall as the three buttons and twice as wide as one, as in the sheet. */
.stage { --round: 40px; --gap: 8px; flex-shrink: 0; display: flex; gap: 10px; }
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

/* Two a row on a phone: the controls on top, the name under them, no round icon. */
@media (max-width: 480px) {
    .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
    .tile { flex-direction: column-reverse; padding: 14px; gap: 12px; }
    .tile-icon { display: none; }
    .stage { align-self: center; }
    .name { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
}
</style>
