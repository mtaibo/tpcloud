<script setup>
import { ref, watch } from 'vue'
import { FolderPlus, Upload } from 'lucide-vue-next'
import Breadcrumb from './Breadcrumb.vue'
import FileRow from './FileRow.vue'

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
    <!-- Toolbar -->
    <div class="toolbar">
      <Breadcrumb
        :location="location"
        :path="currentPath"
        @navigate="(loc, p) => emit('navigate', loc, p)"
      />

      <div class="toolbar-actions">
        <button class="btn-tool" @click="createFolder">
          <FolderPlus class="btn-icon" />
          <span>New Folder</span>
        </button>

        <button class="btn-primary" @click="fileInput.click()" :disabled="uploading">
          <Upload class="btn-icon" />
          <span>{{ uploading ? 'Uploading…' : 'Upload' }}</span>
        </button>

        <input ref="fileInput" type="file" multiple style="display:none" @change="onFileInputChange" />
      </div>
    </div>

    <!-- File area -->
    <div
      class="file-area"
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
        <span class="state-hint">Drag files here or click Upload</span>
      </div>

      <!-- File table -->
      <table v-else class="file-table">
        <thead>
          <tr>
            <th class="th-name">Name</th>
            <th class="th-size">Size</th>
            <th class="th-date">Modified</th>
            <th class="th-actions"></th>
          </tr>
        </thead>
        <tbody>
          <FileRow
            v-for="entry in entries"
            :key="entry.name"
            :entry="entry"
            @open="openItem"
            @delete="deleteItem"
            @download="downloadItem"
          />
        </tbody>
      </table>
    </div>
  </div>
</template>

<style scoped>
.browser {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.6rem 1.25rem;
  flex-shrink: 0;
  background: rgba(255, 255, 255, 0.03);
  backdrop-filter: blur(20px) saturate(140%);
  -webkit-backdrop-filter: blur(20px) saturate(140%);
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.1);
}

.toolbar-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-shrink: 0;
  margin-left: 1rem;
}

.btn-tool {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.4rem 0.8rem;
  border-radius: 10px;
  font-size: 0.8rem;
  color: #e4e4e7;
  background: linear-gradient(145deg, rgba(255, 255, 255, 0.1) 0%, rgba(255, 255, 255, 0.05) 100%);
  border: 0.5px solid rgba(255, 255, 255, 0.18);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.2), 0 2px 8px rgba(0, 0, 0, 0.2);
  cursor: pointer;
  transition: background 0.18s, box-shadow 0.18s;
  white-space: nowrap;
}

.btn-tool:hover {
  background: linear-gradient(145deg, rgba(255, 255, 255, 0.16) 0%, rgba(255, 255, 255, 0.09) 100%);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.26), 0 4px 12px rgba(0, 0, 0, 0.28);
}

.btn-primary {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.4rem 0.8rem;
  border-radius: 10px;
  font-size: 0.8rem;
  color: #fff;
  background: linear-gradient(145deg, rgba(0, 122, 255, 0.95) 0%, rgba(0, 95, 210, 1) 100%);
  border: 0.5px solid rgba(0, 122, 255, 0.5);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.22), 0 2px 10px rgba(0, 122, 255, 0.35);
  cursor: pointer;
  transition: background 0.18s, box-shadow 0.18s;
  white-space: nowrap;
}

.btn-primary:hover:not(:disabled) {
  background: linear-gradient(145deg, rgba(0, 100, 204, 1) 0%, rgba(0, 75, 180, 1) 100%);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.22), 0 4px 14px rgba(0, 122, 255, 0.45);
}

.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }

.btn-icon { width: 14px; height: 14px; flex-shrink: 0; }

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
.th-actions { width: 5rem; }
</style>
