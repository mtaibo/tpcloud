<script setup>
import { ref, computed, nextTick, onMounted, onUnmounted } from 'vue'
import { ChevronLeft, FolderDown, LayoutGrid, List, Download, ExternalLink } from 'lucide-vue-next'
import ShareFileRow from './ShareFileRow.vue'
import ShareGalleryThumb from './ShareGalleryThumb.vue'
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

const ctxMenu = ref(null)
const ctxVisible = ref(false)
const ctxPos = ref({ x: 0, y: 0 })
const ctxEntry = ref(null)

const breadcrumbs = computed(() => currentPath.value.split('/').filter(Boolean))
const rootLabel = computed(() => props.shareInfo?.label || props.token)

const ctxStyle = computed(() => ({
  left: ctxPos.value.x + 'px',
  top: ctxPos.value.y + 'px',
}))

function authHeaders() {
  return props.sessionToken ? { 'X-Share-Session': props.sessionToken } : {}
}

function toggleGallery() {
  galleryMode.value = !galleryMode.value
  localStorage.setItem('share-gallery-mode', galleryMode.value ? '1' : '0')
}

function entryIsImage(entry) {
  const ext = entry.name.split('.').pop()?.toLowerCase() ?? ''
  return IMAGE_EXTS.has(ext)
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

async function loadDir(path = '') {
  loading.value = true
  error.value = ''
  try {
    const params = new URLSearchParams({ path })
    const res = await fetch(`/api/share/${props.token}/files/list?${params}`, {
      headers: authHeaders(),
    })
    if (!res.ok) {
      error.value = 'Failed to load files'
      return
    }
    const data = await res.json()
    entries.value = data.entries
    currentPath.value = path
  } catch {
    error.value = 'Connection error'
  } finally {
    loading.value = false
  }
}

function navigateToCrumb(index) {
  const parts = breadcrumbs.value.slice(0, index + 1)
  loadDir(parts.join('/'))
}

function navigateUp() {
  const parts = breadcrumbs.value.slice(0, -1)
  loadDir(parts.join('/'))
}

function onOpen(entry) {
  const newPath = currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name
  loadDir(newPath)
}

async function downloadEntry(entry) {
  const path = currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name
  const url = `/api/share/${props.token}/files/download?path=${encodeURIComponent(path)}`
  const res = await fetch(url, { headers: authHeaders() })
  const blob = await res.blob()
  const a = document.createElement('a')
  a.href = URL.createObjectURL(blob)
  a.download = entry.name
  a.click()
  URL.revokeObjectURL(a.href)
}

async function downloadZip() {
  const path = currentPath.value
  const url = `/api/share/${props.token}/files/download-zip?path=${encodeURIComponent(path)}`
  const res = await fetch(url, { headers: authHeaders() })
  const blob = await res.blob()
  const dirName = breadcrumbs.value.at(-1) || rootLabel.value
  const a = document.createElement('a')
  a.href = URL.createObjectURL(blob)
  a.download = `${dirName}.zip`
  a.click()
  URL.revokeObjectURL(a.href)
}

function showCtxMenu(entry, e) {
  e.preventDefault()
  ctxEntry.value = entry
  ctxPos.value = { x: e.clientX, y: e.clientY }
  ctxVisible.value = true
  nextTick(() => {
    if (!ctxMenu.value) return
    const rect = ctxMenu.value.getBoundingClientRect()
    const x = Math.min(e.clientX, window.innerWidth - rect.width - 8)
    const y = Math.min(e.clientY, window.innerHeight - rect.height - 8)
    ctxPos.value = { x, y }
  })
}

function hideCtx() {
  ctxVisible.value = false
}

function openCtxEntry() {
  const entry = ctxEntry.value
  hideCtx()
  window.open(viewSrc(entry), '_blank')
}

function downloadCtxEntry() {
  const entry = ctxEntry.value
  hideCtx()
  downloadEntry(entry)
}

function onDocClick(e) {
  if (ctxVisible.value && ctxMenu.value && !ctxMenu.value.contains(e.target)) hideCtx()
}

onMounted(() => {
  document.addEventListener('click', onDocClick)
  loadDir('')
})
onUnmounted(() => document.removeEventListener('click', onDocClick))
</script>

<template>
  <div class="browser">
    <div class="topbar">
      <div class="topbar-inner">
        <div class="breadcrumbs">
          <button
            v-if="breadcrumbs.length"
            class="topbar-btn back-btn"
            @click="navigateUp"
          >
            <ChevronLeft class="btn-icon" />
          </button>

          <span
            class="crumb root-crumb"
            :class="{ active: !breadcrumbs.length }"
            @click="loadDir('')"
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

        <div class="topbar-actions">
          <button class="topbar-btn" :class="{ 'topbar-btn--active': galleryMode }" :title="galleryMode ? 'List view' : 'Gallery view'" @click="toggleGallery">
            <LayoutGrid v-if="!galleryMode" class="btn-icon" />
            <List v-else class="btn-icon" />
          </button>
          <button class="topbar-btn" title="Download as ZIP" @click="downloadZip">
            <FolderDown class="btn-icon" />
          </button>
        </div>
      </div>
    </div>

    <div class="content-wrap">
      <div v-if="loading" class="state-msg">Loading…</div>
      <div v-else-if="error" class="state-msg err">{{ error }}</div>
      <div v-else-if="!entries.length" class="state-msg">Empty folder</div>

      <!-- Gallery grid -->
      <div v-else-if="galleryMode" class="gallery-grid">
        <div
          v-for="entry in entries"
          :key="entry.name"
          class="gallery-card"
          @click="entry.type === 'directory' ? onOpen(entry) : null"
          @dblclick="entry.type === 'file' ? window.open(viewSrc(entry), '_blank') : null"
          @contextmenu.stop="showCtxMenu(entry, $event)"
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

      <!-- File table -->
      <table v-else class="file-table">
        <tbody>
          <ShareFileRow
            v-for="entry in entries"
            :key="entry.name"
            :entry="entry"
            @open="onOpen"
            @download="downloadEntry"
            @contextmenu.native.stop="showCtxMenu(entry, $event)"
          />
        </tbody>
      </table>
    </div>

    <!-- Context menu -->
    <Teleport to="body">
      <Transition name="ctx">
        <div v-if="ctxVisible" ref="ctxMenu" class="ctx-menu" :style="ctxStyle">
          <button v-if="ctxEntry?.type === 'file'" class="ctx-item" @click="openCtxEntry">
            <ExternalLink class="ctx-icon" />
            <span>Open</span>
          </button>
          <button class="ctx-item" @click="downloadCtxEntry">
            <Download class="ctx-icon" />
            <span>{{ ctxEntry?.type === 'directory' ? 'Download as ZIP' : 'Download' }}</span>
          </button>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.browser {
  display: flex;
  flex-direction: column;
  min-height: 100dvh;
}

.topbar {
  position: sticky;
  top: 0;
  background: rgba(0, 0, 0, 0.85);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.08);
  z-index: 10;
  padding: 0 1.5rem;
}

.topbar-inner {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  max-width: 860px;
  margin: 0 auto;
  padding: 10px 0;
}

.breadcrumbs {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.topbar-actions {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  flex-shrink: 0;
}

.topbar-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.06);
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  color: #636366;
  flex-shrink: 0;
  transition: background 0.15s, color 0.15s;
}
.topbar-btn:hover { background: rgba(255, 255, 255, 0.1); color: #fff; }
.topbar-btn--active { background: rgba(255, 255, 255, 0.12); color: #fff; border-color: rgba(255, 255, 255, 0.18); }
.back-btn { background: rgba(255, 255, 255, 0.06); color: #fff; }

.btn-icon { width: 16px; height: 16px; }

.crumb {
  font-size: 0.95rem;
  font-weight: 500;
  color: #636366;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 180px;
  cursor: default;
}
.crumb.active { color: #fff; }
.crumb:not(.active):hover { color: #ababab; }

.crumb-sep {
  color: #3a3a3c;
  font-size: 0.85rem;
  flex-shrink: 0;
}

.content-wrap {
  flex: 1;
  max-width: 860px;
  width: 100%;
  margin: 0 auto;
  padding: 0 1.5rem;
  box-sizing: border-box;
}

.state-msg {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.875rem;
  color: #636366;
  padding: 4rem 0;
}
.err { color: #ff453a; }

.file-table {
  width: 100%;
  border-collapse: collapse;
  table-layout: fixed;
}

.gallery-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
  gap: 12px;
  padding: 16px 0;
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

@media (max-width: 767px) {
  .gallery-grid { grid-template-columns: repeat(auto-fill, minmax(100px, 1fr)); gap: 8px; }
  .gallery-thumb { width: 88px; height: 88px; }
}
</style>

<style>
.ctx-menu {
  position: fixed;
  z-index: 9999;
  min-width: 180px;
  padding: 4px 0;
  background: rgba(28, 28, 30, 0.9);
  backdrop-filter: blur(40px) saturate(180%);
  -webkit-backdrop-filter: blur(40px) saturate(180%);
  border: 0.5px solid rgba(255, 255, 255, 0.14);
  border-radius: 10px;
  box-shadow: 0 8px 40px rgba(0, 0, 0, 0.55), inset 0 0.5px 0 rgba(255, 255, 255, 0.08);
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
.ctx-item:hover { background: #007AFF; color: #fff; }

.ctx-icon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  opacity: 0.7;
}
.ctx-item:hover .ctx-icon { opacity: 1; }

.ctx-enter-active { transition: opacity 0.1s ease, transform 0.1s ease; }
.ctx-leave-active { transition: opacity 0.08s ease, transform 0.08s ease; }
.ctx-enter-from, .ctx-leave-to { opacity: 0; transform: scale(0.96) translateY(-4px); }
</style>
