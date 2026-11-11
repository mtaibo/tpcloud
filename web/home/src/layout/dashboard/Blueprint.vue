<script setup>
import { computed, ref } from 'vue'
import { Map } from 'lucide-vue-next'

import Sheet from '@/ui/Sheet.vue'
import Plan from '@/components/blueprint/Plan.vue'
import BlindsControl from './BlindsControl.vue'

import { useDevices } from '@/config/devices'
import { useBlueprint } from '@/config/blueprint'

// The house from above: every blind where it really is, drawn as closed as it is. Tap one to move it.
const store = useDevices()
const map = useBlueprint()

const selectedId = ref(null)
const open = computed({
    get: () => !!selectedId.value && !!store.blinds[selectedId.value],
    set: v => { if (!v) selectedId.value = null },
})

</script>

<template>
    <div class="view">
        <div class="page plan-page">
            <header class="page-header">
                <h1 class="large-title">Plano</h1>
            </header>

            <div v-if="map.storage.viewBox" class="plan-card">
                <Plan @select="selectedId = $event" />
            </div>
            <div v-else class="empty">
                <Map :size="44" :stroke-width="1.5" class="empty-icon" />
                <h3>Sin plano</h3>
                <p class="subheadline">La API todavía no tiene el plano de la casa.</p>
            </div>
        </div>

        <Sheet v-model:open="open" :title="selectedId ? store.blinds[selectedId]?.name : ''">
            <BlindsControl v-if="selectedId" :id="selectedId" @close="selectedId = null" />
        </Sheet>
    </div>
</template>

<style scoped>
/* The plan takes the rest of the screen under the title and sits in its middle. */
.plan-page { min-height: calc(100dvh - 24px - env(safe-area-inset-top)); display: flex; flex-direction: column; }
.plan-card { flex: 1; display: flex; align-items: center; justify-content: center; padding: 8px 0 40px; }
.plan-card :deep(.plan) { max-height: calc(100dvh - 190px); width: auto; max-width: 100%; }
@media (max-width: 767px) {
    .plan-page { min-height: calc(100dvh - 24px - env(safe-area-inset-top) - 110px - env(safe-area-inset-bottom)); padding-bottom: 0 !important; }
    .plan-card :deep(.plan) { width: 100%; max-height: none; }
}
</style>
