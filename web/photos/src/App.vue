<script setup>
import { computed, onMounted, ref } from 'vue'
import { Camera, ChevronRight, Cloud, Folder, Images, LogOut, MoreHorizontal, Plus, Smartphone, User } from 'lucide-vue-next'
import AlertHost from '@ui/AlertHost.vue'
import Spinner from '@ui/Spinner.vue'
import ContextMenu from './components/ContextMenu.vue'
import Gallery from './components/Gallery.vue'
import { syncLang, t } from '@ui/i18n.js'
import { openMenu } from './menu.js'
import { formatSize } from './utils/formatSize.js'
import { albumIcon, albumMenuItems, albumName, createAlbum } from './albums.js'

// The gallery as a section of its own, next to the store it keeps its photos in (and that
// serves its API): the photos backed up from the iPhone, and in time albums too.
const user = ref(null)
const section = ref('all') // all | camera | screenshots | album:<id>
const albums = ref([])
const disk = ref(null)
const libraryVersion = ref(0) // bumped when the library's photos may have changed (an album keeping them out)
const hiddenOpen = ref(readHiddenOpen())

const initials = computed(() => (user.value?.display_name || '?').split(/\s+/).slice(0, 2).map(w => w[0]?.toUpperCase()).join(''))
// Favoritos and your own albums, then the hidden ones in their own folding section, as in the app.
const visibleAlbums = computed(() => albums.value.filter(a => !a.hidden))
const hiddenAlbums = computed(() => albums.value.filter(a => a.hidden))

function readHiddenOpen() {
  try { return localStorage.getItem('tpphotos-hidden-open') === '1' } catch { return false }
}
function toggleHidden() {
  hiddenOpen.value = !hiddenOpen.value
  try { localStorage.setItem('tpphotos-hidden-open', hiddenOpen.value ? '1' : '0') } catch {}
}

async function onAlbumsChanged({ library } = {}) {
  await loadAlbums()
  if (library) libraryVersion.value++
}

function albumMenu(a, e) {
  const items = albumMenuItems(a, {
    changed: onAlbumsChanged,
    deleted: () => { if (section.value === `album:${a.id}`) section.value = 'all' },
  })
  if (items.length) openMenu(e.currentTarget instanceof Element && e.type === 'click' ? e.currentTarget : e, items, a.name)
}

async function newAlbum() {
  const a = await createAlbum()
  if (!a) return
  await loadAlbums()
  section.value = `album:${a.id}`
}
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
        <a href="/store/" class="item">
          <Folder :size="18" :stroke-width="1.9" class="item-icon" />
          <span class="item-label">{{ t('Files', 'Archivos') }}</span>
        </a>

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

        <div class="group-head">
          <span>{{ t('Albums', 'Álbumes') }}</span>
          <button type="button" class="head-btn" :title="t('New album', 'Nuevo álbum')" @click="newAlbum"><Plus :size="14" :stroke-width="2.4" /></button>
        </div>
        <div v-for="a in visibleAlbums" :key="a.id" class="item-wrap" :class="{ 'has-more': a.kind !== 'favorites' }">
          <button
            type="button"
            class="item"
            :class="{ active: section === `album:${a.id}` }"
            @click="section = `album:${a.id}`"
            @contextmenu.prevent="albumMenu(a, $event)"
          >
            <component :is="albumIcon(a)" :size="18" :stroke-width="1.9" class="item-icon" />
            <span class="item-label">{{ albumName(a) }}</span>
            <span class="item-count">{{ a.count }}</span>
          </button>
          <button v-if="a.kind !== 'favorites'" type="button" class="more" :title="t('Album options', 'Opciones del álbum')" @click.stop="albumMenu(a, $event)"><MoreHorizontal :size="15" :stroke-width="2.2" /></button>
        </div>

        <template v-if="hiddenAlbums.length">
          <div class="group-head fold" @click="toggleHidden">
            <span>{{ t('Hidden', 'Ocultos') }}</span>
            <ChevronRight :size="13" :stroke-width="2.6" class="head-chevron" :class="{ open: hiddenOpen }" />
          </div>
          <template v-if="hiddenOpen">
            <div v-for="a in hiddenAlbums" :key="a.id" class="item-wrap has-more">
              <button
                type="button"
                class="item"
                :class="{ active: section === `album:${a.id}` }"
                @click="section = `album:${a.id}`"
                @contextmenu.prevent="albumMenu(a, $event)"
              >
                <component :is="albumIcon(a)" :size="18" :stroke-width="1.9" class="item-icon" />
                <span class="item-label">{{ albumName(a) }}</span>
                <span class="item-count">{{ a.count }}</span>
              </button>
              <button type="button" class="more" :title="t('Album options', 'Opciones del álbum')" @click.stop="albumMenu(a, $event)"><MoreHorizontal :size="15" :stroke-width="2.2" /></button>
            </div>
          </template>
        </template>
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
    <main class="main"><Gallery v-model:section="section" :albums="albums" :library-version="libraryVersion" @albums-changed="onAlbumsChanged" /></main>
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
.group-head { display: flex; align-items: center; gap: 4px; padding: 14px 10px 4px; font-size: 11px; font-weight: 700; color: var(--label-3); }
.group-head > span { flex: 1; }
.group-head.fold { cursor: default; }
.head-btn { width: 20px; height: 20px; display: grid; place-items: center; border-radius: 6px; color: var(--label-2); }
.head-btn:hover { background: var(--fill-3); color: var(--label); }
.head-chevron { color: var(--label-3); transition: transform 0.2s; }
.head-chevron.open { transform: rotate(90deg); }
.item-wrap { position: relative; }
.more { position: absolute; right: 4px; top: 4px; width: 24px; height: 24px; display: none; place-items: center; border-radius: 7px; color: var(--label-2); }
.more:hover { background: var(--fill-3); color: var(--label); }
.item-wrap:hover .more, .item-wrap:focus-within .more { display: grid; }
.item-wrap.has-more:hover .item-count, .item-wrap.has-more:focus-within .item-count { visibility: hidden; }
@media (hover: none) { .item-wrap .more { display: grid; } .item-wrap.has-more .item-count { visibility: hidden; } }
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
