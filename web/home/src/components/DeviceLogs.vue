<script setup>
import { onMounted, onUnmounted, ref, watch } from 'vue'
import Spinner from '@/ui/Spinner.vue'
import { api } from '@/config/api'
import { onEvent } from '@/config/socket'
import { locale, t } from '@/ui/i18n.js'

// A device's MQTT traffic, newest first, live from the WebSocket.
const props = defineProps({ deviceId: { type: String, required: true } })

const logs    = ref([])
const loading = ref(false)

async function fetchLogs() {
    loading.value = true
    try { logs.value = await api.getLogs(props.deviceId) }
    catch (e) { console.error('DeviceLogs error:', e) }
    finally { loading.value = false }
}

let unsub = null
onMounted(() => {
    fetchLogs()
    unsub = onEvent('device_log', data => { if (data.device_id === props.deviceId) logs.value.unshift(data) })
})
onUnmounted(() => unsub?.())
watch(() => props.deviceId, fetchLogs)

const time = ts => new Date(ts).toLocaleTimeString(locale(), { hour: '2-digit', minute: '2-digit', second: '2-digit' })

const colours = {
    green:  ['BOOT', 'MQTT_CONNECTED', 'PING'],
    red:    ['LWT_OFFLINE'],
    orange: ['FLASH_WRITE'],
    blue:   ['UP', 'DOWN', 'STOP', 'SET_POSITION', 'GET_INFO', 'OTA', 'REBOOT', 'RESET_MEM', 'SET_PREFS', 'SET_POS', 'GET_STATE'],
}
const colour = label => Object.keys(colours).find(c => colours[c].includes(label)) ?? ''
</script>

<template>
    <div class="card logs">
        <div v-if="loading && !logs.length" class="state"><Spinner :size="20" /></div>
        <p v-else-if="!logs.length" class="state subheadline secondary">{{ t('No activity logged.', 'Sin actividad registrada.') }}</p>
        <div v-else class="list">
            <div v-for="log in logs" :key="log.id ?? `${log.timestamp}${log.event_label}`" class="log mono">
                <span class="time">{{ time(log.timestamp) }}</span>
                <span class="dir" :class="log.direction">{{ log.direction === 'rx' ? '↓' : '↑' }}</span>
                <span class="label" :class="colour(log.event_label)">{{ log.event_label }}</span>
                <span class="detail">{{ log.event_detail }}</span>
                <span class="hex">{{ log.payload_hex }}</span>
            </div>
        </div>
    </div>
</template>

<style scoped>
.state { display: grid; place-items: center; padding: 18px; }
.list { max-height: 360px; overflow-y: auto; }
.log { display: flex; align-items: baseline; gap: 12px; padding: 8px 20px; font-size: 12px; white-space: nowrap; }
.log + .log { border-top: 0.5px solid var(--separator); }
.time { color: var(--label-3); }
.dir { color: var(--label-2); }
.dir.tx { color: var(--accent); }
.label { width: 118px; flex-shrink: 0; font-weight: 600; color: var(--label-2); }
.label.green { color: var(--green); }
.label.red { color: var(--red); }
.label.orange { color: var(--orange); }
.label.blue { color: var(--blue); }
.detail { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; color: var(--label-2); }
.hex { color: var(--label-3); }
@media (max-width: 600px) { .hex { display: none; } .label { width: 96px; } }
</style>
