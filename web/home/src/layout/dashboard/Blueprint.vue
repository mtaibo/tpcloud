<script setup>

    import { ref, computed } from 'vue'
    import { useDevices } from '@/config/devices'
    import { useBlueprint } from '@/config/blueprint'

    import LightsLayer   from '@/components/blueprint/Lights.vue'
    import BlindsLayer   from '@/components/blueprint/Blinds.vue'
    import HouseLayer    from '@/components/blueprint/Map.vue'

    import BlindsControl from './BlindsControl.vue'

    const store = useDevices()
    const map = useBlueprint()

    const selectedId = ref(null)
    const selectedBlind = computed(() =>
        selectedId.value ? store.blinds[selectedId.value] : null
    )

</script>

<template>

    <div class="h-full w-full overflow-hidden min-h-0">

        <!-- Blueprint -->
        <div class="flex items-center justify-center p-5 md:p-10 h-full">
            <svg
                :viewBox="map.storage.viewBox ?? '0 0 0 0'"
                class="w-full h-auto max-w-3xl"
                xmlns="http://www.w3.org/2000/svg"
            >
                <HouseLayer />
                <LightsLayer />
                <BlindsLayer @select="selectedId = $event" />
            </svg>
        </div>

        <!-- Popup (bottom sheet on mobile, centered modal on desktop) -->
        <Teleport to="body">
            <Transition name="popup">
                <div
                    v-if="selectedBlind"
                    class="blinds-popup-overlay"
                    @click.self="selectedId = null"
                >
                    <div class="blinds-popup">
                        <BlindsControl :id="selectedId" :device="selectedBlind" @close="selectedId = null" />
                    </div>
                </div>
            </Transition>
        </Teleport>

    </div>

</template>

<style scoped>
.blinds-popup-overlay {
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
    .blinds-popup-overlay {
        align-items: center;
        padding-bottom: 0;
    }
}

.blinds-popup {
    width: 92vw;
    max-width: 24rem;
    border-radius: 32px;
    overflow: hidden;
    background: rgba(26, 26, 28, 0.88);
    backdrop-filter: blur(40px) saturate(180%);
    -webkit-backdrop-filter: blur(40px) saturate(180%);
    border: 0.5px solid rgba(255, 255, 255, 0.14);
    box-shadow:
        0 24px 80px rgba(0, 0, 0, 0.65),
        inset 0 1px 0 rgba(255, 255, 255, 0.12),
        inset 0 -1px 0 rgba(0, 0, 0, 0.2);
}

.popup-enter-active {
    transition: opacity 0.40s ease;
}
.popup-leave-active {
    transition: opacity 0.42s ease;
}
.popup-enter-from,
.popup-leave-to {
    opacity: 0;
}

.popup-enter-active .blinds-popup,
.popup-leave-active .blinds-popup {
    transform-origin: 50% 100%;
}
@media (min-width: 768px) {
    .blinds-popup {
        max-width: 400px;
    }
    .popup-enter-active .blinds-popup,
    .popup-leave-active .blinds-popup {
        transform-origin: 50% 50%;
    }
}
.popup-enter-active .blinds-popup {
    transition: transform 0.50s cubic-bezier(0.16, 1, 0.3, 1), opacity 0.28s ease;
}
.popup-leave-active .blinds-popup {
    transition: transform 0.42s cubic-bezier(0.4, 0, 0.6, 1), opacity 0.38s ease;
}
.popup-enter-from .blinds-popup,
.popup-leave-to .blinds-popup {
    transform: scale(0.08);
    opacity: 0;
}
</style>
