
import { ref, computed } from 'vue'
import { LayoutDashboard, Blinds, Lightbulb, Wand2, Smartphone, Cpu } from 'lucide-vue-next'

import Blueprint      from '@/layout/dashboard/Blueprint.vue'
import BlindsSection  from '@/layout/dashboard/BlindsSection.vue'
import Devices        from '@/layout/settings/Devices.vue'
import FirmwareManager from '@/layout/settings/FirmwareManager.vue'

export const dashboardSections = [
    { id: 'blueprint', name: 'Plano',     icon: LayoutDashboard, component: Blueprint },
    { id: 'lights',    name: 'Luces',     icon: Lightbulb,       component: null },
    { id: 'blinds',    name: 'Persianas', icon: Blinds,          component: BlindsSection },
    { id: 'scenes',    name: 'Escenas',   icon: Wand2,           component: null },
]

export const settingsSections = [
    { id: 'active',   name: 'Dispositivos', icon: Smartphone, component: Devices },
    { id: 'firmware', name: 'Firmware',     icon: Cpu,        component: FirmwareManager },
]

export const allSections = [...dashboardSections, ...settingsSections]

const activeSection = ref(dashboardSections[0]?.id || '')
export const pendingDeviceId = ref(null)

export function navigateToDevice(deviceId) {
    pendingDeviceId.value = deviceId
    activeSection.value = 'active'
}

export function useSections() {

    const activeComponent = computed(() =>
        allSections.find(s => s.id === activeSection.value)?.component || null
    )

    const activeSectionName = computed(() =>
        allSections.find(s => s.id === activeSection.value)?.name ?? ''
    )

    return { dashboardSections, settingsSections, allSections, activeSection, activeComponent, activeSectionName }
}
