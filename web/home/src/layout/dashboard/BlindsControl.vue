<script setup>

    import { ref, computed, watch, nextTick } from 'vue'
    import { X, ChevronUp, ChevronDown, Pause, Blinds } from 'lucide-vue-next'
    import { api } from '@/config/api'
    import BlindSlider from '@/components/BlindSlider.vue'
    import Btn from '@/components/Btn.vue'
    import { positions, handlePositions, getAnim, setPending, isPending } from '@/config/useBlindAnimations'

    const props = defineProps({
        id:     { type: String, required: true },
        device: { type: Object, required: true }
    })
    const emit = defineEmits(['close'])

    getAnim(props.id, props.device.state?.position ?? 0)

    const tempPosition = ref(Math.round(props.device.state?.position ?? 0))
    const isLoading    = ref(false)
    const pressing     = ref({})

    const smoothPos = computed(() => positions[props.id]       ?? 0)
    const handlePos = computed(() => handlePositions[props.id] ?? 0)

    watch(smoothPos, (val) => { tempPosition.value = Math.round(val) })

    const snapToReal = () => {
        const a = getAnim(props.id)
        a.velocity = 0; setPending(props.id, false)
        a.displayPos = a.realPos; a.handlePos = a.realPos
        a.moveStartPos = a.realPos; a.moveStartTime = Date.now()
        positions[props.id]       = a.realPos
        handlePositions[props.id] = a.realPos
        tempPosition.value = Math.round(a.realPos)
    }

    // WS position update: recalibrate anchor while button-moving; ignore while set-pending
    watch(() => props.device.state?.position, (newPos) => {
        if (newPos == null) return
        const a = getAnim(props.id)
        a.realPos = newPos
        if (a.velocity !== 0) {
            a.moveStartPos = newPos; a.moveStartTime = Date.now()
        } else if (!isPending(props.id)) {
            a.displayPos = newPos; a.handlePos = newPos
            positions[props.id] = newPos; handlePositions[props.id] = newPos
            tempPosition.value = Math.round(newPos)
        }
    })

    // Snap when motor reaches IDLE — clears setPending and confirms final position
    watch(() => props.device.state?.motor_state, (newState) => {
        if (newState === 0) {
            const a = getAnim(props.id)
            a.realPos = props.device.state?.position ?? a.realPos
            snapToReal()
        }
    })

    const pressBtn = (key, action) => {
        pressing.value[key] = false
        nextTick(() => {
            pressing.value[key] = true
            setTimeout(() => { pressing.value[key] = false }, 440)
        })
        action()
    }

    const sendCommand = async (command, value = null) => {
        isLoading.value = true
        try { await api.sendCommand(props.id, command, value) }
        catch (e) { console.error('TPHome - BlindsControl error:', e) }
        finally { isLoading.value = false }
    }

    const handleMove = (dir) => pressBtn(dir, () => {
        const a       = getAnim(props.id)
        const timeKey = dir === 'up' ? 'up_time' : 'down_time'
        const sign    = dir === 'up' ? 1 : -1
        setPending(props.id, false)
        a.velocity = sign * 100 / ((props.device.prefs?.[timeKey] ?? 3000) * 10)
        a.moveStartPos = a.displayPos; a.moveStartTime = Date.now()
        sendCommand(dir)
    })

    const handleStop = () => pressBtn('stop', () => { snapToReal(); sendCommand('stop') })

    let dragging = false

    function onSliderDragStart(pos) {
        dragging = true
        const a = getAnim(props.id)
        a.velocity = 0; setPending(props.id, false)
        a.displayPos = pos; a.handlePos = pos
        positions[props.id] = pos; handlePositions[props.id] = pos
        tempPosition.value = pos
    }

    function onSliderDragMove(pos) {
        if (!dragging) return
        const a = getAnim(props.id)
        a.displayPos = pos; a.handlePos = pos
        positions[props.id] = pos; handlePositions[props.id] = pos
        tempPosition.value = pos
    }

    function onSliderDragEnd(pos) {
        if (!dragging) return
        dragging = false
        const a = getAnim(props.id)

        a.handlePos = pos; handlePositions[props.id] = pos
        tempPosition.value = pos
        a.displayPos = a.realPos; a.moveStartPos = a.realPos; a.moveStartTime = Date.now()
        positions[props.id] = a.realPos

        if (pos < a.realPos) {
            a.velocity = -(100 / ((props.device.prefs?.down_time ?? 3000) * 10))
            setPending(props.id, true)
        } else if (pos > a.realPos) {
            a.velocity = 100 / ((props.device.prefs?.up_time ?? 3000) * 10)
            setPending(props.id, true)
        } else {
            a.velocity = 0; setPending(props.id, false)
        }

        sendCommand('set', pos)
    }

</script>

<template>
    <div class="flex flex-col h-full select-none" :class="{ 'opacity-80 pointer-events-none': isLoading }">

        <header class="px-5 pt-5 pb-4 flex items-center justify-between shrink-0">
            <div class="flex items-center gap-3">
                <div class="shrink-0 p-2 bg-tp-accent/10 rounded-[14px]">
                    <Blinds class="text-tp-accent w-5 h-5" />
                </div>
                <h2 class="text-base font-bold tracking-tight text-tp-text">{{ device.name }}</h2>
            </div>
            <Btn muted :pressing="pressing['close']" @click="pressBtn('close', () => emit('close'))">
                <X class="w-[15px] h-[15px] text-tp-muted" />
            </Btn>
        </header>

        <!-- MOBILE: slider + display left, buttons right -->
        <div class="md:hidden flex-1 flex items-center justify-center px-5 pb-6 gap-4">
            <div class="flex flex-col items-center gap-3 flex-1 max-w-[110px]">
                <BlindSlider
                    class="w-full aspect-[3/5]"
                    :fill-pos="smoothPos" :handle-pos="handlePos"
                    @drag-start="onSliderDragStart" @drag-move="onSliderDragMove" @drag-end="onSliderDragEnd"
                />
                <div class="flex items-baseline gap-1">
                    <span class="text-2xl font-mono font-bold text-tp-text">{{ tempPosition }}</span>
                    <span class="text-sm font-bold text-tp-accent">%</span>
                </div>
            </div>
            <div class="flex flex-col gap-3">
                <Btn :pressing="pressing['up']"   @click="handleMove('up')">
                    <ChevronUp class="w-[18px] h-[18px] text-tp-text/80" />
                </Btn>
                <Btn :pressing="pressing['stop']" @click="handleStop">
                    <Pause class="w-[15px] h-[15px] fill-current text-tp-text/80" />
                </Btn>
                <Btn :pressing="pressing['down']" @click="handleMove('down')">
                    <ChevronDown class="w-[18px] h-[18px] text-tp-text/80" />
                </Btn>
            </div>
        </div>

        <!-- DESKTOP: slider + display + buttons stacked -->
        <div class="hidden md:flex flex-1 flex-col items-center justify-center p-6 gap-8">
            <div class="flex flex-col items-center gap-4">
                <BlindSlider
                    class="w-40 h-64"
                    :fill-pos="smoothPos" :handle-pos="handlePos"
                    @drag-start="onSliderDragStart" @drag-move="onSliderDragMove" @drag-end="onSliderDragEnd"
                />
                <div class="flex items-baseline gap-1">
                    <span class="text-3xl font-mono font-bold text-tp-text">{{ tempPosition }}</span>
                    <span class="text-sm font-bold text-tp-accent">%</span>
                </div>
            </div>
            <div class="flex gap-3">
                <Btn :pressing="pressing['up']"   @click="handleMove('up')">
                    <ChevronUp class="w-[18px] h-[18px] text-tp-text/80" />
                </Btn>
                <Btn :pressing="pressing['stop']" @click="handleStop">
                    <Pause class="w-[15px] h-[15px] fill-current text-tp-text/80" />
                </Btn>
                <Btn :pressing="pressing['down']" @click="handleMove('down')">
                    <ChevronDown class="w-[18px] h-[18px] text-tp-text/80" />
                </Btn>
            </div>
        </div>

    </div>
</template>
