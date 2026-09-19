<script setup>
import { ref, computed, onMounted } from 'vue'
import { X, ChevronLeft, Lock, FolderDown, LayoutGrid, List, Download, ExternalLink } from 'lucide-vue-next'
import GalleryThumb from './GalleryThumb.vue'
import ImageViewer from './ImageViewer.vue'
import { getFileIcon, getIconColor, IMAGE_EXTS } from '../fileTypes.js'

const props = defineProps({
  entry: Object,
})
const emit = defineEmits(['close'])

const token = computed(() => props.entry.token)
const rootLabel = computed(() => props.entry.share?.label || props.entry.name || token.value)

const shareInfo = ref(null)
const sessionToken = ref(null)
const infoLoading = ref(true)
const infoError = ref('')
const currentPath = ref('')
const entries = ref([])
const dirLoading = ref(false)
const dirError = ref('')
const passwordInput = ref('')
const passwordError = ref('')
const passwordLoading = ref(false)
const galleryMode = ref(localStorage.getItem('share-gallery-mode') === '1')
const imageViewSrc = ref('')
const imageViewName = ref('')

const needsPassword = computed(() => shareInfo.value?.has_password && !sessionToken.value)
const breadcrumbs = computed(() => currentPath.value.split('/').filter(Boolean))

function authHeaders() {
  return sessionToken.value ? { 'X-Share-Session': sessionToken.value } : {}
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
  if (sessionToken.value) params.set('session', sessionToken.value)
  return `/api/share/${token.value}/files/thumbnail?${params}`
}

onMounted(async () => {
  sessionToken.value = sessionStorage.getItem(`share-session-${token.value}`)
  try {
    const res = await fetch(`/api/share/${token.value}`)
    if (!res.ok) { infoError.value = 'Share not found'; return }
    shareInfo.value = await res.json()
  } catch {
    infoError.value = 'Connection error'
  } finally {
    infoLoading.value = false
  }
  if (!needsPassword.value) loadDir('')
})

async function submitPassword() {
  if (!passwordInput.value) return
  passwordLoading.value = true
  passwordError.value = ''
  try {
    const res = await fetch(`/api/share/${token.value}/auth`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ password: passwordInput.value }),
    })
    if (!res.ok) { passwordError.value = 'Incorrect password'; return }
    const { session_token } = await res.json()
    sessionStorage.setItem(`share-session-${token.value}`, session_token)
    sessionToken.value = session_token
    loadDir('')
  } catch {
    passwordError.value = 'Connection error'
  } finally {
    passwordLoading.value = false
  }
}

async function loadDir(path) {
  dirLoading.value = true
  dirError.value = ''
  entries.value = []
  try {
    const params = new URLSearchParams({ path })
    const res = await fetch(`/api/share/${token.value}/files/list?${params}`, {
      headers: authHeaders(),
    })
    if (!res.ok) { dirError.value = 'Failed to load files'; return }
    const data = await res.json()
    entries.value = data.entries
    currentPath.value = path
  } catch {
    dirError.value = 'Connection error'
  } finally {
    dirLoading.value = false
  }
}

function openEntry(entry) {
  if (entry.type !== 'directory') return
  const path = currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name
  loadDir(path)
}

async function viewEntry(entry) {
  if (entry.type === 'directory') return
  const path = currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name
  const ext = entry.name.split('.').pop()?.toLowerCase() ?? ''
  if (IMAGE_EXTS.has(ext)) {
    const params = new URLSearchParams({ path })
    if (sessionToken.value) params.set('session', sessionToken.value)
    imageViewSrc.value = `/api/share/${token.value}/files/view?${params}`
    imageViewName.value = entry.name
    return
  }
  downloadEntry(entry)
}

async function downloadEntry(entry) {
  const path = currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name
  const url = `/api/share/${token.value}/files/download?path=${encodeURIComponent(path)}`
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
  const url = `/api/share/${token.value}/files/download-zip?path=${encodeURIComponent(path)}`
  const res = await fetch(url, { headers: authHeaders() })
  const blob = await res.blob()
  const dirName = breadcrumbs.value.at(-1) || rootLabel.value
  const a = document.createElement('a')
  a.href = URL.createObjectURL(blob)
  a.download = `${dirName}.zip`
  a.click()
  URL.revokeObjectURL(a.href)
}

function navigateUp() {
  const parts = breadcrumbs.value.slice(0, -1)
  loadDir(parts.join('/'))
}

function navigateToCrumb(index) {
  const parts = breadcrumbs.value.slice(0, index + 1)
  loadDir(parts.join('/'))
}
</script>

<template>
  <div class="inline-share" @click.self="emit('close')">
    <div class="panel">

      <!-- Loading share info -->
      <div v-if="infoLoading" class="state-full">Loading…</div>
      <div v-else-if="infoError" class="state-full err">{{ infoError }}</div>

      <!-- Password prompt -->
      <template v-else-if="needsPassword">
        <div class="panel-topbar">
          <button class="close-btn" @click="emit('close')">
            <X class="close-icon" />
          </button>
          <span class="topbar-title">{{ rootLabel }}</span>
        </div>
        <div class="password-body">
          <div class="password-card">
            <Lock class="lock-icon" />
            <h2 class="pw-title">Password required</h2>
            <p class="pw-sub">This shared folder is password protected.</p>
            <input
              v-model="passwordInput"
              class="pw-input"
              type="password"
              placeholder="Enter password"
              autocomplete="current-password"
              @keydown.enter="submitPassword"
            />
            <p v-if="passwordError" class="pw-error">{{ passwordError }}</p>
            <button class="pw-btn" :disabled="passwordLoading || !passwordInput" @click="submitPassword">
              {{ passwordLoading ? 'Verifying…' : 'Unlock' }}
            </button>
          </div>
        </div>
      </template>

      <!-- File browser -->
      <template v-else>
        <div class="panel-topbar">
          <button class="close-btn" @click="emit('close')">
            <X class="close-icon" />
          </button>
          <div class="breadcrumbs">
            <button v-if="breadcrumbs.length" class="back-btn" @click="navigateUp">
              <ChevronLeft class="back-icon" />
            </button>
            <span class="crumb" :class="{ active: !breadcrumbs.length }" @click="loadDir('')">{{ rootLabel }}</span>
            <template v-for="(seg, i) in breadcrumbs" :key="i">
              <span class="crumb-sep">/</span>
              <span class="crumb" :class="{ active: i === breadcrumbs.length - 1 }" @click="navigateToCrumb(i)">{{ seg }}</span>
            </template>
          </div>
          <button class="topbar-btn" :class="{ 'topbar-btn--active': galleryMode }" :title="galleryMode ? 'List view' : 'Gallery view'" @click="toggleGallery">
            <LayoutGrid v-if="!galleryMode" class="tb-icon" />
            <List v-else class="tb-icon" />
          </button>
          <button class="topbar-btn" title="Download as ZIP" @click="downloadZip">
            <FolderDown class="tb-icon" />
          </button>
        </div>

        <div class="panel-body">
          <div v-if="dirLoading" class="state-msg">Loading…</div>
          <div v-else-if="dirError" class="state-msg err">{{ dirError }}</div>
          <div v-else-if="!entries.length" class="state-msg">Empty folder</div>

          <!-- Gallery -->
          <div v-else-if="galleryMode" class="gallery-grid">
            <div
              v-for="entry in entries"
              :key="entry.name"
              class="gallery-card"
              @click="entry.type === 'directory' ? openEntry(entry) : null"
              @dblclick="(entry.type === 'file') ? viewEntry(entry) : null"
            >
              <div class="gallery-thumb">
                <GalleryThumb
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
            <tbody>
              <tr
                v-for="entry in entries"
                :key="entry.name"
                class="file-row"
                @click="entry.type === 'directory' ? openEntry(entry) : null"
                @dblclick="entry.type === 'file' ? viewEntry(entry) : null"
              >
                <td class="cell-name">
                  <div class="name-inner">
                    <component :is="getFileIcon(entry)" class="row-icon" :style="{ color: getIconColor(entry) }" />
                    <span class="row-name">{{ entry.name }}</span>
                  </div>
                </td>
                <td class="cell-action">
                  <button v-if="entry.type === 'file'" class="dl-btn" @click.stop="downloadEntry(entry)">
                    <Download class="dl-icon" />
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </template>

    </div>

    <!-- Image viewer -->
    <ImageViewer
      v-if="imageViewSrc"
      :src="imageViewSrc"
      :name="imageViewName"
      @close="imageViewSrc = ''; imageViewName = ''"
    />
  </div>
</template>

<style scoped>
.inline-share {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(0, 0, 0, 0.7);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding: 3vh 1rem;
  overflow-y: auto;
}

.panel {
  width: 100%;
  max-width: 800px;
  min-height: 60vh;
  background: #111113;
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  border-radius: 16px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  box-shadow: 0 32px 80px rgba(0, 0, 0, 0.7);
  flex-shrink: 0;
}

.state-full {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.875rem;
  color: #636366;
  padding: 4rem;
}
.state-full.err { color: #ff453a; }

/* Topbar */
.panel-topbar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 10px 1rem;
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.08);
  flex-shrink: 0;
}

.close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.07);
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  color: rgba(255, 255, 255, 0.7);
  flex-shrink: 0;
  transition: background 0.12s, color 0.12s;
}
.close-btn:hover { background: rgba(255, 255, 255, 0.12); color: #fff; }
.close-icon { width: 15px; height: 15px; }

.topbar-title {
  flex: 1;
  font-size: 0.95rem;
  font-weight: 600;
  color: #fff;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.breadcrumbs {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.back-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.06);
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  color: rgba(255, 255, 255, 0.8);
  flex-shrink: 0;
}
.back-btn:hover { background: rgba(255, 255, 255, 0.1); }
.back-icon { width: 14px; height: 14px; stroke-width: 2.5; }

.crumb {
  font-size: 0.9rem;
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

.crumb-sep { color: #3a3a3c; font-size: 0.85rem; flex-shrink: 0; }

.topbar-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.06);
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  color: #636366;
  flex-shrink: 0;
  transition: background 0.15s, color 0.15s;
}
.topbar-btn:hover { background: rgba(255, 255, 255, 0.1); color: #fff; }
.topbar-btn--active { color: #fff; background: rgba(255, 255, 255, 0.12); }
.tb-icon { width: 15px; height: 15px; }

/* Password prompt */
.password-body {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
}

.password-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
  max-width: 300px;
  width: 100%;
}

.lock-icon { width: 28px; height: 28px; color: #8E8E93; }

.pw-title {
  font-size: 1rem;
  font-weight: 600;
  color: #fff;
  text-align: center;
}

.pw-sub {
  font-size: 0.8rem;
  color: #636366;
  text-align: center;
}

.pw-input {
  width: 100%;
  padding: 10px 14px;
  background: rgba(255, 255, 255, 0.06);
  border: 0.5px solid rgba(255, 255, 255, 0.12);
  border-radius: 10px;
  color: #fff;
  font-size: 0.9rem;
  font-family: inherit;
  outline: none;
  box-sizing: border-box;
  transition: border-color 0.15s, box-shadow 0.15s;
}
.pw-input::placeholder { color: #3a3a3c; }
.pw-input:focus { border-color: #8E8E93; box-shadow: 0 0 0 3px rgba(142, 142, 147, 0.2); }

.pw-error { font-size: 0.8rem; color: #ff453a; text-align: center; }

.pw-btn {
  width: 100%;
  padding: 10px;
  border-radius: 10px;
  font-size: 0.9rem;
  font-weight: 600;
  color: #fff;
  background: #8E8E93;
  border: none;
  font-family: inherit;
  transition: opacity 0.12s;
}
.pw-btn:disabled { opacity: 0.4; cursor: default; }
.pw-btn:not(:disabled):hover { opacity: 0.88; }

/* Panel body */
.panel-body {
  flex: 1;
  overflow-y: auto;
}

.state-msg {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.875rem;
  color: #636366;
  padding: 3rem;
}
.state-msg.err { color: #ff453a; }

/* File list */
.file-table {
  width: 100%;
  border-collapse: collapse;
  table-layout: fixed;
}

.file-row {
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.05);
  transition: background 0.1s;
  cursor: default;
}
.file-row:hover { background: rgba(255, 255, 255, 0.04); }

.cell-name { padding: 0.9rem 1.25rem; width: 100%; }

.name-inner {
  display: flex;
  align-items: center;
  gap: 0.7rem;
  font-size: 0.95rem;
}

.row-icon { width: 20px; height: 20px; flex-shrink: 0; }

.row-name {
  color: #d1d1d6;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cell-action { padding: 0 1rem 0 0.5rem; }

.dl-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: transparent;
  border: none;
  color: #636366;
  opacity: 0;
  transition: opacity 0.15s, background 0.15s, color 0.15s;
}
.file-row:hover .dl-btn { opacity: 1; }
.dl-btn:hover { background: rgba(255,255,255,0.07); color: #fff; }
.dl-icon { width: 15px; height: 15px; }

/* Gallery */
.gallery-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
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
  width: 100px;
  height: 100px;
  border-radius: 8px;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.05);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.gallery-icon { width: 40px; height: 40px; }

.gallery-name {
  font-size: 0.72rem;
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
  .inline-share { padding: 0; align-items: flex-end; }
  .panel { max-width: 100%; border-bottom-left-radius: 0; border-bottom-right-radius: 0; min-height: 80vh; }
}
</style>
