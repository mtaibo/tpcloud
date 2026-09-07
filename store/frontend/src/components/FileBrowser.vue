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
  if (!confirm(`¿Eliminar "${entry.name}"?\nEsta acción no se puede deshacer.`)) return
  const path = props.currentPath ? `${props.currentPath}/${entry.name}` : entry.name
  const params = new URLSearchParams({ path, location: props.location })
  const res = await fetch(`/api/files?${params}`, { method: 'DELETE' })
  if (res.ok) {
    loadDirectory()
  } else {
    const data = await res.json().catch(() => ({}))
    alert(data.detail || 'Error al eliminar')
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
  const name = prompt('Nombre de la nueva carpeta:')
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
    alert(data.detail || 'Error al crear carpeta')
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
      alert(data.detail || 'Error al subir archivos')
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
  <div class="flex-1 flex flex-col overflow-hidden">
    <!-- Toolbar -->
    <div
      class="flex items-center justify-between px-4 py-2 shrink-0"
      style="background: #1c1c1e; border-bottom: 1px solid rgba(255,255,255,0.08);"
    >
      <Breadcrumb
        :location="location"
        :path="currentPath"
        @navigate="(loc, p) => emit('navigate', loc, p)"
      />

      <div class="flex items-center gap-2 shrink-0 ml-4">
        <button
          @click="createFolder"
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm transition-colors"
          style="background: rgba(255,255,255,0.06); color: #fff;"
          @mouseover="(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.1)'"
          @mouseleave="(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.06)'"
        >
          <FolderPlus class="w-4 h-4" />
          <span>Carpeta</span>
        </button>

        <button
          @click="fileInput.click()"
          :disabled="uploading"
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm text-white transition-colors disabled:opacity-50"
          style="background: #007AFF;"
          @mouseover="(e) => { if (!uploading) e.currentTarget.style.background = '#0066CC' }"
          @mouseleave="(e) => e.currentTarget.style.background = '#007AFF'"
        >
          <Upload class="w-4 h-4" />
          <span>{{ uploading ? 'Subiendo…' : 'Subir' }}</span>
        </button>

        <input ref="fileInput" type="file" multiple class="hidden" @change="onFileInputChange" />
      </div>
    </div>

    <!-- File area with drag & drop -->
    <div
      class="relative flex-1 overflow-y-auto"
      @dragover="onDragOver"
      @dragleave="onDragLeave"
      @drop="onDrop"
    >
      <!-- Drag overlay -->
      <div
        v-if="isDragOver"
        class="absolute inset-0 flex items-center justify-center pointer-events-none z-10"
        style="background: rgba(0,122,255,0.08); border: 2px dashed #007AFF;"
      >
        <p class="text-lg font-medium" style="color: #007AFF;">Soltar archivos aquí</p>
      </div>

      <!-- Loading -->
      <div v-if="loading" class="flex items-center justify-center h-32">
        <span class="text-sm" style="color: #636366;">Cargando…</span>
      </div>

      <!-- Error -->
      <div v-else-if="error" class="flex items-center justify-center h-32">
        <span class="text-sm" style="color: #ff453a;">{{ error }}</span>
      </div>

      <!-- Empty -->
      <div v-else-if="!entries.length" class="flex flex-col items-center justify-center h-48 gap-2">
        <span class="text-sm" style="color: #636366;">Carpeta vacía</span>
        <span class="text-xs" style="color: #636366;">Arrastra archivos aquí o usa el botón Subir</span>
      </div>

      <!-- File table -->
      <table v-else class="w-full">
        <thead>
          <tr style="border-bottom: 1px solid rgba(255,255,255,0.08);">
            <th class="text-left px-4 py-2 text-xs font-medium w-full" style="color: #636366;">Nombre</th>
            <th class="text-right px-4 py-2 text-xs font-medium whitespace-nowrap" style="color: #636366;">Tamaño</th>
            <th class="text-right px-4 py-2 text-xs font-medium whitespace-nowrap" style="color: #636366;">Modificado</th>
            <th class="px-4 py-2 text-xs font-medium whitespace-nowrap w-20" style="color: #636366;"></th>
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
