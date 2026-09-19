<script setup>
import { ref, computed, nextTick, onMounted, onUnmounted } from 'vue'
import { ChevronLeft, ChevronRight, MoreHorizontal, LayoutGrid, List, FolderDown, Download, ExternalLink } from 'lucide-vue-next'
import ShareFileRow from './ShareFileRow.vue'
import ShareGalleryThumb from './ShareGalleryThumb.vue'
import ImageViewer from './ImageViewer.vue'
import { getFileIcon, getIconColor, IMAGE_EXTS } from '../fileTypes.js'

const props = defineProps({
  token: String,
  shareInfo: Object,
  sessionToken: String,
})

const currentPath = ref('')
const entries = ref([])
const loading = ref(false)
const error = ref('')
const galleryMode = ref(localStorage.getItem('share-gallery-mode') === '1')

const navHistory = ref([''])
const navIndex = ref(0)
const canGoBack = computed(() => navIndex.value > 0)
const canGoForward = computed(() => navIndex.value < navHistory.value.length - 1)

const contextMenu = ref(null)
const moreBtn = ref(null)
const menuVisible = ref(false)
const menuPos = ref({ x: 0, y: 0 })
const activeEntry = ref(null)

const imageViewEntry = ref(null)
const imageViewSrc = computed(() => {
  if (!imageViewEntry.value) return ''
  const path = currentPath.value ? `${currentPath.value}/${imageViewEntry.value.name}` : imageViewEntry.value.name
  const params = new URLSearchParams({ path })
  if (props.sessionToken) params.set('session', props.sessionToken)
  return `/api/share/${props.token}/files/view?${params}`
})

const breadcrumbs = computed(() => currentPath.value.split('/').filter(Boolean))
const rootLabel = computed(() => props.shareInfo?.label || props.token)
const currentFolderName = computed(() => {
  if (!currentPath.value) return rootLabel.value
  const parts = currentPath.value.split('/').filter(Boolean)
  return parts[parts.length - 1]
})
const menuStyle = computed(() => ({ left: menuPos.value.x + 'px', top: menuPos.value.y + 'px' }))

function authHeaders() {
  return props.sessionToken ? { 'X-Share-Session': props.sessionToken } : {}
}

function toggleGallery() {
  galleryMode.value = !galleryMode.value
  localStorage.setItem('share-gallery-mode', galleryMode.value ? '1' : '0')
}

function entryIsImage(entry) {
  return IMAGE_EXTS.has(entry.name.split('.').pop()?.toLowerCase() ?? '')
}

function thumbSrc(entry) {
  const path = currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name
  const params = new URLSearchParams({ path })
  if (props.sessionToken) params.set('session', props.sessionToken)
  return `/api/share/${props.token}/files/thumbnail?${params}`
}

function viewSrc(entry) {
  const path = currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name
  const params = new URLSearchParams({ path })
  if (props.sessionToken) params.set('session', props.sessionToken)
  return `/api/share/${props.token}/files/view?${params}`
}

async function loadDir(path) {
  loading.value = true
  error.value = ''
  try {
    const params = new URLSearchParams({ path })
    const res = await fetch(`/api/share/${props.token}/files/list?${params}`, {
      headers: authHeaders(),
    })
    if (!res.ok) { error.value = 'Failed to load files'; return }
    const data = await res.json()
    entries.value = data.entries
    currentPath.value = path
  } catch {
    error.value = 'Connection error'
  } finally {
    loading.value = false
  }
}

async function navigateTo(path, push = true) {
  if (push) {
    navHistory.value = navHistory.value.slice(0, navIndex.value + 1)
    navHistory.value.push(path)
    navIndex.value++
  }
  await loadDir(path)
}

function goBack() {
  if (!canGoBack.value) return
  navIndex.value--
  loadDir(navHistory.value[navIndex.value])
}

function goForward() {
  if (!canGoForward.value) return
  navIndex.value++
  loadDir(navHistory.value[navIndex.value])
}

function onOpen(entry) {
  const newPath = currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name
  navigateTo(newPath)
}

function navigateToCrumb(index) {
  const parts = breadcrumbs.value.slice(0, index + 1)
  navigateTo(parts.join('/'))
}

async function downloadEntry(entry) {
  const path = currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name
  const res = await fetch(`/api/share/${props.token}/files/download?path=${encodeURIComponent(path)}`, {
    headers: authHeaders(),
  })
  const blob = await res.blob()
  const a = document.createElement('a')
  a.href = URL.createObjectURL(blob)
  a.download = entry.name
  a.click()
  URL.revokeObjectURL(a.href)
}

async function downloadZip() {
  const path = currentPath.value
  const res = await fetch(`/api/share/${props.token}/files/download-zip?path=${encodeURIComponent(path)}`, {
    headers: authHeaders(),
  })
  const blob = await res.blob()
  const name = breadcrumbs.value.at(-1) || rootLabel.value
  const a = document.createElement('a')
  a.href = URL.createObjectURL(blob)
  a.download = `${name}.zip`
  a.click()
  URL.revokeObjectURL(a.href)
}

function showMenu(e) {
  e.preventDefault()
  activeEntry.value = null
  _openMenu(e)
}

function showMenuForEntry(entry, e) {
  e.preventDefault()
  activeEntry.value = entry
  _openMenu(e)
}

function _openMenu(e) {
  menuPos.value = { x: e.clientX, y: e.clientY }
  menuVisible.value = true
  nextTick(() => {
    if (!contextMenu.value) return
    const rect = contextMenu.value.getBoundingClientRect()
    menuPos.value = {
      x: Math.min(e.clientX, window.innerWidth - rect.width - 8),
      y: Math.min(e.clientY, window.innerHeight - rect.height - 8),
    }
  })
}

function hideMenu() { menuVisible.value = false }

function showMoreMenu() {
  activeEntry.value = null
  menuPos.value = { x: -9999, y: -9999 }
  menuVisible.value = true
  nextTick(() => {
    if (!contextMenu.value || !moreBtn.value) return
    const btn = moreBtn.value.getBoundingClientRect()
    const menu = contextMenu.value.getBoundingClientRect()
    menuPos.value = {
      x: Math.max(8, btn.right - menu.width),
      y: Math.min(btn.bottom + 8, window.innerHeight - menu.height - 8),
    }
  })
}

function onFileActivate(entry) {
  const ext = entry.name.split('.').pop()?.toLowerCase() ?? ''
  if (IMAGE_EXTS.has(ext)) {
    imageViewEntry.value = entry
  } else {
    openInTab(entry)
  }
}

function onMiddleClick(entry, e) {
  if (e.button === 1 && entry.type === 'file') openInTab(entry)
}

async function openInTab(entry) {
  const path = currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name
  const newTab = window.open('', '_blank')
  if (!newTab) return
  try {
    const tokenRes = await fetch(`/api/share/${props.token}/files/token`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ path }),
    })
    if (tokenRes.ok) {
      const { token: fileToken } = await tokenRes.json()
      const infoRes = await fetch(`/api/open/${fileToken}/info`)
      const ct = infoRes.ok ? (infoRes.headers.get('content-type') || '') : ''
      if (ct.includes('application/json')) {
        newTab.location.href = `/${fileToken}`
        return
      }
    }
  } catch {}
  newTab.location.href = viewSrc(entry)
}

function openActiveItem() {
  onFileActivate(activeEntry.value)
  hideMenu()
}

function downloadActiveItem() {
  downloadEntry(activeEntry.value)
  hideMenu()
}

function downloadFolderZip() {
  const entry = activeEntry.value
  hideMenu()
  const path = currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name
  fetch(`/api/share/${props.token}/files/download-zip?path=${encodeURIComponent(path)}`, { headers: authHeaders() })
    .then(r => r.blob())
    .then(blob => {
      const a = document.createElement('a')
      a.href = URL.createObjectURL(blob)
      a.download = `${entry.name}.zip`
      a.click()
      URL.revokeObjectURL(a.href)
    })
}

function onDocClick(e) {
  if (menuVisible.value && contextMenu.value && !contextMenu.value.contains(e.target)) hideMenu()
}

onMounted(() => {
  document.addEventListener('click', onDocClick)
  loadDir('')
})
onUnmounted(() => document.removeEventListener('click', onDocClick))
</script>

<template>
  <div class="browser">

    <!-- Topbar -->
    <div class="file-topbar">
      <div class="nav-pill">
        <button class="nav-btn" :disabled="!canGoBack" @click="goBack" title="Back">
          <ChevronLeft class="nav-icon" />
        </button>
        <div class="nav-divider" />
        <button class="nav-btn" :disabled="!canGoForward" @click="goForward" title="Forward">
          <ChevronRight class="nav-icon" />
        </button>
      </div>

      <span class="folder-name">{{ currentFolderName }}</span>

      <button
        class="more-btn"
        :class="{ 'more-btn--active': galleryMode }"
        :title="galleryMode ? 'List view' : 'Gallery view'"
        @click="toggleGallery"
      >
        <LayoutGrid v-if="!galleryMode" class="more-icon" />
        <List v-else class="more-icon" />
      </button>

      <button ref="moreBtn" class="more-btn" @click.stop="showMoreMenu">
        <MoreHorizontal class="more-icon" />
      </button>
    </div>

    <!-- File area -->
    <div class="file-area" @contextmenu="showMenu">

      <div v-if="loading" class="state-center">
        <span class="state-text">Loading…</span>
      </div>
      <div v-else-if="error" class="state-center">
        <span class="state-text error">{{ error }}</span>
      </div>
      <div v-else-if="!entries.length" class="state-center">
        <span class="state-text">Empty folder</span>
      </div>

      <!-- Gallery -->
      <div v-else-if="galleryMode" class="gallery-grid">
        <div
          v-for="entry in entries"
          :key="entry.name"
          class="gallery-card"
          @click="entry.type === 'directory' ? onOpen(entry) : null"
          @dblclick="entry.type === 'file' ? onFileActivate(entry) : null"
          @auxclick.prevent="onMiddleClick(entry, $event)"
          @contextmenu.stop="showMenuForEntry(entry, $event)"
        >
          <div class="gallery-thumb">
            <ShareGalleryThumb
              v-if="entry.type === 'file' && entryIsImage(entry)"
              :src="thumbSrc(entry)"
              :alt="entry.name"
            />
            <component
              v-else
              :is="getFileIcon(entry)"
              class="gallery-icon"
              :style="{ color: getIconColor(entry) }"
            />
          </div>
          <span class="gallery-name">{{ entry.name }}</span>
        </div>
      </div>

      <!-- List -->
      <table v-else class="file-table">
        <thead>
          <tr>
            <th class="th-name">Name</th>
            <th class="th-size">Size</th>
          </tr>
        </thead>
        <tbody>
          <ShareFileRow
            v-for="entry in entries"
            :key="entry.name"
            :entry="entry"
            @open="onOpen"
            @open-file="onFileActivate"
            @open-tab="openInTab"
            @download="downloadEntry"
            @contextmenu.stop="showMenuForEntry(entry, $event)"
          />
        </tbody>
      </table>
    </div>

    <!-- Bottom bar with breadcrumbs -->
    <div class="bottom-bar">
      <div class="breadcrumbs">
        <span
          class="crumb"
          :class="{ active: !breadcrumbs.length }"
          @click="navigateTo('')"
        >{{ rootLabel }}</span>
        <template v-for="(seg, i) in breadcrumbs" :key="i">
          <span class="crumb-sep">/</span>
          <span
            class="crumb"
            :class="{ active: i === breadcrumbs.length - 1 }"
            @click="navigateToCrumb(i)"
          >{{ seg }}</span>
        </template>
      </div>
    </div>

    <!-- Context menu -->
    <Teleport to="body">
      <Transition name="ctx">
        <div v-if="menuVisible" ref="contextMenu" class="ctx-menu" :style="menuStyle">

          <!-- File/folder entry menu -->
          <template v-if="activeEntry">
            <button v-if="activeEntry.type === 'file'" class="ctx-item" @click="openActiveItem">
              <ExternalLink class="ctx-icon" />
              <span>Open</span>
            </button>
            <button class="ctx-item" @click="activeEntry.type === 'directory' ? downloadFolderZip() : downloadActiveItem()">
              <Download class="ctx-icon" />
              <span>{{ activeEntry.type === 'directory' ? 'Download as ZIP' : 'Download' }}</span>
            </button>
          </template>

          <!-- Background menu -->
          <template v-else>
            <button class="ctx-item" @click="downloadZip(); hideMenu()">
              <FolderDown class="ctx-icon" />
              <span>Download as ZIP</span>
            </button>
          </template>

        </div>
      </Transition>
    </Teleport>

    <ImageViewer
      v-if="imageViewEntry"
      :src="imageViewSrc"
      :name="imageViewEntry.name"
      @close="imageViewEntry = null"
    />

  </div>
</template>

<style scoped>
.browser {
  display: flex;
  flex-direction: column;
  height: 100dvh;
  max-width: 900px;
  margin: 0 auto;
}

/* ── Topbar ── */
.file-topbar {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  padding: 10px 1.25rem;
  gap: 0;
}

.nav-pill {
  display: flex;
  align-items: center;
  background: rgba(255, 255, 255, 0.07);
  border: 0.5px solid rgba(255, 255, 255, 0.12);
  border-radius: 999px;
  backdrop-filter: blur(20px) saturate(150%);
  -webkit-backdrop-filter: blur(20px) saturate(150%);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25), inset 0 0.5px 0 rgba(255, 255, 255, 0.08);
  overflow: hidden;
  flex-shrink: 0;
}

.nav-btn {
  width: 40px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.85);
  transition: background 0.12s, color 0.12s;
}
.nav-btn:hover:not(:disabled) { background: rgba(255, 255, 255, 0.08); }
.nav-btn:active:not(:disabled) { background: rgba(255, 255, 255, 0.04); }
.nav-btn:disabled { color: rgba(255, 255, 255, 0.2); cursor: default; }

.nav-divider {
  width: 0.5px;
  height: 14px;
  background: rgba(255, 255, 255, 0.15);
  flex-shrink: 0;
}

.nav-icon { width: 16px; height: 16px; stroke-width: 2.5; }

.folder-name {
  flex: 1;
  font-size: 1rem;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.75);
  margin-left: 0.75rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}

.more-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  margin-left: auto;
  background: rgba(255, 255, 255, 0.07);
  border: 0.5px solid rgba(255, 255, 255, 0.12);
  border-radius: 999px;
  color: rgba(255, 255, 255, 0.85);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25), inset 0 0.5px 0 rgba(255, 255, 255, 0.08);
  transition: background 0.15s;
}
.more-btn:hover { background: rgba(255, 255, 255, 0.10); }
.more-btn:active { background: rgba(255, 255, 255, 0.04); }
.more-btn--active { background: rgba(255, 255, 255, 0.14); border-color: rgba(255, 255, 255, 0.22); }
.more-icon { width: 16px; height: 16px; }

/* ── File area ── */
.file-area {
  flex: 1;
  overflow-y: auto;
  position: relative;
}

.state-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 10rem;
}
.state-text { font-size: 0.875rem; color: #525252; }
.state-text.error { color: #ff453a; }

/* List table */
.file-table {
  width: 100%;
  border-collapse: collapse;
  table-layout: fixed;
}
.file-table thead th {
  padding: 0.65rem 1.25rem;
  font-size: 0.7rem;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.07em;
  color: #525252;
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.1);
  user-select: none;
}
.th-name { text-align: left; width: 100%; }
.th-size { text-align: right; white-space: nowrap; }

/* Gallery */
.gallery-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
  gap: 12px;
  padding: 16px 1.25rem;
}

.gallery-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 10px 8px;
  border-radius: 10px;
  cursor: default;
  transition: background 0.1s;
  user-select: none;
}
.gallery-card:hover { background: rgba(255, 255, 255, 0.06); }

.gallery-thumb {
  width: 110px;
  height: 110px;
  border-radius: 8px;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.05);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.gallery-icon { width: 44px; height: 44px; }
.gallery-name {
  font-size: 0.75rem;
  color: #d1d1d6;
  text-align: center;
  word-break: break-word;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  line-height: 1.3;
  max-width: 100%;
}

/* ── Bottom bar ── */
.bottom-bar {
  flex-shrink: 0;
  padding: 0.65rem 1.25rem;
  border-top: 0.5px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.02);
}

.breadcrumbs {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  flex-wrap: nowrap;
  overflow: hidden;
}

.crumb {
  font-size: 0.75rem;
  color: #3a3a3c;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 160px;
  cursor: default;
  transition: color 0.12s;
}
.crumb:hover { color: #636366; }
.crumb.active { color: #525252; }

.crumb-sep { color: #2c2c2e; font-size: 0.7rem; flex-shrink: 0; }

/* ── Mobile ── */
@media (max-width: 767px) {
  .file-topbar {
    position: fixed;
    top: 0; left: 0; right: 0;
    height: calc(80px + env(safe-area-inset-top));
    padding-top: env(safe-area-inset-top);
    padding-left: 1.25rem;
    padding-right: 1.25rem;
    z-index: 100;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(24px) saturate(180%);
    -webkit-backdrop-filter: blur(24px) saturate(180%);
    border-bottom: 0.5px solid rgba(255, 255, 255, 0.07);
  }
  .file-area {
    padding-top: calc(80px + env(safe-area-inset-top));
    padding-bottom: 60px;
  }
  .folder-name { font-size: 1.25rem; margin-left: 1.5rem; }
  .bottom-bar { display: none; }
  .th-size { display: none; }
  .gallery-grid { grid-template-columns: repeat(auto-fill, minmax(100px, 1fr)); gap: 8px; padding: 12px 1rem; }
  .gallery-thumb { width: 88px; height: 88px; }
}
</style>

<style>
.ctx-menu {
  position: fixed;
  z-index: 9999;
  min-width: 210px;
  padding: 4px 0;
  background: rgba(28, 28, 30, 0.9);
  backdrop-filter: blur(40px) saturate(180%);
  -webkit-backdrop-filter: blur(40px) saturate(180%);
  border: 0.5px solid rgba(255, 255, 255, 0.14);
  border-radius: 10px;
  box-shadow: 0 8px 40px rgba(0, 0, 0, 0.55), 0 2px 8px rgba(0, 0, 0, 0.3), inset 0 0.5px 0 rgba(255, 255, 255, 0.08);
}

.ctx-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: calc(100% - 8px);
  margin: 0 4px;
  padding: 6px 10px;
  font-size: 13px;
  color: #e4e4e7;
  background: none;
  border: none;
  border-radius: 6px;
  text-align: left;
  transition: background 0.08s, color 0.08s;
  font-family: inherit;
  letter-spacing: -0.01em;
}
.ctx-item:hover { background: #8E8E93; color: #fff; }
.ctx-icon { width: 14px; height: 14px; flex-shrink: 0; opacity: 0.7; }
.ctx-item:hover .ctx-icon { opacity: 1; }

.ctx-enter-active { transition: opacity 0.1s ease, transform 0.1s ease; }
.ctx-leave-active { transition: opacity 0.08s ease, transform 0.08s ease; }
.ctx-enter-from, .ctx-leave-to { opacity: 0; transform: scale(0.96) translateY(-4px); }

@media (max-width: 767px) {
  .ctx-menu { min-width: 260px; background: rgba(20, 20, 22, 0.97); backdrop-filter: blur(60px) saturate(220%); -webkit-backdrop-filter: blur(60px) saturate(220%); }
  .ctx-item { font-size: 15px; padding: 8px 12px; gap: 12px; }
  .ctx-icon { width: 16px; height: 16px; }
}
</style>
