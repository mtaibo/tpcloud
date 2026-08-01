<script setup>

    import { ref, computed, watch } from 'vue'
    import { X, ChevronRight, Cpu, Blinds, Lightbulb } from 'lucide-vue-next'
    import { useBlueprint } from '@/config/blueprint'
    import { useDevices } from '@/config/devices'
    import { api } from '@/config/api'
    import HouseLayer  from '@/components/blueprint/Map.vue'
    import LightsLayer from '@/components/blueprint/Lights.vue'
    import BlindsLayer from '@/components/blueprint/Blinds.vue'
    import Btn from '@/components/Btn.vue'

    const props = defineProps({
        device: { type: Object, default: null }
    })
    const emit = defineEmits(['done', 'cancel'])

    const map   = useBlueprint()
    const store = useDevices()

    const pickedId = ref(null)
    const picking  = ref(false)

    const picked = computed(() => {
        if (!pickedId.value) return null
        for (const [category, devices] of Object.entries(store.storage)) {
            if (pickedId.value in devices) return { name: devices[pickedId.value]?.name, category }
        }
        return null
    })

    function onPick(id) { pickedId.value = id }

    async function confirm() {
        if (!pickedId.value) return
        picking.value = true
        try {
            const category = Object.keys(store.storage).find(cat => pickedId.value in store.storage[cat])
            if (!category) return
            await api.configurePending(props.device.mac, pickedId.value, store.storage[category][pickedId.value].prefs)
            emit('done')
        } catch (e) {
            console.error('TPHome - Config error:', e)
        } finally {
            picking.value = false
        }
    }

    watch(() => props.device, async (d) => {
        if (!d) { pickedId.value = null; return }
        const loads = []
        if (Object.keys(store.storage).length === 0) loads.push(store.setup())
        if (map.unconfigured)                        loads.push(map.setup())
        await Promise.all(loads)
    }, { immediate: true })

</script>

<template>

    <Teleport to="body">
        <Transition name="popup">
            <div v-if="device" class="map-popup-overlay" @click.self="emit('cancel')">
                <div class="map-popup">

                    <!-- Header -->
                    <header class="px-5 pt-5 pb-4 flex items-center gap-3 shrink-0">
                        <Cpu class="text-white w-5 h-5 shrink-0" />
                        <span class="text-base font-bold tracking-tight text-tp-text font-mono shrink-0">{{ device.mac }}</span>
                        <template v-if="picked">
                            <component :is="picked.category === 'blinds' ? Blinds : Lightbulb" class="w-4 h-4 text-tp-muted shrink-0" />
                            <span class="flex-1 text-sm text-tp-muted truncate text-right">{{ picked.name }}</span>
                        </template>
                        <div v-if="!picked" class="flex-1" />
                        <Transition name="btn-fade">
                            <Btn v-if="picked" muted :class="{ 'opacity-50 pointer-events-none': picking }" @click="confirm">
                                <ChevronRight class="w-[15px] h-[15px] text-tp-muted" />
                            </Btn>
                        </Transition>
                        <Btn muted @click="emit('cancel')">
                            <X class="w-[15px] h-[15px] text-tp-muted" />
                        </Btn>
                    </header>

                    <!-- Blueprint -->
                    <div class="flex items-center justify-center px-5 pb-5 md:px-8 md:pb-8">
                        <svg
                            :viewBox="map.storage.viewBox ?? '0 0 0 0'"
                            class="w-full h-auto"
                            xmlns="http://www.w3.org/2000/svg"
                        >
                            <HouseLayer />
                            <LightsLayer mode="config" @pick="onPick" />
                            <BlindsLayer mode="config" @pick="onPick" />
                        </svg>
                    </div>

                </div>
            </div>
        </Transition>
    </Teleport>

</template>

<style scoped>
.map-popup-overlay {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding-bottom: 2rem;
    background: rgba(0, 0, 0, 0.32);
    backdrop-filter: blur(7px);
    -webkit-backdrop-filter: blur(7px);
}

@media (min-width: 768px) {
    .map-popup-overlay {
        align-items: center;
        padding-bottom: 0;
    }
}

.map-popup {
    width: 92vw;
    max-width: 48rem;
    max-height: 90vh;
    overflow-y: auto;
    border-radius: 32px;
    background: rgba(26, 26, 28, 0.88);
    backdrop-filter: blur(40px) saturate(180%);
    -webkit-backdrop-filter: blur(40px) saturate(180%);
    border: 0.5px solid rgba(255, 255, 255, 0.14);
    box-shadow:
        0 24px 80px rgba(0, 0, 0, 0.65),
        inset 0 1px 0 rgba(255, 255, 255, 0.12),
        inset 0 -1px 0 rgba(0, 0, 0, 0.2);
}

.popup-enter-active { transition: opacity 0.40s ease; }
.popup-leave-active { transition: opacity 0.42s ease; }
.popup-enter-from,
.popup-leave-to     { opacity: 0; }

.popup-enter-active .map-popup,
.popup-leave-active .map-popup { transform-origin: 50% 100%; }
@media (min-width: 768px) {
    .popup-enter-active .map-popup,
    .popup-leave-active .map-popup { transform-origin: 50% 50%; }
}
.popup-enter-active .map-popup {
    transition: transform 0.50s cubic-bezier(0.16, 1, 0.3, 1), opacity 0.28s ease;
}
.popup-leave-active .map-popup {
    transition: transform 0.42s cubic-bezier(0.4, 0, 0.6, 1), opacity 0.38s ease;
}
.popup-enter-from .map-popup,
.popup-leave-to   .map-popup { transform: scale(0.08); opacity: 0; }

.btn-fade-enter-active,
.btn-fade-leave-active { transition: opacity 0.2s ease, transform 0.2s ease; }
.btn-fade-enter-from,
.btn-fade-leave-to     { opacity: 0; transform: translateY(4px); }
</style>
