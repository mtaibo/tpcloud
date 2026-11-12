<script setup>
import { computed, ref } from 'vue'
import { ChevronDown, ChevronUp, Lock, LockOpen, Pause } from 'lucide-vue-next'

import BlindSlider from '@/components/BlindSlider.vue'

import { useDevices } from '@/config/devices'
import { positions, handlePositions, getAnim } from '@/config/useBlindAnimations'
import { dragMove, dragStart, moveBlind, setBlind, stopBlind } from '@/config/blindControl'
import { t } from '@/ui/i18n.js'
import { ask, toast } from '@/ui/dialog.js'
import { api } from '@/config/api'
import { session } from '@/config/user'

// What's inside a blind's sheet: the blind as a bar as tall as up / stop / down, beside them.
// Its settings are the gear in the sheet's bar.
const props = defineProps({ id: { type: String, required: true } })

const store = useDevices()
const device = computed(() => store.blinds[props.id])
getAnim(props.id, device.value?.state?.position ?? 0)

const fill = computed(() => positions[props.id] ?? 0)
const target = computed(() => handlePositions[props.id] ?? 0)
const disabled = computed(() => !!device.value?.locked)

// Admins can lock a blind from here too (the API checks it again): a locked switch turns its
// lights off and ignores its buttons and every command until it's unlocked.
const isAdmin = computed(() => session.user?.is_admin === true)
const locking = ref(false)
async function toggleLock() {
    const lock = !device.value.locked
    if (lock) {
        const ok = await ask({
            title: t(`Lock ${device.value.name}?`, `¿Bloquear ${device.value.name}?`),
            message: t(
                'Its lights will go off and it won’t move, from its buttons or from here, until an administrator unlocks it.',
                'Se apagarán sus luces y no se podrá mover ni con los botones ni desde aquí hasta que un administrador la desbloquee.',
            ),
            actions: [
                { label: t('Cancel', 'Cancelar'), role: 'cancel', value: false },
                { label: t('Lock', 'Bloquear'), role: 'destructive', value: true },
            ],
        })
        if (!ok) return
    }
    locking.value = true
    try {
        await api.setLocked(props.id, lock)
        store.storage.blinds[props.id].locked = lock
        toast(lock ? t('Locked', 'Bloqueada') : t('Unlocked', 'Desbloqueada'))
    } catch (e) {
        toast(e.response?.data?.detail || t('Couldn’t change the lock', 'No se pudo cambiar el bloqueo'), 'warning')
    } finally {
        locking.value = false
    }
}
</script>

<template>
    <div v-if="device" class="control blind-control">
        <span v-if="device.locked" class="badge orange"><Lock :size="12" :stroke-width="2.4" />{{ t('Locked by an administrator', 'Bloqueada por un administrador') }}</span>

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
                <button class="round" type="button" :aria-label="t('Raise', 'Subir')" :disabled="disabled" @click="moveBlind(id, 'up')">
                    <ChevronUp :size="24" :stroke-width="2.4" />
                </button>
                <button class="round" type="button" :aria-label="t('Stop', 'Parar')" :disabled="disabled" @click="stopBlind(id)">
                    <Pause :size="20" :stroke-width="2.4" fill="currentColor" />
                </button>
                <button class="round" type="button" :aria-label="t('Lower', 'Bajar')" :disabled="disabled" @click="moveBlind(id, 'down')">
                    <ChevronDown :size="24" :stroke-width="2.4" />
                </button>
            </div>
        </div>

        <button v-if="isAdmin" class="lock-btn" :class="{ on: device.locked }" type="button" :disabled="locking" @click="toggleLock">
            <component :is="device.locked ? LockOpen : Lock" :size="16" :stroke-width="2.4" />
            {{ device.locked ? t('Unlock', 'Desbloquear') : t('Lock', 'Bloquear') }}
        </button>
    </div>
</template>

<style>
/* A blind's sheet only holds the bar and three buttons, so on a wide screen it's narrow too. */
@media (min-width: 700px) { .sheet:has(.blind-control) { max-width: 340px; } }
</style>

<style scoped>
.control { --round: 56px; --gap: 14px; display: flex; flex-direction: column; align-items: center; gap: 16px; padding: 8px 0 12px; }
.stage { display: flex; align-items: center; justify-content: center; gap: 22px; }
/* As tall as the three buttons, and twice as wide as one. */
.slider { width: calc(2 * var(--round)); height: calc(3 * var(--round) + 2 * var(--gap)); border-radius: calc(var(--round) / 2); }
.buttons { display: flex; flex-direction: column; gap: var(--gap); }
.round {
    width: var(--round);
    height: var(--round);
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
/* Admins only: under the controls, a plain pill; orange to undo a lock. */
.lock-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 36px;
    padding: 0 16px;
    border-radius: 999px;
    font-size: 15px;
    font-weight: 600;
    color: var(--label);
    background: var(--fill-3);
    transition: transform 0.2s var(--spring), background 0.2s, opacity 0.2s;
}
.lock-btn:hover:not(:disabled) { background: var(--fill-2); }
.lock-btn:active:not(:disabled) { transform: scale(0.96); }
.lock-btn:disabled { opacity: 0.5; }
.lock-btn.on { color: #fff; background: var(--orange); }
</style>
