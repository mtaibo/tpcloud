<script setup>
import { computed, onMounted, ref } from 'vue'
import { BookImage, Camera, ChevronRight, Cloud, Folder, Heart, Images, LogOut, Smartphone, User } from 'lucide-vue-next'
import AlertHost from './ui/AlertHost.vue'
import Spinner from './ui/Spinner.vue'
import ContextMenu from './components/ContextMenu.vue'
import Gallery from './components/Gallery.vue'
import { syncLang, t } from './ui/i18n.js'
import { openMenu } from './menu.js'
import { formatSize } from './utils/formatSize.js'

// The gallery as a section of its own, next to the store it keeps its photos in (and that
// serves its API): the photos backed up from the iPhone, and in time albums too.
const user = ref(null)
const section = ref('all') // all | camera | screenshots | album:<id>
const albums = ref([])
const disk = ref(null)

const initials = computed(() => (user.value?.display_name || '?').split(/\s+/).slice(0, 2).map(w => w[0]?.toUpperCase()).join(''))
// Hidden albums (hidden from the iPhone app) stay out of the sidebar.
const visibleAlbums = computed(() => albums.value.filter(a => !a.hidden))
const usedPct = computed(() => (disk.value ? Math.round((disk.value.used / (disk.value.used + disk.value.free)) * 100) : 0))

const sections = computed(() => [
  { value: 'all', label: t('Library', 'Biblioteca'), icon: Images },
  { value: 'camera', label: t('Camera', 'Cámara'), icon: Camera },
  { value: 'screenshots', label: t('Screenshots', 'Capturas'), icon: Smartphone },
])

onMounted(async () => {
  try {
    const res = await fetch('/auth/passkey/me')
    if (!res.ok) throw new Error()
    user.value = await res.json()
    syncLang(user.value)
  } catch {
    window.location.href = `/auth/signin?next=${encodeURIComponent(window.location.pathname)}`
    return
  }
  loadAlbums()
  loadDisk()
})

// The same storage meter as Archivos': the photos live on the same disk.
async function loadDisk() {
  try {
    const res = await fetch('/store/api/files/disk')
    if (res.ok) disk.value = await res.json()
  } catch { /* the meter just stays hidden */ }
}

function userMenu(e) {
  openMenu(e.currentTarget, [
    { label: t('Your account', 'Tu cuenta'), icon: User, action: () => { window.location.href = '/accounts' } },
    { label: t('Back to TPCloud', 'Volver a TPCloud'), icon: Cloud, action: () => { window.location.href = '/' } },
    { separator: true },
    { label: t('Log out', 'Cerrar sesión'), icon: LogOut, danger: true, action: logout },
  ])
}

async function logout() {
  await fetch('/auth/passkey/logout', { method: 'POST' })
  window.location.href = '/login'
}

// Favoritos and, later, albums of your own, each with how many photos it holds.
async function loadAlbums() {
  try {
    const res = await fetch('/store/api/albums')
    if (res.ok) albums.value = await res.json()
  } catch { /* the section just stays empty */ }
}
</script>

<template>
  <div v-if="user" class="app">
    <aside class="sidebar">
      <a href="/" class="brand" :title="t('Back to TPCloud', 'Volver a TPCloud')">
        <span class="brand-icon"><Images :size="15" :stroke-width="2.4" /></span>
        <span>{{ t('Photos', 'Galería') }}</span>
      </a>
      <nav class="nav">
        <div class="group-head"><span>{{ t('Photos', 'Fotos') }}</span></div>
        <button
          v-for="s in sections"
          :key="s.value"
          type="button"
          class="item"
          :class="{ active: section === s.value }"
          @click="section = s.value"
        >
          <component :is="s.icon" :size="18" :stroke-width="1.9" class="item-icon" />
          <span class="item-label">{{ s.label }}</span>
        </button>

        <template v-if="visibleAlbums.length">
          <div class="group-head"><span>{{ t('Albums', 'Álbumes') }}</span></div>
          <button
            v-for="a in visibleAlbums"
            :key="a.id"
            type="button"
            class="item"
            :class="{ active: section === `album:${a.id}` }"
            @click="section = `album:${a.id}`"
          >
            <component :is="a.kind === 'favorites' ? Heart : BookImage" :size="18" :stroke-width="1.9" class="item-icon" />
            <span class="item-label">{{ a.kind === 'favorites' ? t('Favourites', 'Favoritos') : a.name }}</span>
            <span class="item-count">{{ a.count }}</span>
          </button>
        </template>

        <div class="group-head"><span>{{ t('Locations', 'Ubicaciones') }}</span></div>
        <a href="/store/" class="item" :title="t('Open Files', 'Abrir Archivos')">
          <Folder :size="18" :stroke-width="1.9" class="item-icon" />
          <span class="item-label">{{ t('Files', 'Archivos') }}</span>
        </a>
      </nav>

      <div v-if="disk" class="storage">
        <div class="meter"><i :style="{ width: `${usedPct}%` }" :class="{ full: usedPct > 90 }" /></div>
        <p>{{ t(`${formatSize(disk.free)} available of ${formatSize(disk.used + disk.free)}`, `${formatSize(disk.free)} disponibles de ${formatSize(disk.used + disk.free)}`) }}</p>
      </div>

      <button type="button" class="user" @click="userMenu">
        <span class="avatar" style="width: 30px; height: 30px; font-size: 12px">{{ initials }}</span>
        <span class="user-name">{{ user.display_name }}</span>
        <ChevronRight :size="14" :stroke-width="2.4" class="user-chevron" />
      </button>
    </aside>
    <main class="main"><Gallery v-model:section="section" :albums="albums" @albums-changed="loadAlbums" /></main>
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
.item-count { flex-shrink: 0; font-size: 12px; color: var(--label-3); font-variant-numeric: tabular-nums; }
.storage { padding: 10px 10px 8px; border-top: 0.5px solid var(--separator); }
.meter { height: 6px; border-radius: 3px; background: var(--fill-3); overflow: hidden; }
.meter i { display: block; height: 100%; border-radius: 3px; background: var(--accent); }
.meter i.full { background: var(--orange); }
.storage p { margin-top: 6px; font-size: 11.5px; color: var(--label-2); }
.user { display: flex; align-items: center; gap: 9px; padding: 6px 8px; border-radius: 12px; text-align: left; color: var(--label); }
.user:hover { background: var(--fill-4); }
.user-name { flex: 1; min-width: 0; font-size: 13.5px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.user-chevron { color: var(--label-3); transform: rotate(-90deg); }

@media (max-width: 767px) {
  .sidebar { display: none; }
}
</style>
