<script setup>
import { computed } from 'vue'
import { ChevronDown, ChevronUp, Lock, Pause } from 'lucide-vue-next'

import ListSection from '@/ui/ListSection.vue'
import ListRow     from '@/ui/ListRow.vue'
import BlindSlider from '@/components/BlindSlider.vue'

import { useDevices } from '@/config/devices'
import { positions, handlePositions, getAnim } from '@/config/useBlindAnimations'
import { blindStatus, dragMove, dragStart, moveBlind, setBlind, stopBlind } from '@/config/blindControl'
import { navigateToDevice } from '@/config/sections'

// What's inside a blind's sheet: the big slider, up / stop / down, a few positions and its settings.
const props = defineProps({ id: { type: String, required: true } })
const emit = defineEmits(['close'])

const store = useDevices()
const device = computed(() => store.blinds[props.id])
getAnim(props.id, device.value?.state?.position ?? 0)

const fill = computed(() => positions[props.id] ?? 0)
const target = computed(() => handlePositions[props.id] ?? 0)
const disabled = computed(() => !!device.value?.locked)

const presets = [
    { label: 'Cerrar', value: 0 },
    { label: '50 %', value: 50 },
    { label: 'Abrir', value: 100 },
]

function settings() {
    emit('close')
    navigateToDevice(props.id)
}
</script>

<template>
    <div v-if="device" class="control">
        <div class="readout">
            <span class="percent">{{ Math.round(target) }}<small>%</small></span>
            <span class="subheadline secondary">{{ blindStatus(device, fill) }}</span>
            <span v-if="device.locked" class="badge orange"><Lock :size="12" :stroke-width="2.4" />Bloqueada por un administrador</span>
        </div>

        <div class="stage">
            <BlindSlider
                class="slider"
                :fill-pos="fill"
                :handle-pos="target"
                :disabled="disabled"
                @drag-start="dragStart(id, $event)"
                @drag-move="dragMove(id, $event)"
                @drag-end="setBlind(id, $event)"
            />
            <div class="buttons">
                <button class="round" type="button" aria-label="Subir" :disabled="disabled" @click="moveBlind(id, 'up')">
                    <ChevronUp :size="24" :stroke-width="2.4" />
                </button>
                <button class="round" type="button" aria-label="Parar" :disabled="disabled" @click="stopBlind(id)">
                    <Pause :size="20" :stroke-width="2.4" fill="currentColor" />
                </button>
                <button class="round" type="button" aria-label="Bajar" :disabled="disabled" @click="moveBlind(id, 'down')">
                    <ChevronDown :size="24" :stroke-width="2.4" />
                </button>
            </div>
        </div>

        <div class="presets">
            <button
                v-for="p in presets"
                :key="p.value"
                type="button"
                class="btn btn-bordered small"
                :disabled="disabled"
                @click="setBlind(id, p.value)"
            >{{ p.label }}</button>
        </div>

        <ListSection>
            <ListRow icon="gear" tint="gray" title="Ajustes del dispositivo" :value="id" clickable chevron @click="settings" />
        </ListSection>
    </div>
</template>

<style scoped>
.control { display: flex; flex-direction: column; gap: 22px; padding-top: 4px; }
.readout { display: flex; flex-direction: column; align-items: center; gap: 4px; text-align: center; }
.percent { font-family: var(--font-rounded); font-size: 52px; line-height: 1; font-weight: 700; font-variant-numeric: tabular-nums; }
.percent small { margin-left: 2px; font-size: 24px; color: var(--label-2); }
.readout .badge { margin-top: 6px; }
.stage { display: flex; align-items: center; justify-content: center; gap: 22px; }
.slider { width: 132px; height: 280px; }
.buttons { display: flex; flex-direction: column; gap: 14px; }
.round {
    width: 56px;
    height: 56px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: var(--label);
    background: var(--fill-3);
    transition: transform 0.2s var(--spring), background 0.2s;
}
.round:hover:not(:disabled) { background: var(--fill-2); }
.round:active:not(:disabled) { transform: scale(0.9); }
.round:disabled { opacity: 0.35; }
.presets { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; }
.presets .btn { min-height: 40px; font-size: 15px; }
.control :deep(.section) { margin-bottom: 0; }
@media (max-height: 760px) { .slider { height: 220px; } }
</style>
