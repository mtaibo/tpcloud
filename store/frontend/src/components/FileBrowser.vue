<script setup>
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { FolderPlus, Upload, FilePlus, Pencil, FolderInput, Copy, CopyPlus, Download, Archive, ArchiveRestore, Trash2, ChevronLeft, ChevronRight, ExternalLink, MoreHorizontal, Star, Share2, LayoutGrid, List } from 'lucide-vue-next'
import Breadcrumb from './Breadcrumb.vue'
import FileRow from './FileRow.vue'
import MoveModal from './MoveModal.vue'
import RenameModal from './RenameModal.vue'
import DeleteModal from './DeleteModal.vue'
import CreateModal from './CreateModal.vue'
import ShareModal from './ShareModal.vue'
import SharePropertiesModal from './SharePropertiesModal.vue'
import ImageViewer from './ImageViewer.vue'
import GalleryThumb from './GalleryThumb.vue'
import InlineShareBrowser from './InlineShareBrowser.vue'
import { useFavourites } from '../useFavourites.js'
import { IMAGE_EXTS, getFileIcon, getIconColor } from '../fileTypes.js'

const props = defineProps({
  user: Object,
  location: String,
  currentPath: String,
  canGoBack: Boolean,
  canGoForward: Boolean,
  viewAsAdmin: Boolean,
})

const emit = defineEmits(['navigate', 'go-back', 'go-forward', 'open-shares'])

const entries = ref([])
const loading = ref(false)
const error = ref(null)
const uploading = ref(false)
const isDragOver = ref(false)
const fileInput = ref(null)

const contextMenu = ref(null)
const moreBtn = ref(null)
const menuVisible = ref(false)
const menuPos = ref({ x: 0, y: 0 })
const activeEntry = ref(null)

const pickerEntry = ref(null)
const pickerMode = ref('move')
const renameEntry = ref(null)
const deleteEntry = ref(null)
const createMode = ref(null)
const shareEntry = ref(null)
const sharePropsEntry = ref(null)
const imageViewEntry = ref(null)
const inlineShareEntry = ref(null)
const galleryMode = ref(localStorage.getItem('gallery-mode') === '1')

function toggleGallery() {
  galleryMode.value = !galleryMode.value
  localStorage.setItem('gallery-mode', galleryMode.value ? '1' : '0')
}

function entryIsImage(entry) {
  const ext = entry.name.split('.').pop()?.toLowerCase() ?? ''
  return IMAGE_EXTS.has(ext)
}

function galleryThumbSrc(entry) {
  const path = props.currentPath ? `${props.currentPath}/${entry.name}` : entry.name
  const params = new URLSearchParams({ path, location: props.location })
  return `/api/files/thumbnail?${params}`
}

function galleryEntryIcon(entry) { return getFileIcon(entry) }
function galleryEntryColor(entry) { return getIconColor(entry) }

const imageViewSrc = computed(() => {
  if (!imageViewEntry.value) return ''
  const path = props.currentPath
    ? `${props.currentPath}/${imageViewEntry.value.name}`
    : imageViewEntry.value.name
  const params = new URLSearchParams({ path, location: props.location })
  return `/api/files/view?${params}`
})

const { add: addFav } = useFavourites()

const activeIsZip = computed(() =>
  activeEntry.value?.name?.toLowerCase().endsWith('.zip') ?? false
)

const mobileFolderName = computed(() => {
  if (!props.currentPath) return 'Files'
  const parts = props.currentPath.split('/').filter(Boolean)
  const last = parts[parts.length - 1]
  if (props.currentPath === `users/${props.user.email}`) return 'Home'
  if (last === 'shared') return 'Shared'
  return last.charAt(0).toUpperCase() + last.slice(1)
})

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
    renameEntry.value = null
    deleteEntry.value = null
    createMode.value = null
    shareEntry.value = null
    sharePropsEntry.value = null
    imageViewEntry.value = null
    inlineShareEntry.value = null
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
  entries.value = []
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
    let real = data.entries

    if (props.location === 'external' && props.currentPath === 'shared') {
      const sres = await fetch('/api/shares')
      if (sres.ok) {
        const sdata = await sres.json()
        const virtual = sdata.map(s => ({
          type: 'share-link',
          name: s.path.split('/').filter(Boolean).at(-1) || s.token,
          token: s.token,
          url: s.url,
          share: s,
          size: null,
          modified: null,
        }))
        real = [...virtual, ...real]
      }
    }

    entries.value = real
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

async function viewItem(entry) {
  if (entry.type === 'share-link') {
    inlineShareEntry.value = entry
    return
  }
  const path = props.currentPath ? `${props.currentPath}/${entry.name}` : entry.name
  const ext = entry.name.split('.').pop()?.toLowerCase() ?? ''
  if (IMAGE_EXTS.has(ext)) {
    imageViewEntry.value = entry
    return
  }
  const res = await fetch('/api/files/token', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path, location: props.location }),
  })
  const data = await res.json()
  window.open(`/api/files/open/${data.token}`, '_blank')
}

function downloadItem(entry) {
  const path = props.currentPath ? `${props.currentPath}/${entry.name}` : entry.name
  const params = new URLSearchParams({ path, location: props.location })
  const a = document.createElement('a')
  a.href = `/api/files/download?${params}`
  a.download = entry.name
  a.click()
}

function createFolder() {
  hideMenu()
  createMode.value = 'folder'
}

function createFile() {
  hideMenu()
  createMode.value = 'file'
}

async function onCreated(name) {
  const mode = createMode.value
  createMode.value = null
  const path = props.currentPath ? `${props.currentPath}/${name}` : name
  const endpoint = mode === 'folder' ? '/api/files/mkdir' : '/api/files/touch'
  const res = await fetch(endpoint, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path, location: props.location }),
  })
  if (res.ok) {
    loadDirectory()
  } else {
    const data = await res.json().catch(() => ({}))
    alert(data.detail || `Error creating ${mode}`)
  }
}

function renameItem() {
  renameEntry.value = activeEntry.value
  hideMenu()
}

async function onRenamed(newName) {
  const entry = renameEntry.value
  renameEntry.value = null
  const path = props.currentPath ? `${props.currentPath}/${entry.name}` : entry.name
  const res = await fetch('/api/files/rename', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path, new_name: newName, location: props.location }),
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

function openActiveItem() {
  const entry = activeEntry.value
  hideMenu()
  viewItem(entry)
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

function deleteActiveItem() {
  deleteEntry.value = activeEntry.value
  hideMenu()
}

async function onDeleteConfirmed() {
  const entry = deleteEntry.value
  deleteEntry.value = null
  await deleteItem(entry)
}

function showMoreMenu() {
  activeEntry.value = null
  menuPos.value = { x: -9999, y: -9999 }
  menuVisible.value = true
  nextTick(() => {
    if (!contextMenu.value || !moreBtn.value) return
    const btn = moreBtn.value.getBoundingClientRect()
    const menu = contextMenu.value.getBoundingClientRect()
    const x = Math.max(8, btn.right - menu.width)
    const y = Math.min(btn.bottom + 8, window.innerHeight - menu.height - 8)
    menuPos.value = { x, y }
  })
}

function shareActiveItem() {
  shareEntry.value = activeEntry.value
  hideMenu()
}

function openShareInBrowser() {
  window.open(activeEntry.value.url, '_blank')
  hideMenu()
}

function copyShareLink() {
  navigator.clipboard.writeText(activeEntry.value.url)
  hideMenu()
}

function editShareProps() {
  sharePropsEntry.value = activeEntry.value
  hideMenu()
}

async function deleteShareEntry() {
  const entry = activeEntry.value
  hideMenu()
  await fetch(`/api/shares/${entry.token}`, { method: 'DELETE' })
  loadDirectory()
}

function openManageLinks() {
  hideMenu()
  emit('open-shares')
}

function addCurrentToFavourites() {
  addFav(props.location, props.currentPath, props.user.email)
  hideMenu()
}

function addEntryToFavourites() {
  const entry = activeEntry.value
  const path = props.currentPath ? `${props.currentPath}/${entry.name}` : entry.name
  addFav(props.location, path, props.user.email)
  hideMenu()
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

    <!-- Navigation buttons -->
    <div class="file-topbar">
      <div class="nav-pill">
        <button
          class="nav-btn"
          :disabled="!canGoBack"
          @click="emit('go-back')"
          title="Back"
        >
          <ChevronLeft class="nav-icon" />
        </button>
        <div class="nav-divider" />
        <button
          class="nav-btn"
          :disabled="!canGoForward"
          @click="emit('go-forward')"
          title="Forward"
        >
          <ChevronRight class="nav-icon" />
        </button>
      </div>
      <span class="mobile-folder-name">{{ mobileFolderName }}</span>
      <button class="more-btn" :class="{ 'more-btn--active': galleryMode }" @click="toggleGallery" :title="galleryMode ? 'List view' : 'Gallery view'">
        <LayoutGrid v-if="!galleryMode" class="more-icon" />
        <List v-else class="more-icon" />
      </button>
      <button ref="moreBtn" class="more-btn" @click.stop="showMoreMenu">
        <MoreHorizontal class="more-icon" />
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

      <!-- Gallery grid -->
      <div v-else-if="galleryMode" class="gallery-grid">
        <div
          v-for="entry in entries"
          :key="entry.name"
          class="gallery-card"
          @click="entry.type === 'directory' ? openItem(entry) : null"
          @dblclick="(entry.type === 'file' || entry.type === 'share-link') ? viewItem(entry) : null"
          @contextmenu.stop="showMenuForEntry(entry, $event)"
        >
          <div class="gallery-thumb">
            <GalleryThumb
              v-if="entry.type === 'file' && entryIsImage(entry)"
              :src="galleryThumbSrc(entry)"
              :alt="entry.name"
            />
            <component
              v-else
              :is="galleryEntryIcon(entry)"
              class="gallery-icon"
              :style="{ color: galleryEntryColor(entry) }"
            />
          </div>
          <span class="gallery-name">{{ entry.name }}</span>
        </div>
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
            @view="viewItem"
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
        :view-as-admin="viewAsAdmin"
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
          <!-- Share-link entry menu -->
          <template v-if="activeEntry?.type === 'share-link'">
            <button class="ctx-item" @click="openShareInBrowser">
              <ExternalLink class="ctx-icon" />
              <span>Open in Browser</span>
            </button>
            <button class="ctx-item" @click="copyShareLink">
              <Copy class="ctx-icon" />
              <span>Copy Link</span>
            </button>
            <div class="ctx-sep" />
            <button class="ctx-item" @click="editShareProps">
              <Pencil class="ctx-icon" />
              <span>Properties…</span>
            </button>
            <div class="ctx-sep" />
            <button class="ctx-item ctx-item--danger" @click="deleteShareEntry">
              <Trash2 class="ctx-icon" />
              <span>Delete</span>
            </button>
          </template>

          <!-- Regular entry menu (right-click on a file/folder) -->
          <template v-else-if="activeEntry">
            <button v-if="activeEntry.type === 'file'" class="ctx-item" @click="openActiveItem">
              <ExternalLink class="ctx-icon" />
              <span>Open</span>
            </button>
            <button class="ctx-item" @click="renameItem">
              <Pencil class="ctx-icon" />
              <span>Rename</span>
            </button>
            <button v-if="activeEntry.type === 'directory'" class="ctx-item" @click="addEntryToFavourites">
              <Star class="ctx-icon" />
              <span>Add to Favorites</span>
            </button>
            <button v-if="activeEntry.type === 'directory'" class="ctx-item" @click="shareActiveItem">
              <Share2 class="ctx-icon" />
              <span>Share…</span>
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

          <!-- Background menu (right-click on empty area / ··· button) -->
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
            <div class="ctx-sep" />
            <button class="ctx-item" @click="addCurrentToFavourites">
              <Star class="ctx-icon" />
              <span>Add to Favorites</span>
            </button>
            <button class="ctx-item" @click="openManageLinks">
              <Share2 class="ctx-icon" />
              <span>Manage links</span>
            </button>
          </template>
        </div>
      </Transition>
    </Teleport>

    <!-- Delete modal -->
    <DeleteModal
      v-if="deleteEntry"
      :entry="deleteEntry"
      @close="deleteEntry = null"
      @confirmed="onDeleteConfirmed"
    />

    <!-- Create modal -->
    <CreateModal
      v-if="createMode"
      :type="createMode"
      @close="createMode = null"
      @created="onCreated"
    />

    <!-- Rename modal -->
    <RenameModal
      v-if="renameEntry"
      :entry="renameEntry"
      @close="renameEntry = null"
      @renamed="onRenamed"
    />

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

    <!-- Share modal -->
    <ShareModal
      v-if="shareEntry"
      :entry="shareEntry"
      :location="location"
      :current-path="currentPath"
      @close="shareEntry = null"
    />

    <!-- Image viewer -->
    <ImageViewer
      v-if="imageViewEntry"
      :src="imageViewSrc"
      :name="imageViewEntry.name"
      @close="imageViewEntry = null"
    />

    <!-- Share properties modal -->
    <SharePropertiesModal
      v-if="sharePropsEntry"
      :share="sharePropsEntry.share"
      @close="sharePropsEntry = null"
      @updated="sharePropsEntry = null; loadDirectory()"
      @deleted="sharePropsEntry = null; loadDirectory()"
    />

    <!-- Inline share browser -->
    <InlineShareBrowser
      v-if="inlineShareEntry"
      :entry="inlineShareEntry"
      @close="inlineShareEntry = null"
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
  display: flex;
  align-items: center;
  padding: 10px 1.25rem;
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

.nav-btn:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.08);
}

.nav-btn:active:not(:disabled) {
  background: rgba(255, 255, 255, 0.04);
}

.nav-btn:disabled {
  color: rgba(255, 255, 255, 0.2);
  cursor: default;
}

.nav-divider {
  width: 0.5px;
  height: 14px;
  background: rgba(255, 255, 255, 0.15);
  flex-shrink: 0;
}

.nav-icon {
  width: 16px;
  height: 16px;
  stroke-width: 2.5;
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

.mobile-folder-name {
  display: block;
  font-size: 1rem;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.75);
  margin-left: 0.75rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
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
}

.more-btn:hover {
  background: rgba(255, 255, 255, 0.10);
}

.more-btn:active {
  background: rgba(255, 255, 255, 0.04);
}

.more-btn--active {
  background: rgba(255, 255, 255, 0.14);
  border-color: rgba(255, 255, 255, 0.22);
}

.more-icon {
  width: 16px;
  height: 16px;
}

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

.gallery-card:hover {
  background: rgba(255, 255, 255, 0.06);
}

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

.gallery-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.gallery-icon {
  width: 44px;
  height: 44px;
}

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
    padding-bottom: 110px;
  }

  .mobile-folder-name {
    display: block;
    flex: 1;
    min-width: 0;
    font-size: 1.25rem;
    margin-left: 1.5rem;
  }

  .nav-pill {
    flex-shrink: 0;
  }

  .bottom-bar { display: none; }
  .th-size, .th-date { display: none; }

  .gallery-grid {
    grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
    gap: 8px;
    padding: 12px 1rem;
  }

  .gallery-thumb {
    width: 88px;
    height: 88px;
  }
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

@media (max-width: 767px) {
  .ctx-menu {
    min-width: 260px;
    background: rgba(20, 20, 22, 0.97);
    backdrop-filter: blur(60px) saturate(220%);
    -webkit-backdrop-filter: blur(60px) saturate(220%);
  }
  .ctx-item { font-size: 15px; padding: 8px 12px; gap: 12px; }
  .ctx-icon { width: 16px; height: 16px; }
  .ctx-sep { margin: 4px 0; }
}
</style>
