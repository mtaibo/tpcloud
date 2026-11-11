<script setup>
import { computed, onMounted, ref, watch } from 'vue'
import { Blinds, ChevronLeft, Cpu, Lightbulb } from 'lucide-vue-next'

import ListSection from '@/ui/ListSection.vue'
import ListRow     from '@/ui/ListRow.vue'
import Toggle      from '@/ui/Toggle.vue'
import Spinner     from '@/ui/Spinner.vue'
import MapPopup    from './MapPopup.vue'
import DeviceLogs  from '@/components/DeviceLogs.vue'

import { useDevices } from '@/config/devices'
import { api } from '@/config/api'
import { pendingDeviceId } from '@/config/sections'
import { useDeviceDetail } from '@/config/useDeviceDetail'
import { session } from '@/config/user'

// Every switch: the new ones waiting to be placed, the configured ones by kind, and a device's
// own screen (actions, preferences, where it is on the plan, firmware and its MQTT log).
const store = useDevices()
const isAdmin = computed(() => session.user?.is_admin === true) // the API checks again on every call

// ---- List ----

const pendingDevices  = ref([])
const selectedPending = ref(null)

async function fetchPending() {
    try { pendingDevices.value = (await api.getPending()).map(mac => ({ mac })) }
    catch (e) { console.error('TPHome - Error fetching pending devices:', e) }
}
function onConfigDone() { selectedPending.value = null; fetchPending(); store.setup() }

onMounted(fetchPending)

const groups = computed(() => [
    { label: 'Persianas', icon: Blinds,    devices: Object.entries(store.storage.blinds ?? {}).map(([id, d]) => ({ id, ...d, category: 'blinds' })) },
    { label: 'Luces',     icon: Lightbulb, devices: Object.entries(store.storage.lights ?? {}).map(([id, d]) => ({ id, ...d, category: 'lights' })) },
].filter(g => g.devices.length))

// ---- Detail ----

const {
    selectedDevice, devicePrefs, deviceMap,
    firmwareList, selectedFirmware,
    savingPrefs, savingMap, flashingFirmware,
    prefLabels, mapLabels, timeKeys, percentKeys,
    prefsChanged, mapChanged,
    open: openDevice, close: closeDevice,
    ping, getInfo, toggleLock, resetPosition, deleteDevice,
    savePrefs, saveMap, flashFirmware,
} = useDeviceDetail()

watch(pendingDeviceId, id => {
    if (!id) return
    for (const [category, devices] of Object.entries(store.storage)) {
        if (id in devices) { openDevice({ ...devices[id], id, category }); break }
    }
    pendingDeviceId.value = null
}, { immediate: true })

const live = computed(() => {
    const d = selectedDevice.value
    return d ? store.storage[d.category]?.[d.id] ?? d : null
})
const locked = computed(() => !!live.value?.locked)

// Times and percentages are stored in hundredths; they're edited in seconds and percent.
const scaled = key => timeKeys.value.has(key) || percentKeys.value.has(key)
const unit = key => (timeKeys.value.has(key) ? 's' : percentKeys.value.has(key) ? '%' : '')
const shown = key => (scaled(key) ? (Number(devicePrefs[key]) / 100).toString() : devicePrefs[key])
function setPref(key, raw) {
    if (scaled(key)) devicePrefs[key] = Math.round(Number(String(raw).replace(',', '.')) * 100)
    else devicePrefs[key] = raw
}
</script>

<template>
    <div class="view">
        <Transition name="push" mode="out-in">
            <!-- One device -->
            <div v-if="selectedDevice" :key="selectedDevice.id" class="page">
                <div class="bar">
                    <button class="back" type="button" @click="closeDevice">
                        <ChevronLeft :size="22" :stroke-width="2.4" />Dispositivos
                    </button>
                </div>

                <header class="device-head">
                    <span class="device-icon">
                        <component :is="selectedDevice.category === 'blinds' ? Blinds : Lightbulb" :size="30" :stroke-width="1.8" />
                    </span>
                    <h1 class="title-1">{{ live.name }}</h1>
                    <div class="badges">
                        <span class="mono secondary">{{ selectedDevice.id }}</span>
                        <span class="badge" :class="live.connection?.online ? 'green' : 'red'">{{ live.connection?.online ? 'Conectado' : 'Sin conexión' }}</span>
                        <span v-if="locked" class="badge orange">Bloqueado</span>
                    </div>
                </header>

                <div class="columns">
                    <div>
                        <ListSection header="Acciones">
                            <ListRow icon="network" tint="green" title="Ping" clickable @click="ping()" />
                            <ListRow icon="refresh" tint="blue" title="Actualizar información" clickable @click="getInfo()" />
                            <ListRow
                                v-if="isAdmin && selectedDevice.category === 'blinds'"
                                icon="lock"
                                tint="orange"
                                :title="locked ? 'Desbloquear' : 'Bloquear'"
                                clickable
                                @click="toggleLock()"
                            />
                            <ListRow v-if="selectedDevice.category === 'blinds'" icon="location" tint="indigo" title="Reiniciar posición" clickable @click="resetPosition()" />
                            <ListRow v-if="!locked" title="Eliminar dispositivo" clickable destructive @click="deleteDevice()" />
                        </ListSection>

                        <ListSection v-if="Object.keys(devicePrefs).length" header="Preferencias">
                            <ListRow v-for="(value, key) in devicePrefs" :key="key" :title="prefLabels[key] ?? key">
                                <template #trailing>
                                    <Toggle v-if="typeof value === 'boolean'" v-model="devicePrefs[key]" :label="prefLabels[key] ?? key" />
                                    <span v-else class="field">
                                        <input
                                            class="row-input trailing"
                                            type="text"
                                            :inputmode="typeof value === 'number' ? 'decimal' : 'text'"
                                            :value="shown(key)"
                                            @change="setPref(key, $event.target.value)"
                                        />
                                        <span v-if="unit(key)" class="unit">{{ unit(key) }}</span>
                                    </span>
                                </template>
                            </ListRow>
                        </ListSection>
                        <button v-if="prefsChanged || savingPrefs" class="btn btn-prominent block save" type="button" :disabled="savingPrefs" @click="savePrefs()">
                            <Spinner v-if="savingPrefs" :size="18" style="color: #fff" />
                            <span v-else>Guardar y enviar</span>
                        </button>
                    </div>

                    <div>
                        <ListSection v-if="Object.keys(deviceMap).length" header="Posición en el plano">
                            <ListRow v-for="(value, key) in deviceMap" :key="key" :title="mapLabels[key] ?? key">
                                <template #trailing>
                                    <input v-model="deviceMap[key]" class="row-input trailing" inputmode="numeric" />
                                </template>
                            </ListRow>
                        </ListSection>
                        <button v-if="mapChanged || savingMap" class="btn btn-prominent block save" type="button" :disabled="savingMap" @click="saveMap()">
                            <Spinner v-if="savingMap" :size="18" style="color: #fff" />
                            <span v-else>Guardar posición</span>
                        </button>

                        <ListSection header="Firmware" :footer="firmwareList.length ? 'Elige una versión e instálala por OTA.' : 'No hay firmwares subidos.'">
                            <ListRow
                                v-for="fw in firmwareList"
                                :key="fw.version"
                                :title="fw.name"
                                :subtitle="`v${fw.version} · ${fw.chip}`"
                                small-subtitle
                                clickable
                                @click="selectedFirmware = selectedFirmware === fw.version ? null : fw.version"
                            >
                                <template #trailing>
                                    <span v-if="fw.active" class="badge green">Activo</span>
                                    <span class="check" :class="{ on: selectedFirmware === fw.version }" />
                                </template>
                            </ListRow>
                        </ListSection>
                        <button v-if="selectedFirmware || flashingFirmware" class="btn btn-prominent block save" type="button" :disabled="flashingFirmware" @click="flashFirmware()">
                            <Spinner v-if="flashingFirmware" :size="18" style="color: #fff" />
                            <span v-else>Instalar v{{ selectedFirmware }}</span>
                        </button>
                    </div>
                </div>

                <section class="section">
                    <div class="section-header"><span>Actividad MQTT</span></div>
                    <DeviceLogs :device-id="selectedDevice.id" />
                </section>
            </div>

            <!-- The list -->
            <div v-else key="list" class="page">
                <header class="page-header">
                    <h1 class="large-title">Dispositivos</h1>
                </header>

                <ListSection v-if="pendingDevices.length" header="Sin configurar" footer="Interruptores nuevos que ya se han conectado. Elige cuál es en el plano.">
                    <ListRow
                        v-for="d in pendingDevices"
                        :key="d.mac"
                        :title="d.mac"
                        subtitle="Toca para colocarlo"
                        small-subtitle
                        clickable
                        chevron
                        @click="selectedPending = d"
                    >
                        <template #leading><span class="row-icon" style="background: var(--gray)"><Cpu :size="18" :stroke-width="2" /></span></template>
                    </ListRow>
                </ListSection>

                <div v-if="!groups.length && !pendingDevices.length" class="empty">
                    <Cpu :size="44" :stroke-width="1.5" class="empty-icon" />
                    <h3>Sin dispositivos</h3>
                    <p class="subheadline">Enciende un interruptor nuevo y aparecerá aquí para configurarlo.</p>
                </div>

                <ListSection v-for="g in groups" :key="g.label" :header="g.label">
                    <ListRow
                        v-for="d in g.devices"
                        :key="d.id"
                        :title="d.name"
                        :subtitle="d.locked ? `${d.id} · Bloqueado` : d.id"
                        small-subtitle
                        :value="d.connection?.online ? '' : 'Sin conexión'"
                        clickable
                        chevron
                        @click="openDevice(d)"
                    >
                        <template #leading>
                            <span class="row-icon dev-icon" :style="{ background: d.connection?.online ? 'var(--accent)' : 'var(--gray)' }">
                                <component :is="g.icon" :size="18" :stroke-width="2" />
                            </span>
                        </template>
                    </ListRow>
                </ListSection>
            </div>
        </Transition>

        <MapPopup :device="selectedPending" @done="onConfigDone" @cancel="selectedPending = null" />
    </div>
</template>

<style scoped>
.bar { margin: 0 0 6px; }
.back { display: inline-flex; align-items: center; gap: 2px; height: 40px; padding: 0 14px 0 8px; border-radius: 999px; font-size: 17px; color: var(--accent); background: var(--fill-3); }
.back:hover { background: var(--fill-2); }
.device-head { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 4px 0 26px; text-align: center; }
.device-icon { width: 72px; height: 72px; display: grid; place-items: center; border-radius: 22px; color: #fff; background: var(--accent); margin-bottom: 6px; }
.badges { display: flex; flex-wrap: wrap; align-items: center; justify-content: center; gap: 8px; }
.columns { display: grid; gap: 0 24px; }
@media (min-width: 1100px) { .columns { grid-template-columns: 1fr 1fr; align-items: start; } }
.save { margin: -12px 0 26px; }
.field { display: inline-flex; align-items: center; gap: 4px; max-width: 120px; }
.field .row-input { width: 80px; }
.unit { color: var(--label-2); font-size: 15px; }
.check { width: 22px; height: 22px; border-radius: 50%; box-shadow: inset 0 0 0 1.5px var(--label-3); transition: box-shadow 0.2s; }
.check.on { box-shadow: inset 0 0 0 7px var(--accent); }
</style>
