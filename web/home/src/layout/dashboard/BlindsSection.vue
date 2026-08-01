<script setup>

    import { ref, watch, nextTick } from 'vue'

    import { ChevronUp, ChevronDown, ChevronRight, Pause, Settings, Blinds } from 'lucide-vue-next'
    import { positions, handlePositions, getAnim, setPending, isPending } from '@/config/useBlindAnimations'

    import { api } from '@/config/api'
    import { useDevices } from '@/config/devices'
    import { navigateToDevice } from '@/config/sections.js'

    import Btn from '@/components/Btn.vue'
    import BlindSlider from '@/components/BlindSlider.vue'

    const store = useDevices()

    const loading       = ref({})
    const pressing      = ref({})
    const positionInput = ref({})
    const dragging      = {}

    // WS update: recalibrate anchor while moving; auto-detect direction when motor is
    // running but velocity is unknown (e.g. blind was started from another view)
    watch(() => store.blinds, (blinds) => {
        for (const [id, device] of Object.entries(blinds)) {
            if (dragging[id]) continue
            const newPos     = device.state?.position ?? 0
            const motorState = device.state?.motor_state ?? 0
            const a = getAnim(id, newPos)
            const prevRealPos = a.realPos
            a.realPos = newPos

            if (motorState === 0) {
                // Motor idle: snap everything
                setPending(id, false)
                a.velocity = 0; a.displayPos = newPos; a.handlePos = newPos
                a.moveStartPos = newPos; a.moveStartTime = Date.now()
                positions[id] = newPos; handlePositions[id] = newPos
            } else if (a.velocity !== 0) {
                // Already animating: just recalibrate the prediction anchor
                a.moveStartPos = newPos; a.moveStartTime = Date.now()
            } else if (isPending(id)) {
                // Waiting for slider set to confirm: don't touch display
            } else if (prevRealPos !== newPos) {
                // Motor moving but velocity unknown (e.g. started from another view).
                // Infer direction from position delta and start predictive animation.
                const isGoingUp = newPos > prevRealPos
                a.velocity = isGoingUp
                    ? 100 / ((device?.prefs?.up_time   ?? 3000) * 10)
                    : -(100 / ((device?.prefs?.down_time ?? 3000) * 10))
                a.displayPos = newPos; a.handlePos = newPos
                a.moveStartPos = newPos; a.moveStartTime = Date.now()
                positions[id] = newPos; handlePositions[id] = newPos
            } else {
                // First update while motor may be running, no delta yet: snap once
                a.displayPos = newPos; a.handlePos = newPos
                positions[id] = newPos; handlePositions[id] = newPos
            }
        }
    }, { immediate: true, deep: true })

    const sendCommand = async (id, cmd, val = null) => {
        loading.value[id] = true
        try { await api.sendCommand(id, cmd, val) }
        catch (e) { console.error('TPHome - BlindsSection:', e) }
        finally { loading.value[id] = false }
    }

    const pressBtn = (key, action) => {
        pressing.value[key] = false
        nextTick(() => {
            pressing.value[key] = true
            setTimeout(() => { pressing.value[key] = false }, 440)
        })
        action()
    }

    const handleMove = (id, dir) => pressBtn(`${id}-${dir}`, () => {
        const device  = store.blinds[id]
        const a       = getAnim(id, positions[id])
        const timeKey = dir === 'up' ? 'up_time' : 'down_time'
        const sign    = dir === 'up' ? 1 : -1
        setPending(id, false)
        a.velocity = sign * 100 / ((device?.prefs?.[timeKey] ?? 3000) * 10)
        a.moveStartPos = a.displayPos; a.moveStartTime = Date.now()
        sendCommand(id, dir)
    })

    const handleStop = (id) => pressBtn(`${id}-stop`, () => {
        const a = getAnim(id)
        a.velocity = 0; a.displayPos = a.realPos; a.handlePos = a.realPos
        a.moveStartPos = a.realPos
        positions[id] = a.realPos; handlePositions[id] = a.realPos
        setPending(id, false)
        sendCommand(id, 'stop')
    })

    const handleSettings = (id) => pressBtn(`${id}-cfg`, () => navigateToDevice(id))

    function sendPosition(id) {
        const raw = positionInput.value[id]
        if (raw === '' || raw == null) return
        const pos = Math.min(100, Math.max(0, Number(raw)))
        positionInput.value[id] = ''
        onBlindDragEnd(id, pos)
    }

    function onBlindDragStart(id, pos) {
        dragging[id] = true
        const a = getAnim(id, pos)
        a.velocity = 0; setPending(id, false)
        a.displayPos = pos; a.handlePos = pos
        positions[id] = pos; handlePositions[id] = pos
    }

    function onBlindDragMove(id, pos) {
        const a = getAnim(id)
        a.displayPos = pos; a.handlePos = pos
        positions[id] = pos; handlePositions[id] = pos
    }

    function onBlindDragEnd(id, pos) {
        dragging[id] = false
        const a = getAnim(id)

        a.handlePos = pos; handlePositions[id] = pos
        a.displayPos = a.realPos; a.moveStartPos = a.realPos; a.moveStartTime = Date.now()
        positions[id] = a.realPos

        const device = store.blinds[id]
        if (pos < a.realPos) {
            a.velocity = -(100 / ((device?.prefs?.down_time ?? 3000) * 10))
            setPending(id, true)
        } else if (pos > a.realPos) {
            a.velocity = 100 / ((device?.prefs?.up_time ?? 3000) * 10)
            setPending(id, true)
        } else {
            a.velocity = 0; setPending(id, false)
        }

        sendCommand(id, 'set', pos)
    }

</script>

<template>

    <div class="h-full overflow-y-auto overflow-x-hidden pt-28 pb-32 px-5 md:pt-8 md:pb-8 md:px-8">

        <!-- Empty state -->
        <div v-if="Object.keys(store.blinds).length === 0"
             class="flex flex-col items-center justify-center h-full gap-4 text-tp-muted">
            <Blinds class="w-12 h-12 opacity-20" />
            <p class="text-sm tracking-wide">No hay persianas disponibles</p>
        </div>

        <!-- Cards grid -->
        <div v-else class="flex flex-wrap justify-start md:justify-center gap-4">
            <div
                v-for="(device, id) in store.blinds"
                :key="id"
                class="blind-card transition-opacity duration-300 select-none w-[calc(50%-8px)] md:w-48 shrink-0"
                :class="{ 'opacity-50 pointer-events-none': loading[id] }"
            >
                <!-- Header -->
                <div class="flex items-start justify-between mb-4 gap-2">
                    <div class="flex items-start gap-2 flex-1 min-w-0">
                        <div class="w-2 h-2 rounded-full shrink-0 mt-[3px] transition-all duration-300"
                             :class="device.connection?.online
                                 ? 'bg-tp-on shadow-[0_0_6px_var(--color-tp-on)]'
                                 : 'bg-tp-off'">
                        </div>
                        <h3 class="text-sm font-semibold text-tp-text leading-tight max-w-[60px]">{{ device.name }}</h3>
                    </div>
                    <div class="flex items-baseline gap-0.5 shrink-0">
                        <span class="text-sm font-mono font-bold text-tp-text tabular-nums leading-none">{{ Math.round(positions[id] ?? 0) }}</span>
                        <span class="text-[9px] font-bold text-tp-accent leading-none mb-0.5">%</span>
                    </div>
                </div>

                <!-- Control area: slider left, buttons right -->
                <div class="flex gap-2 items-start">

                    <BlindSlider
                        class="flex-1 h-[180px]"
                        :fill-pos="positions[id] ?? 0"
                        :handle-pos="handlePositions[id] ?? 0"
                        @drag-start="onBlindDragStart(id, $event)"
                        @drag-move="onBlindDragMove(id, $event)"
                        @drag-end="onBlindDragEnd(id, $event)"
                    />

                    <!-- Buttons column -->
                    <div class="flex flex-col justify-between items-center h-[180px]">
                        <Btn :pressing="pressing[`${id}-up`]" @click="handleMove(id, 'up')">
                            <ChevronUp class="w-[18px] h-[18px] text-tp-text/80" />
                        </Btn>
                        <Btn :pressing="pressing[`${id}-stop`]" @click="handleStop(id)">
                            <Pause class="w-[15px] h-[15px] fill-current text-tp-text/80" />
                        </Btn>
                        <Btn :pressing="pressing[`${id}-down`]" @click="handleMove(id, 'down')">
                            <ChevronDown class="w-[18px] h-[18px] text-tp-text/80" />
                        </Btn>
                        <Btn muted :pressing="pressing[`${id}-cfg`]" @click="handleSettings(id)">
                            <Settings class="w-[15px] h-[15px] text-tp-text" />
                        </Btn>
                    </div>
                </div>

                <!-- Position input -->
                <div class="flex items-center gap-2 mt-3">
                    <input
                        v-model="positionInput[id]"
                        type="number"
                        min="0"
                        max="100"
                        placeholder="0 – 100"
                        class="pos-input"
                        @keydown.enter="sendPosition(id)"
                    />
                    <Btn @click="sendPosition(id)">
                        <ChevronRight class="w-[18px] h-[18px] text-tp-text/80" />
                    </Btn>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.blind-card {
    padding: 16px;
    border-radius: 22px;
    background: rgba(255, 255, 255, 0.065);
    backdrop-filter: blur(24px) saturate(160%);
    -webkit-backdrop-filter: blur(24px) saturate(160%);
    border: 0.5px solid rgba(255, 255, 255, 0.14);
    box-shadow:
        0 8px 32px rgba(0, 0, 0, 0.35),
        inset 0 1px 0 rgba(255, 255, 255, 0.12),
        inset 0 -1px 0 rgba(0, 0, 0, 0.15);
}

.pos-input {
    flex: 1;
    min-width: 0;
    height: 40px;
    padding: 0 10px;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.14);
    border: none;
    font-size: 0.8125rem;
    color: var(--color-tp-text);
    outline: none;
    text-align: center;
}

.pos-input::placeholder { color: var(--color-tp-muted); }
.pos-input::-webkit-outer-spin-button,
.pos-input::-webkit-inner-spin-button { -webkit-appearance: none; }
.pos-input[type=number] { -moz-appearance: textfield; }
</style>
