<script setup>
    import { ref, onMounted, onUnmounted, watch } from 'vue'
    import { api } from '@/config/api'
    import { onEvent } from '@/config/socket'

    const props = defineProps({
        deviceId: { type: String, required: true }
    })

    const logs    = ref([])
    const loading = ref(false)

    async function fetchLogs() {
        loading.value = true
        try {
            logs.value = await api.getLogs(props.deviceId)
        } catch (e) {
            console.error('DeviceLogs error:', e)
        } finally {
            loading.value = false
        }
    }

    onMounted(async () => {
        await fetchLogs()
        const unsub = onEvent('device_log', (data) => {
            if (data.device_id === props.deviceId) logs.value.unshift(data)
        })
        onUnmounted(unsub)
    })

    watch(() => props.deviceId, fetchLogs)

    function formatTime(ts) {
        return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
    }

    function labelColor(label) {
        const green  = ['BOOT', 'MQTT_CONNECTED', 'PING']
        const red    = ['LWT_OFFLINE']
        const orange = ['FLASH_WRITE']
        const blue   = ['UP', 'DOWN', 'STOP', 'SET_POSITION', 'GET_INFO', 'OTA', 'REBOOT', 'RESET_MEM', 'SET_PREFS', 'SET_POS', 'GET_STATE']
        if (green.includes(label))  return 'text-tp-on'
        if (red.includes(label))    return 'text-red-400'
        if (orange.includes(label)) return 'text-orange-400'
        if (blue.includes(label))   return 'text-tp-accent'
        return 'text-tp-muted/70'
    }
</script>

<template>
    <div class="rounded-2xl overflow-hidden bg-[#111113]">
        <div v-if="loading" class="px-4 py-3 text-xs text-tp-muted/50 italic">Cargando logs…</div>
        <div v-else-if="logs.length === 0" class="px-4 py-3 text-xs text-tp-muted/50 italic">Sin actividad registrada.</div>
        <div v-else class="max-h-80 overflow-y-auto divide-y divide-white/5">
            <div v-for="log in logs" :key="log.id" class="flex items-baseline gap-3 px-4 py-2 font-mono text-xs">
                <span class="text-tp-muted/50 shrink-0">{{ formatTime(log.timestamp) }}</span>
                <span class="shrink-0" :class="log.direction === 'rx' ? 'text-tp-muted' : 'text-tp-accent'">
                    {{ log.direction === 'rx' ? '↓' : '↑' }}
                </span>
                <span class="font-semibold shrink-0 w-28 uppercase tracking-wide" :class="labelColor(log.event_label)">
                    {{ log.event_label }}
                </span>
                <span class="text-tp-muted/70 flex-1 truncate">{{ log.event_detail }}</span>
                <span class="text-tp-muted/30 shrink-0">{{ log.payload_hex }}</span>
            </div>
        </div>
    </div>
</template>
