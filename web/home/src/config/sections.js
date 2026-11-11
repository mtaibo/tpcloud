import { ref, computed, markRaw } from 'vue'
import { Blinds, Cpu, HardDriveDownload, Map } from 'lucide-vue-next'

import Blueprint       from '@/layout/dashboard/Blueprint.vue'
import BlindsSection   from '@/layout/dashboard/BlindsSection.vue'
import Devices         from '@/layout/settings/Devices.vue'
import FirmwareManager from '@/layout/settings/FirmwareManager.vue'

// What the sidebar (and the phone's tab bar) can open. Lights and scenes come back once their
// firmware works; until then they'd only be empty screens.
export const homeSections = [
    { id: 'blueprint', name: 'Plano',     icon: markRaw(Map),    component: markRaw(Blueprint) },
    { id: 'blinds',    name: 'Persianas', icon: markRaw(Blinds), component: markRaw(BlindsSection) },
]

export const settingsSections = [
    { id: 'devices',  name: 'Dispositivos', icon: markRaw(Cpu),               component: markRaw(Devices) },
    { id: 'firmware', name: 'Firmware',     icon: markRaw(HardDriveDownload), component: markRaw(FirmwareManager) },
]

export const allSections = [...homeSections, ...settingsSections]

const activeSection = ref(homeSections[0].id)
export const pendingDeviceId = ref(null)

// Opens a device's settings from anywhere (a blind's sheet, the plan…).
export function navigateToDevice(deviceId) {
    pendingDeviceId.value = deviceId
    activeSection.value = 'devices'
}

export function useSections() {
    const activeComponent = computed(() => allSections.find(s => s.id === activeSection.value)?.component ?? null)
    return { homeSections, settingsSections, allSections, activeSection, activeComponent }
}
