<script setup>
import { onMounted, ref } from 'vue'

import AlertHost   from '@/ui/AlertHost.vue'
import Spinner     from '@/ui/Spinner.vue'
import ContextMenu from '@/components/ContextMenu.vue'
import Sidebar     from '@/layout/Sidebar.vue'
import TabBar      from '@/layout/TabBar.vue'

import { useDevices }   from '@/config/devices'
import { useBlueprint } from '@/config/blueprint'
import { useSections }  from '@/config/sections'
import { syncBlinds }   from '@/config/blindControl'
import { loadUser }     from '@/config/user'

const devices = useDevices()
const map = useBlueprint()
const { activeSection, activeComponent } = useSections()
const ready = ref(false)

syncBlinds()

onMounted(async () => {
    try {
        await loadUser()
    } catch {
        window.location.href = `/auth/signin?next=${encodeURIComponent(window.location.pathname)}`
        return
    }
    ready.value = true
    await Promise.all([devices.setup(), map.setup()])
})
</script>

<template>
    <div v-if="ready" class="app">
        <Sidebar class="desktop-only" />
        <main class="main">
            <Transition name="view" mode="out-in">
                <component :is="activeComponent" :key="activeSection" />
            </Transition>
        </main>
        <TabBar />
    </div>
    <div v-else class="splash"><Spinner :size="26" /></div>

    <ContextMenu />
    <AlertHost />
</template>

<style>
/* Casa's colour, as on the dashboard's tile. */
:root { --accent: var(--orange); }
html, body, #app { height: 100%; }
/* One screen like the other TPCloud apps: views scroll inside, with no bar and no bounce. */
html, body { overflow: hidden; overscroll-behavior: none; }
* { scrollbar-width: none; user-select: none; -webkit-user-select: none; -webkit-touch-callout: none; }
*::-webkit-scrollbar { display: none; }
input, textarea, .mono { user-select: text; -webkit-user-select: text; }

/* A section's own scroll area, under the large title. */
.view { flex: 1; min-width: 0; height: 100dvh; overflow-y: auto; overscroll-behavior: none; padding-top: calc(24px + env(safe-area-inset-top)); }
.view .page { max-width: 980px; }
@media (max-width: 767px) { .view .page { padding-bottom: calc(110px + env(safe-area-inset-bottom)); } }
</style>

<style scoped>
.app { display: flex; height: 100dvh; background: var(--bg); }
.main { flex: 1; min-width: 0; display: flex; }
.splash { height: 100dvh; display: grid; place-items: center; }
@media (max-width: 767px) { .desktop-only { display: none; } }

.view-enter-active { transition: opacity 0.22s var(--ease), transform 0.28s var(--ease); }
.view-leave-active { transition: opacity 0.12s ease-in; }
.view-enter-from { opacity: 0; transform: translateY(8px); }
.view-leave-to { opacity: 0; }
</style>
