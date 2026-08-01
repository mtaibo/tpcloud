<script setup>

    import { ref, computed, watch, onMounted } from 'vue'
    import { Lightbulb, Blinds, Trash2, Radio, Info, ChevronRight, Crosshair, Cpu, ChevronLeft, Save, Zap } from 'lucide-vue-next'
    import { useDevices } from '@/config/devices'
    import { api } from '@/config/api'
    import { pendingDeviceId } from '@/config/sections.js'
    import MapPopup from './MapPopup.vue'
    import Btn from '@/components/Btn.vue'
    import { useDeviceDetail } from '@/config/useDeviceDetail'

    const store = useDevices()

    // --- List view ---

    const pendingDevices  = ref([])
    const selectedPending = ref(null)

    async function fetchPending() {
        try {
            pendingDevices.value = (await api.getPending()).map(m => ({ mac: m }))
        } catch (e) {
            console.error('TPHome - Error fetching pending devices:', e)
        }
    }

    function startConfig(device) { selectedPending.value = device }
    function onConfigDone()      { selectedPending.value = null; fetchPending(); store.setup() }
    function onConfigCancel()    { selectedPending.value = null }

    onMounted(fetchPending)

    const deviceGroups = computed(() => [
        { label: 'Persianas', devices: Object.entries(store.storage.blinds ?? {}).map(([id, d]) => ({ id, ...d, category: 'blinds' })) },
        { label: 'Luces',     devices: Object.entries(store.storage.lights ?? {}).map(([id, d]) => ({ id, ...d, category: 'lights' })) },
    ].filter(g => g.devices.length > 0))

    // --- Detail view ---

    const {
        selectedDevice, devicePrefs, deviceMap,
        firmwareList, selectedFirmware,
        savingPrefs, savingMap, flashingFirmware,
        focusedPrefKey, prefLabels, mapLabels,
        timeKeys, percentKeys, prefsChanged, mapChanged,
        formatPrefDisplay, startEditPref,
        open:          openDevice,
        close:         closeDevice,
        ping:          pingDevice,
        getInfo:       getDeviceInfo,
        resetPosition,
        deleteDevice,
        savePrefs,
        saveMap,
        flashFirmware,
    } = useDeviceDetail()

    watch(pendingDeviceId, (id) => {
        if (!id) return
        for (const [category, devices] of Object.entries(store.storage)) {
            if (id in devices) { openDevice({ ...devices[id], id, category }); break }
        }
        pendingDeviceId.value = null
    }, { immediate: true })

</script>

<template>

    <div class="h-full flex flex-col">

        <!-- Desktop header -->
        <div v-if="selectedDevice" class="hidden md:flex items-center gap-4 px-8 h-[72px] shrink-0">
            <Btn :pressing="false" @click="closeDevice">
                <ChevronLeft class="w-[18px] h-[18px] text-tp-text/80" />
            </Btn>
            <component :is="selectedDevice.category === 'blinds' ? Blinds : Lightbulb" class="w-5 h-5 text-white shrink-0" />
            <span class="text-lg font-semibold text-tp-text">{{ selectedDevice.name }}</span>
            <div class="w-2 h-2 rounded-full shrink-0" :class="selectedDevice.connection?.online ? 'bg-tp-on shadow-[0_0_6px_var(--color-tp-on)]' : 'bg-tp-off'"></div>
        </div>

        <!-- LIST VIEW -->
        <div v-if="!selectedDevice" class="flex-1 overflow-y-auto px-5 md:px-8 pt-28 md:pt-6 pb-32 md:pb-8 flex flex-col gap-8">

            <section v-if="pendingDevices.length > 0">
                <p class="text-base font-semibold text-white px-1 pb-3 pt-1">Sin configurar</p>
                <div class="rounded-2xl overflow-hidden bg-[#111113] device-list">
                    <div
                        v-for="device in pendingDevices"
                        :key="device.mac"
                        class="flex items-center gap-4 px-4 py-3 select-none"
                        @click="startConfig(device)"
                    >
                        <Cpu class="w-4 h-4 shrink-0 text-tp-muted" />
                        <span class="font-mono text-xs text-tp-muted flex-1">{{ device.mac }}</span>
                        <ChevronRight class="w-4 h-4 shrink-0 text-tp-muted/50" />
                    </div>
                </div>
            </section>

            <section>
                <div v-if="deviceGroups.length === 0" class="text-sm text-tp-muted/50 italic px-1">
                    No hay dispositivos configurados.
                </div>
                <div v-for="group in deviceGroups" :key="group.label" class="mb-6">
                    <p class="text-base font-semibold text-white px-1 pb-3 pt-1">{{ group.label }}</p>
                    <div class="rounded-2xl overflow-hidden bg-[#111113] device-list">
                        <div
                            v-for="device in group.devices"
                            :key="device.id"
                            class="flex items-center gap-4 px-4 py-3 select-none"
                            @click="openDevice(device)"
                        >
                            <div class="w-2 h-2 rounded-full shrink-0" :class="device.connection?.online ? 'bg-tp-on shadow-[0_0_6px_var(--color-tp-on)]' : 'bg-tp-off'"></div>
                            <span class="font-mono text-xs text-tp-muted shrink-0 hidden md:block">{{ device.id }}</span>
                            <span class="text-sm text-tp-text flex-1 truncate">{{ device.name }}</span>
                            <ChevronRight class="w-4 h-4 shrink-0 text-tp-muted/50" />
                        </div>
                    </div>
                </div>
            </section>

            <MapPopup :device="selectedPending" @done="onConfigDone" @cancel="onConfigCancel" />

        </div>

        <!-- DETAIL VIEW -->
        <div v-else class="flex-1 overflow-y-auto px-5 md:px-8 pt-24 md:pt-0 pb-32 md:pb-8 flex flex-col gap-6">

            <!-- Mobile back + title -->
            <div class="md:hidden flex items-center gap-3 pt-4">
                <Btn :pressing="false" @click="closeDevice">
                    <ChevronLeft class="w-[18px] h-[18px] text-tp-text/80" />
                </Btn>
                <span class="text-lg font-semibold text-tp-text">{{ selectedDevice.name }}</span>
            </div>

            <!-- Actions -->
            <div class="pt-2">
                <p class="text-base font-semibold text-white px-1 pb-3">Acciones</p>
                <div class="rounded-2xl overflow-hidden bg-[#111113] device-list">
                    <button @click="pingDevice()" class="action-row">
                        <Radio class="w-4 h-4 shrink-0" /><span>Ping</span>
                    </button>
                    <button @click="getDeviceInfo()" class="action-row">
                        <Info class="w-4 h-4 shrink-0" /><span>Actualizar información</span>
                    </button>
                    <button v-if="selectedDevice.category === 'blinds'" @click="resetPosition()" class="action-row">
                        <Crosshair class="w-4 h-4 shrink-0" /><span>Reiniciar posición</span>
                    </button>
                    <button @click="deleteDevice()" class="action-row action-row-danger">
                        <Trash2 class="w-4 h-4 shrink-0" /><span>Eliminar dispositivo</span>
                    </button>
                </div>
            </div>

            <!-- Prefs + Map + Firmware -->
            <div class="flex gap-6 items-start">

                <!-- Prefs editor -->
                <div v-if="Object.keys(devicePrefs).length > 0" class="flex-1 min-w-0">
                    <p class="text-base font-semibold text-white px-1 pb-3">Preferencias</p>
                    <div class="rounded-2xl overflow-hidden bg-[#111113] device-list mb-3">
                        <div
                            v-for="(value, key) in devicePrefs"
                            :key="key"
                            class="flex items-center gap-4 px-4 py-3"
                        >
                            <span class="text-sm text-tp-text flex-1">{{ prefLabels[key] ?? key }}</span>
                            <div
                                v-if="typeof value === 'boolean'"
                                class="pref-checkbox"
                                :class="devicePrefs[key] ? 'pref-checkbox--on' : ''"
                                @click="devicePrefs[key] = !devicePrefs[key]"
                            />
                            <div
                                v-else-if="(timeKeys.has(key) || percentKeys.has(key)) && focusedPrefKey !== key"
                                class="field-input field-input-display w-20"
                                @click="startEditPref(key)"
                            >{{ formatPrefDisplay(key, devicePrefs[key]) }}</div>
                            <input
                                v-else
                                v-model="devicePrefs[key]"
                                class="field-input w-20"
                                :data-pref-input="key"
                                :type="typeof value === 'number' ? 'number' : 'text'"
                                @blur="focusedPrefKey = null"
                            />
                        </div>
                    </div>
                    <Transition name="btn-fade">
                        <button v-if="prefsChanged || savingPrefs" @click="savePrefs()" class="action-primary">
                            <Save class="w-4 h-4 shrink-0" />
                            <span>{{ savingPrefs ? 'Guardando…' : 'Guardar y enviar' }}</span>
                        </button>
                    </Transition>
                </div>

                <!-- Map editor -->
                <div v-if="Object.keys(deviceMap).length > 0" class="flex-1 min-w-0">
                    <p class="text-base font-semibold text-white px-1 pb-3">Posición</p>
                    <div class="rounded-2xl overflow-hidden bg-[#111113] device-list mb-3">
                        <div
                            v-for="(value, key) in deviceMap"
                            :key="key"
                            class="flex items-center gap-4 px-4 py-3"
                        >
                            <span class="text-sm text-tp-text flex-1">{{ mapLabels[key] ?? key }}</span>
                            <input v-model="deviceMap[key]" type="number" class="field-input w-20" />
                        </div>
                    </div>
                    <Transition name="btn-fade">
                        <button v-if="mapChanged || savingMap" @click="saveMap()" class="action-primary">
                            <Save class="w-4 h-4 shrink-0" />
                            <span>{{ savingMap ? 'Guardando…' : 'Guardar y enviar' }}</span>
                        </button>
                    </Transition>
                </div>

                <!-- Firmware selector -->
                <div class="flex-1 min-w-0">
                    <p class="text-base font-semibold text-white px-1 pb-3">Firmware</p>
                    <div v-if="firmwareList.length === 0" class="text-sm text-tp-muted/50 italic px-1 mb-3">
                        No hay firmwares disponibles.
                    </div>
                    <div v-else class="rounded-2xl overflow-hidden bg-[#111113] device-list mb-3">
                        <div
                            v-for="fw in firmwareList"
                            :key="fw.version"
                            class="flex items-center gap-4 px-4 py-3 select-none"
                            @click="selectedFirmware = selectedFirmware === fw.version ? null : fw.version"
                        >
                            <div class="w-4 h-4 rounded-full border border-tp-muted/40 flex items-center justify-center shrink-0">
                                <div v-if="selectedFirmware === fw.version" class="w-2 h-2 rounded-full bg-tp-accent"></div>
                            </div>
                            <div class="flex-1 min-w-0">
                                <p class="text-sm text-tp-text truncate">{{ fw.name }}</p>
                                <p class="text-xs font-mono text-tp-muted">v{{ fw.version }} · {{ fw.chip }}</p>
                            </div>
                            <span v-if="fw.active" class="text-xs text-tp-on font-mono uppercase tracking-wider shrink-0">Activo</span>
                        </div>
                    </div>
                    <Transition name="btn-fade">
                        <button v-if="selectedFirmware || flashingFirmware" @click="flashFirmware()" class="action-primary">
                            <Zap class="w-4 h-4 shrink-0" />
                            <span>{{ flashingFirmware ? 'Subiendo…' : 'Subir firmware' }}</span>
                        </button>
                    </Transition>
                </div>

            </div>

        </div>

    </div>

</template>

<style scoped>
    .action-row {
        display: flex;
        align-items: center;
        gap: 12px;
        width: 100%;
        padding: 12px 16px;
        font-size: 0.875rem;
        color: var(--color-tp-muted);
        transition: color 0.15s ease;
        text-align: left;
    }
    .action-row:hover { color: var(--color-tp-text); }

    .field-input-display {
        display: flex;
        align-items: center;
        justify-content: flex-end;
    }

    .pref-checkbox {
        position: relative;
        width: 20px;
        height: 20px;
        background: rgba(255, 255, 255, 0.06);
        border: 0.5px solid rgba(255, 255, 255, 0.12);
        border-radius: 8px;
        transition: background 0.15s ease, border-color 0.15s ease;
        flex-shrink: 0;
    }
    .pref-checkbox--on {
        background: var(--color-tp-accent);
        border-color: var(--color-tp-accent);
    }

    .btn-fade-enter-active,
    .btn-fade-leave-active {
        transition: opacity 0.2s ease, transform 0.2s ease;
    }
    .btn-fade-enter-from,
    .btn-fade-leave-to {
        opacity: 0;
        transform: translateY(4px);
    }
</style>
