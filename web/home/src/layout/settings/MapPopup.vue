<script setup>
import { computed, ref, watch } from 'vue'

import Sheet from '@/ui/Sheet.vue'
import Plan from '@/components/blueprint/Plan.vue'

import { useBlueprint } from '@/config/blueprint'
import { useDevices } from '@/config/devices'
import { api } from '@/config/api'
import { toast } from '@/ui/dialog.js'
import { t } from '@/ui/i18n.js'

// Placing a new switch: tap which blind or light on the plan it is, then ✓ sends it its ID and
// that device's preferences.
const props = defineProps({ device: { type: Object, default: null } })
const emit = defineEmits(['done', 'cancel'])

const map   = useBlueprint()
const store = useDevices()

const pickedId = ref(null)
const busy     = ref(false)

const open = computed({
    get: () => !!props.device,
    set: v => { if (!v) emit('cancel') },
})

const categoryOf = id => Object.keys(store.storage).find(cat => id in store.storage[cat])
const picked = computed(() => {
    const category = pickedId.value && categoryOf(pickedId.value)
    return category ? { id: pickedId.value, name: store.storage[category][pickedId.value]?.name, category } : null
})

async function confirm() {
    if (!picked.value) return
    busy.value = true
    try {
        await api.configurePending(props.device.mac, picked.value.id, store.storage[picked.value.category][picked.value.id].prefs)
        toast(t(`${props.device.mac} is now ${picked.value.name}`, `${props.device.mac} es ahora ${picked.value.name}`))
        emit('done')
    } catch (e) {
        console.error('TPHome - Config error:', e)
        toast(t('Couldn’t set it up', 'No se pudo configurar'), 'warning')
    } finally {
        busy.value = false
    }
}

watch(() => props.device, async d => {
    pickedId.value = null
    if (!d) return
    const loads = []
    if (!Object.keys(store.storage).length) loads.push(store.setup())
    if (map.unconfigured) loads.push(map.setup())
    await Promise.all(loads)
}, { immediate: true })
</script>

<template>
    <Sheet
        v-model:open="open"
        :title="t('Place switch', 'Colocar interruptor')"
        confirmable
        :can-confirm="!!picked"
        :busy="busy"
        @confirm="confirm"
    >
        <p class="subheadline secondary intro">
            <span class="mono">{{ device?.mac }}</span><br />
            {{ picked ? t(`It will be ${picked.name} (${picked.id}).`, `Será ${picked.name} (${picked.id}).`) : t('Tap the blind or light it controls on the map.', 'Toca en el plano la persiana o la luz que controla.') }}
        </p>
        <div class="plan-card">
            <Plan mode="config" :picked="pickedId" @pick="pickedId = $event" />
        </div>
    </Sheet>
</template>

<style scoped>
.intro { text-align: center; margin-bottom: 16px; }
.plan-card { padding: 16px; border-radius: var(--radius-card); background: var(--bg); }
</style>
