<script setup>
import { onMounted, ref } from 'vue'
import { Camera, Cloud, Folder, Images, Smartphone } from 'lucide-vue-next'
import AlertHost from './ui/AlertHost.vue'
import Spinner from './ui/Spinner.vue'
import ContextMenu from './components/ContextMenu.vue'
import Gallery from './components/Gallery.vue'

// The gallery as a section of its own, next to the store it keeps its photos in (and that
// serves its API): the photos backed up from the iPhone, and in time albums too.
const user = ref(null)
const filter = ref('all')

const sections = [
  { value: 'all', label: 'Biblioteca', icon: Images },
  { value: 'camera', label: 'Cámara', icon: Camera },
  { value: 'screenshots', label: 'Capturas', icon: Smartphone },
]

onMounted(async () => {
  try {
    const res = await fetch('/auth/passkey/me')
    if (!res.ok) throw new Error()
    user.value = await res.json()
  } catch {
    window.location.href = `/auth/signin?next=${encodeURIComponent(window.location.pathname)}`
  }
})
</script>

<template>
  <div v-if="user" class="app">
    <aside class="sidebar">
      <a href="/" class="brand" title="Volver a TPCloud">
        <span class="brand-icon"><Images :size="15" :stroke-width="2.4" /></span>
        <span>Galería</span>
      </a>
      <nav class="nav">
        <div class="group-head"><span>Fotos</span></div>
        <button
          v-for="s in sections"
          :key="s.value"
          type="button"
          class="item"
          :class="{ active: filter === s.value }"
          @click="filter = s.value"
        >
          <component :is="s.icon" :size="18" :stroke-width="1.9" class="item-icon" />
          <span class="item-label">{{ s.label }}</span>
        </button>
      </nav>
      <div class="foot">
        <a href="/store/" class="item"><Folder :size="18" :stroke-width="1.9" class="item-icon" /><span class="item-label">Archivos</span></a>
        <a href="/" class="item"><Cloud :size="18" :stroke-width="1.9" class="item-icon" /><span class="item-label">TPCloud</span></a>
      </div>
    </aside>
    <main class="main"><Gallery v-model:filter="filter" /></main>
  </div>
  <div v-else class="splash"><Spinner :size="26" /></div>

  <ContextMenu />
  <AlertHost />
</template>

<style>
html, body, #app { height: 100%; }
body { overflow: hidden; }
* { user-select: none; -webkit-user-select: none; -webkit-touch-callout: none; }
</style>

<style scoped>
.app { display: flex; height: 100dvh; background: var(--bg); }
.main { flex: 1; min-width: 0; display: flex; }
.splash { height: 100dvh; display: grid; place-items: center; }

/* Floating sidebar like the store's (macOS 26). */
.sidebar {
  width: 248px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  margin: 8px 0 8px 8px;
  padding: 10px 10px 8px;
  border-radius: 20px;
  background: color-mix(in srgb, var(--cell) 70%, transparent);
  backdrop-filter: blur(30px) saturate(180%);
  -webkit-backdrop-filter: blur(30px) saturate(180%);
  box-shadow: 0 0 0 0.5px var(--separator), 0 8px 30px rgba(0, 0, 0, 0.06);
}
@media (prefers-color-scheme: dark) {
  .sidebar { background: rgba(44, 44, 46, 0.55); box-shadow: 0 0 0 0.5px rgba(255, 255, 255, 0.08); }
}
.brand { display: flex; align-items: center; gap: 8px; padding: 6px 10px 10px; font-size: 14px; font-weight: 700; letter-spacing: -0.2px; color: var(--label); }
.brand-icon { width: 24px; height: 24px; display: grid; place-items: center; border-radius: 7px; color: #fff; background: linear-gradient(180deg, #ff5f8f, #ff2d55); }
.nav { flex: 1; overflow-y: auto; }
.group-head { padding: 14px 10px 4px; font-size: 11px; font-weight: 700; color: var(--label-3); }
.item {
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  height: 32px;
  padding: 0 10px;
  border-radius: 9px;
  font-size: 13.5px;
  letter-spacing: -0.1px;
  text-align: left;
  color: var(--label);
  cursor: default;
}
.item:hover { background: var(--fill-4); }
.item.active { background: var(--fill-2); font-weight: 600; }
.item-icon { flex-shrink: 0; color: var(--accent); }
.item-label { flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.foot { padding-top: 8px; border-top: 0.5px solid var(--separator); }

@media (max-width: 767px) {
  .sidebar { display: none; }
}
</style>
