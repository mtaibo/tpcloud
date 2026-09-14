<script setup>
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { FolderPlus, Upload, FilePlus, Pencil, FolderInput, Copy, CopyPlus, Download, Archive, ArchiveRestore, Trash2, ChevronLeft } from 'lucide-vue-next'
import Breadcrumb from './Breadcrumb.vue'
import FileRow from './FileRow.vue'
import MoveModal from './MoveModal.vue'

const props = defineProps({
  user: Object,
  location: String,
  currentPath: String,
})

const emit = defineEmits(['navigate'])

const entries = ref([])
const loading = ref(false)
const error = ref(null)
const uploading = ref(false)
const isDragOver = ref(false)
const fileInput = ref(null)

const contextMenu = ref(null)
const menuVisible = ref(false)
const menuPos = ref({ x: 0, y: 0 })
const activeEntry = ref(null)

const pickerEntry = ref(null)
const pickerMode = ref('move')

const activeIsZip = computed(() =>
  activeEntry.value?.name?.toLowerCase().endsWith('.zip') ?? false
)

const canGoBack = computed(() => !!props.currentPath)

function goBack() {
  if (!props.currentPath) return
  const parts = props.currentPath.split('/').filter(Boolean)
  parts.pop()
  emit('navigate', props.location, parts.join('/'))
}

const menuStyle = computed(() => ({
  left: menuPos.value.x + 'px',
  top: menuPos.value.y + 'px',
}))

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
    const x = Math.min(e.clientX, window.innerWidth - rect.width - 8)
    const y = Math.min(e.clientY, window.innerHeight - rect.height - 8)
    menuPos.value = { x, y }
  })
}

function hideMenu() {
  menuVisible.value = false
}

function onDocClick(e) {
  if (menuVisible.value && contextMenu.value && !contextMenu.value.contains(e.target)) {
    hideMenu()
  }
}

function onDocKeydown(e) {
  if (e.key === 'Escape') {
    hideMenu()
    pickerEntry.value = null
  }
}

onMounted(() => {
  document.addEventListener('click', onDocClick)
  document.addEventListener('keydown', onDocKeydown)
})

onUnmounted(() => {
  document.removeEventListener('click', onDocClick)
  document.removeEventListener('keydown', onDocKeydown)
})

async function loadDirectory() {
  loading.value = true
  error.value = null
  try {
    const params = new URLSearchParams({ path: props.currentPath, location: props.location })
    const res = await fetch(`/api/files/list?${params}`)
    if (!res.ok) {
      const data = await res.json().catch(() => ({}))
      throw new Error(data.detail || `Error ${res.status}`)
    }
    const data = await res.json()
    entries.value = data.entries
  } catch (e) {
    error.value = e.message
    entries.value = []
  } finally {
    loading.value = false
  }
}

watch([() => props.location, () => props.currentPath], loadDirectory, { immediate: true })

function openItem(entry) {
  if (entry.type !== 'directory') return
  const newPath = props.currentPath
    ? `${props.currentPath}/${entry.name}`
    : entry.name
  emit('navigate', props.location, newPath)
}

async function deleteItem(entry) {
  if (!confirm(`Delete "${entry.name}"?\nThis action cannot be undone.`)) return
  const path = props.currentPath ? `${props.currentPath}/${entry.name}` : entry.name
  const params = new URLSearchParams({ path, location: props.location })
  const res = await fetch(`/api/files?${params}`, { method: 'DELETE' })
  if (res.ok) {
    loadDirectory()
  } else {
    const data = await res.json().catch(() => ({}))
    alert(data.detail || 'Error deleting item')
  }
}

function downloadItem(entry) {
  const path = props.currentPath ? `${props.currentPath}/${entry.name}` : entry.name
  const params = new URLSearchParams({ path, location: props.location })
  const a = document.createElement('a')
  a.href = `/api/files/download?${params}`
  a.download = entry.name
  a.click()
}

async function createFolder() {
  hideMenu()
  const name = prompt('New folder name:')
  if (!name || !name.trim()) return
  const path = props.currentPath ? `${props.currentPath}/${name.trim()}` : name.trim()
  const res = await fetch('/api/files/mkdir', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path, location: props.location }),
  })
  if (res.ok) {
    loadDirectory()
  } else {
    const data = await res.json().catch(() => ({}))
    alert(data.detail || 'Error creating folder')
  }
}

async function createFile() {
  hideMenu()
  const name = prompt('New file name:')
  if (!name || !name.trim()) return
  const path = props.currentPath ? `${props.currentPath}/${name.trim()}` : name.trim()
  const res = await fetch('/api/files/touch', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path, location: props.location }),
  })
  if (res.ok) {
    loadDirectory()
  } else {
    const data = await res.json().catch(() => ({}))
    alert(data.detail || 'Error creating file')
  }
}

async function renameItem() {
  const entry = activeEntry.value
  hideMenu()
  const newName = prompt('Rename to:', entry.name)
  if (!newName || !newName.trim() || newName.trim() === entry.name) return
  const path = props.currentPath ? `${props.currentPath}/${entry.name}` : entry.name
  const res = await fetch('/api/files/rename', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path, new_name: newName.trim(), location: props.location }),
  })
  if (res.ok) {
    loadDirectory()
  } else {
    const data = await res.json().catch(() => ({}))
    alert(data.detail || 'Error renaming')
  }
}

function startMove() {
  pickerEntry.value = activeEntry.value
  pickerMode.value = 'move'
  hideMenu()
}

function startCopy() {
  pickerEntry.value = activeEntry.value
  pickerMode.value = 'copy'
  hideMenu()
}

async function duplicateItem() {
  const entry = activeEntry.value
  hideMenu()
  const path = props.currentPath ? `${props.currentPath}/${entry.name}` : entry.name
  const res = await fetch('/api/files/duplicate', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path, location: props.location }),
  })
  if (res.ok) {
    loadDirectory()
  } else {
    const data = await res.json().catch(() => ({}))
    alert(data.detail || 'Error duplicating')
  }
}

function downloadActiveItem() {
  const entry = activeEntry.value
  hideMenu()
  downloadItem(entry)
}

function downloadFolderAsZip() {
  const entry = activeEntry.value
  hideMenu()
  const path = props.currentPath ? `${props.currentPath}/${entry.name}` : entry.name
  const params = new URLSearchParams({ path, location: props.location })
  const a = document.createElement('a')
  a.href = `/api/files/download-zip?${params}`
  a.download = `${entry.name}.zip`
  a.click()
}

async function compressItem() {
  const entry = activeEntry.value
  hideMenu()
  const path = props.currentPath ? `${props.currentPath}/${entry.name}` : entry.name
  const res = await fetch('/api/files/compress', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path, location: props.location }),
  })
  if (res.ok) {
    loadDirectory()
  } else {
    const data = await res.json().catch(() => ({}))
    alert(data.detail || 'Error compressing')
  }
}

async function decompressItem() {
  const entry = activeEntry.value
  hideMenu()
  const path = props.currentPath ? `${props.currentPath}/${entry.name}` : entry.name
  const res = await fetch('/api/files/decompress', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path, location: props.location }),
  })
  if (res.ok) {
    loadDirectory()
  } else {
    const data = await res.json().catch(() => ({}))
    alert(data.detail || 'Error decompressing')
  }
}

async function deleteActiveItem() {
  const entry = activeEntry.value
  hideMenu()
  await deleteItem(entry)
}

function triggerUpload() {
  hideMenu()
  fileInput.value.click()
}

async function uploadFiles(files) {
  if (!files || !files.length) return
  uploading.value = true
  const formData = new FormData()
  for (const file of files) formData.append('files', file)
  const params = new URLSearchParams({ path: props.currentPath, location: props.location })
  try {
    const res = await fetch(`/api/files/upload?${params}`, { method: 'POST', body: formData })
    if (!res.ok) {
      const data = await res.json().catch(() => ({}))
      alert(data.detail || 'Error uploading files')
    } else {
      loadDirectory()
    }
  } finally {
    uploading.value = false
  }
}

function onFileInputChange(e) {
  uploadFiles(Array.from(e.target.files))
  e.target.value = ''
}

function onDragOver(e) {
  e.preventDefault()
  isDragOver.value = true
}

function onDragLeave(e) {
  if (!e.currentTarget.contains(e.relatedTarget)) {
    isDragOver.value = false
  }
}

function onDrop(e) {
  e.preventDefault()
  isDragOver.value = false
  uploadFiles(Array.from(e.dataTransfer.files))
}
</script>

<template>
  <div class="browser">
    <input ref="fileInput" type="file" multiple style="display:none" @change="onFileInputChange" />

    <!-- Back button -->
    <div class="file-topbar">
      <button
        class="back-btn"
        :class="{ visible: canGoBack }"
        :disabled="!canGoBack"
        @click="goBack"
        title="Go back"
      >
        <ChevronLeft class="back-icon" />
      </button>
    </div>

    <!-- File area -->
    <div
      class="file-area"
      @contextmenu="showMenu"
      @dragover="onDragOver"
      @dragleave="onDragLeave"
      @drop="onDrop"
    >
      <!-- Drag overlay -->
      <div v-if="isDragOver" class="drag-overlay">
        <p>Drop files here</p>
      </div>

      <!-- Loading -->
      <div v-if="loading" class="state-center">
        <span class="state-text">Loading…</span>
      </div>

      <!-- Error -->
      <div v-else-if="error" class="state-center">
        <span class="state-text error">{{ error }}</span>
      </div>

      <!-- Empty -->
      <div v-else-if="!entries.length" class="state-center">
        <span class="state-text">Empty folder</span>
        <span class="state-hint">Right-click to upload or create files</span>
      </div>

      <!-- File table -->
      <table v-else class="file-table">
        <thead>
          <tr>
            <th class="th-name">Name</th>
            <th class="th-size">Size</th>
            <th class="th-date">Modified</th>
          </tr>
        </thead>
        <tbody>
          <FileRow
            v-for="entry in entries"
            :key="entry.name"
            :entry="entry"
            @open="openItem"
            @contextmenu="showMenuForEntry"
          />
        </tbody>
      </table>
    </div>

    <!-- Bottom bar -->
    <div class="bottom-bar">
      <Breadcrumb
        :user="user"
        :location="location"
        :path="currentPath"
        @navigate="(loc, p) => emit('navigate', loc, p)"
      />
      <span v-if="uploading" class="uploading-indicator">Uploading…</span>
    </div>

    <!-- Context menu -->
    <Teleport to="body">
      <Transition name="ctx">
        <div
          v-if="menuVisible"
          ref="contextMenu"
          class="ctx-menu"
          :style="menuStyle"
        >
          <!-- Entry menu (right-click on a file/folder) -->
          <template v-if="activeEntry">
            <button class="ctx-item" @click="renameItem">
              <Pencil class="ctx-icon" />
              <span>Rename</span>
            </button>
            <div class="ctx-sep" />
            <button class="ctx-item" @click="startMove">
              <FolderInput class="ctx-icon" />
              <span>Move to…</span>
            </button>
            <button class="ctx-item" @click="startCopy">
              <Copy class="ctx-icon" />
              <span>Copy to…</span>
            </button>
            <button class="ctx-item" @click="duplicateItem">
              <CopyPlus class="ctx-icon" />
              <span>Duplicate</span>
            </button>
            <div class="ctx-sep" />
            <button v-if="activeEntry.type === 'file'" class="ctx-item" @click="downloadActiveItem">
              <Download class="ctx-icon" />
              <span>Download</span>
            </button>
            <button v-else class="ctx-item" @click="downloadFolderAsZip">
              <Download class="ctx-icon" />
              <span>Download as ZIP</span>
            </button>
            <button v-if="!activeIsZip" class="ctx-item" @click="compressItem">
              <Archive class="ctx-icon" />
              <span>Compress</span>
            </button>
            <button v-if="activeIsZip" class="ctx-item" @click="decompressItem">
              <ArchiveRestore class="ctx-icon" />
              <span>Decompress</span>
            </button>
            <div class="ctx-sep" />
            <button class="ctx-item ctx-item--danger" @click="deleteActiveItem">
              <Trash2 class="ctx-icon" />
              <span>Delete</span>
            </button>
          </template>

          <!-- Background menu (right-click on empty area) -->
          <template v-else>
            <button class="ctx-item" @click="triggerUpload">
              <Upload class="ctx-icon" />
              <span>Upload Files…</span>
            </button>
            <div class="ctx-sep" />
            <button class="ctx-item" @click="createFolder">
              <FolderPlus class="ctx-icon" />
              <span>New Folder</span>
            </button>
            <button class="ctx-item" @click="createFile">
              <FilePlus class="ctx-icon" />
              <span>New File</span>
            </button>
          </template>
        </div>
      </Transition>
    </Teleport>

    <!-- Move / Copy modal -->
    <MoveModal
      v-if="pickerEntry"
      :entry="pickerEntry"
      :location="location"
      :current-path="currentPath"
      :mode="pickerMode"
      @close="pickerEntry = null"
      @moved="pickerEntry = null; loadDirectory()"
    />
  </div>
</template>

<style scoped>
.browser {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.file-topbar {
  flex-shrink: 0;
  height: 44px;
  display: flex;
  align-items: center;
  padding: 0 1.25rem;
}

.back-btn {
  width: 30px;
  height: 30px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  cursor: pointer;
  background: linear-gradient(180deg,
    rgba(255, 255, 255, 0.16) 0%,
    rgba(255, 255, 255, 0.06) 100%
  );
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  box-shadow:
    0 0 0 0.5px rgba(255, 255, 255, 0.2),
    inset 0 1px 0 rgba(255, 255, 255, 0.24),
    inset 0 -0.5px 0 rgba(0, 0, 0, 0.28),
    0 4px 14px rgba(0, 0, 0, 0.32),
    0 1px 4px rgba(0, 0, 0, 0.2);
  color: rgba(255, 255, 255, 0.75);
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.2s, background 0.18s, box-shadow 0.18s, color 0.18s, transform 0.18s;
}

.back-btn.visible {
  opacity: 1;
  pointer-events: auto;
}

.back-btn:hover {
  background: linear-gradient(180deg,
    rgba(255, 255, 255, 0.22) 0%,
    rgba(255, 255, 255, 0.1) 100%
  );
  box-shadow:
    0 0 0 0.5px rgba(255, 255, 255, 0.26),
    inset 0 1px 0 rgba(255, 255, 255, 0.3),
    inset 0 -0.5px 0 rgba(0, 0, 0, 0.28),
    0 6px 20px rgba(0, 0, 0, 0.38),
    0 2px 6px rgba(0, 0, 0, 0.24);
  color: #fff;
  transform: translateY(-0.5px);
}

.back-btn:active {
  transform: translateY(0) scale(0.96);
}

.back-icon {
  width: 14px;
  height: 14px;
  stroke-width: 2.5;
  margin-right: -1px;
}

.file-area {
  flex: 1;
  overflow-y: auto;
  position: relative;
}

.drag-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  z-index: 10;
  background: rgba(0, 122, 255, 0.07);
  border: 2px dashed rgba(0, 122, 255, 0.5);
}

.drag-overlay p {
  color: #007AFF;
  font-size: 1rem;
  font-weight: 500;
}

.state-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 10rem;
  gap: 0.5rem;
}

.state-text { font-size: 0.875rem; color: #525252; }
.state-text.error { color: #ff453a; }
.state-hint { font-size: 0.75rem; color: #404040; }

.file-table {
  width: 100%;
  border-collapse: collapse;
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
.th-date { text-align: right; white-space: nowrap; }

.bottom-bar {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.65rem 1.25rem;
  border-top: 0.5px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.02);
}

.uploading-indicator {
  font-size: 0.75rem;
  color: #636366;
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
  box-shadow:
    0 8px 40px rgba(0, 0, 0, 0.55),
    0 2px 8px rgba(0, 0, 0, 0.3),
    inset 0 0.5px 0 rgba(255, 255, 255, 0.08);
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
  cursor: pointer;
  text-align: left;
  transition: background 0.08s, color 0.08s;
  font-family: inherit;
  letter-spacing: -0.01em;
}

.ctx-item:hover {
  background: #007AFF;
  color: #fff;
}

.ctx-item:hover .ctx-icon {
  opacity: 1;
}

.ctx-item--danger { color: #ff453a; }
.ctx-item--danger:hover { background: rgba(255, 69, 58, 0.18); color: #ff453a; }

.ctx-icon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  opacity: 0.7;
}

.ctx-sep {
  height: 0.5px;
  background: rgba(255, 255, 255, 0.1);
  margin: 3px 0;
}

.ctx-enter-active {
  transition: opacity 0.1s ease, transform 0.1s ease;
}
.ctx-leave-active {
  transition: opacity 0.08s ease, transform 0.08s ease;
}
.ctx-enter-from {
  opacity: 0;
  transform: scale(0.96) translateY(-4px);
}
.ctx-leave-to {
  opacity: 0;
  transform: scale(0.96) translateY(-4px);
}
</style>
