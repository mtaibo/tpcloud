<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { ChevronLeft, ChevronRight, Download, ExternalLink, Eye, FolderDown, ImageDown, LayoutGrid, List, MoreHorizontal } from 'lucide-vue-next'
import Spinner from '@ui/Spinner.vue'
import { openMenu } from '../menu.js'
import { kindName } from '../fileTypes.js'
import { formatDate, formatSize } from '../utils/formatSize.js'
import { canSaveToPhotos, downloadLink, isMedia, isVideo, shareFileUrls } from '../media.js'
import { vLongPress } from '../longPress.js'
import FileIcon from './FileIcon.vue'
import ShareGalleryThumb from './ShareGalleryThumb.vue'
import MediaViewer from './MediaViewer.vue'
import SaveToPhotos from './SaveToPhotos.vue'

// A shared folder as its visitors see it: the folder up top with what they can do with it,
// subfolders and other files in a list like Files, photos and videos in a grid like Photos.
const props = defineProps({
  token: String,
  shareInfo: Object,
  sessionToken: String,
})

const currentPath = ref('')
const entries = ref([])
const loading = ref(true)
const error = ref('')
const scrolled = ref(false)
// Until the visitor picks a view, a folder that's mostly photos and videos opens as a grid.
const storedView = localStorage.getItem('share-view') ?? (localStorage.getItem('share-gallery-mode') === '1' ? 'grid' : null)
const view = ref(storedView || 'grid')
const urls = shareFileUrls(props.token, props.sessionToken, () => currentPath.value)

const history = ref([''])
const historyIndex = ref(0)

const viewerIndex = ref(null)
const saveItems = ref(null)

const rootLabel = computed(() => props.shareInfo?.label || props.token)
const crumbs = computed(() => currentPath.value.split('/').filter(Boolean))
const title = computed(() => crumbs.value.at(-1) || rootLabel.value)
const parentPath = computed(() => (crumbs.value.length ? crumbs.value.slice(0, -1).join('/') : null))

const collator = new Intl.Collator('en', { numeric: true, sensitivity: 'base' })
const sorted = computed(() => [...entries.value].sort((a, b) => {
  const da = a.type === 'directory'
  const db = b.type === 'directory'
  return da !== db ? (da ? -1 : 1) : collator.compare(a.name, b.name)
}))
const folders = computed(() => sorted.value.filter(e => e.type === 'directory'))
const media = computed(() => sorted.value.filter(isMedia))
const others = computed(() => sorted.value.filter(e => e.type === 'file' && !isMedia(e)))
const files = computed(() => sorted.value.filter(e => e.type === 'file'))

const summary = computed(() => {
  const parts = []
  const photos = media.value.filter(e => !isVideo(e)).length
  const videos = media.value.length - photos
  if (folders.value.length) parts.push(folders.value.length === 1 ? '1 folder' : `${folders.value.length} folders`)
  if (photos) parts.push(photos === 1 ? '1 photo' : `${photos} photos`)
  if (videos) parts.push(videos === 1 ? '1 video' : `${videos} videos`)
  if (others.value.length) parts.push(others.value.length === 1 ? '1 file' : `${others.value.length} files`)
  const bytes = files.value.reduce((n, e) => n + (e.size || 0), 0)
  return [parts.join(', ') || 'Empty', bytes ? formatSize(bytes) : ''].filter(Boolean).join(' · ')
})

function authHeaders() {
  return props.sessionToken ? { 'X-Share-Session': props.sessionToken } : {}
}

function setView(v) {
  view.value = v
  localStorage.setItem('share-view', v)
}

function zipUrl(path) {
  const params = new URLSearchParams({ path })
  if (props.sessionToken) params.set('session', props.sessionToken)
  return `/share/api/share/${props.token}/files/download-zip?${params}`
}

async function loadDir(path) {
  loading.value = true
  error.value = ''
  try {
    const res = await fetch(`/share/api/share/${props.token}/files/list?${new URLSearchParams({ path })}`, { headers: authHeaders() })
    if (!res.ok) { error.value = "This folder couldn't be opened."; return }
    const data = await res.json()
    entries.value = data.entries
    currentPath.value = path
    if (!storedView && data.entries.length) view.value = data.entries.filter(isMedia).length * 2 >= data.entries.length ? 'grid' : 'list'
    document.querySelector('.share-scroll')?.scrollTo({ top: 0 })
  } catch {
    error.value = 'No connection. Try again.'
  } finally {
    loading.value = false
  }
}

function navigateTo(path) {
  history.value = history.value.slice(0, historyIndex.value + 1)
  history.value.push(path)
  historyIndex.value++
  loadDir(path)
}

function openFolder(entry) {
  navigateTo(currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name)
}

function back() {
  if (parentPath.value !== null) navigateTo(parentPath.value)
}

function activate(entry) {
  if (entry.type === 'directory') return openFolder(entry)
  if (isMedia(entry)) viewerIndex.value = media.value.indexOf(entry)
  else openInTab(entry)
}

// Downloads go straight to the browser's download manager instead of being read into memory
// first, which with phone videos meant hundreds of megabytes held by the page before anything
// was saved.
function download(entry) {
  if (entry.type === 'directory') {
    const path = currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name
    downloadLink(zipUrl(path), `${entry.name}.zip`)
  } else {
    downloadLink(urls.download(entry), entry.name)
  }
}

function downloadZip() {
  downloadLink(zipUrl(currentPath.value), `${title.value}.zip`)
}

// One download per file, spaced out so the browser doesn't drop any. It asks once whether the
// page may download several files.
async function downloadAll() {
  for (const entry of files.value) {
    downloadLink(urls.download(entry), entry.name)
    await new Promise(resolve => setTimeout(resolve, 600))
  }
}

async function openInTab(entry) {
  const path = currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name
  const tab = window.open('', '_blank')
  if (!tab) return
  try {
    const tokenRes = await fetch(`/share/api/share/${props.token}/files/token`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ path }),
    })
    if (tokenRes.ok) {
      const { token: fileToken } = await tokenRes.json()
      const infoRes = await fetch(`/share/api/open/${fileToken}/info`)
      if ((infoRes.ok ? infoRes.headers.get('content-type') || '' : '').includes('application/json')) {
        tab.location.href = `/share/${fileToken}`
        return
      }
    }
  } catch { /* fall back to the raw file */ }
  tab.location.href = urls.view(entry)
}

function entryMenu(entry, at) {
  openMenu(at, [
    entry.type === 'file' && { label: 'Open', icon: Eye, action: () => activate(entry) },
    entry.type === 'file' && !isMedia(entry) && { label: 'Open in New Tab', icon: ExternalLink, action: () => openInTab(entry) },
    canSaveToPhotos && isMedia(entry) && { label: 'Save to Photos', icon: ImageDown, action: () => { saveItems.value = [entry] } },
    { label: entry.type === 'directory' ? 'Download as ZIP' : 'Download', icon: Download, action: () => download(entry) },
  ], at instanceof Element ? '' : entry.name)
}

function folderMenu(at) {
  openMenu(at, [
    { label: 'Gallery', icon: LayoutGrid, checked: view.value === 'grid', action: () => setView('grid') },
    { label: 'List', icon: List, checked: view.value === 'list', action: () => setView('list') },
    { separator: true },
    canSaveToPhotos && media.value.length && { label: 'Save All to Photos', icon: ImageDown, action: () => { saveItems.value = media.value } },
    files.value.length > 1 && { label: 'Download Files One by One', icon: Download, action: downloadAll },
    { label: 'Download All (ZIP)', icon: FolderDown, action: downloadZip },
  ])
}

function onContext(entry, e) {
  e.preventDefault()
  e.stopPropagation()
  entryMenu(entry, e)
}

function onPress(entry, point) {
  entryMenu(entry, { clientX: point.x, clientY: point.y })
}

function onScroll(e) { scrolled.value = e.target.scrollTop > 60 }

function onKey(e) {
  if (viewerIndex.value !== null || saveItems.value || document.querySelector('.menu')) return
  if ((e.key === 'Backspace' || (e.metaKey && e.key === 'ArrowUp')) && parentPath.value !== null) back()
}

onMounted(() => {
  loadDir('')
  document.addEventListener('keydown', onKey)
})
onUnmounted(() => document.removeEventListener('keydown', onKey))
</script>

<template>
  <div class="share">
    <header class="bar" :class="{ scrolled }">
      <button v-if="parentPath !== null" type="button" class="round glass-control" aria-label="Back" @click="back">
        <ChevronLeft :size="22" :stroke-width="2.3" />
      </button>
      <span v-else class="brand"><FileIcon :entry="{ type: 'share-link', name: '' }" :size="26" /> TPCloud</span>
      <span class="bar-title">{{ title }}</span>
      <div class="bar-actions">
        <div class="segmented-pill glass-control desktop" role="group" aria-label="View">
          <button type="button" :class="{ on: view === 'grid' }" aria-label="Gallery" @click="setView('grid')"><LayoutGrid :size="17" /></button>
          <button type="button" :class="{ on: view === 'list' }" aria-label="List" @click="setView('list')"><List :size="18" /></button>
        </div>
        <button type="button" class="round glass-control" aria-label="More" @click="folderMenu($event.currentTarget)"><MoreHorizontal :size="21" /></button>
      </div>
    </header>

    <main class="share-scroll" @scroll.passive="onScroll" @contextmenu.prevent="folderMenu($event)">
      <div class="inner">
        <section class="hero">
          <FileIcon :entry="{ type: 'directory', name: title }" :size="104" class="hero-icon" />
          <h1 class="title-1 hero-title">{{ title }}</h1>
          <nav v-if="crumbs.length" class="crumbs subheadline">
            <button type="button" @click="navigateTo('')">{{ rootLabel }}</button>
            <template v-for="(c, i) in crumbs.slice(0, -1)" :key="i">
              <ChevronRight :size="13" :stroke-width="2.4" />
              <button type="button" @click="navigateTo(crumbs.slice(0, i + 1).join('/'))">{{ c }}</button>
            </template>
          </nav>
          <p class="subheadline secondary">{{ loading ? 'Loading…' : summary }}</p>
          <div v-if="!loading && entries.length" class="hero-actions">
            <button type="button" class="btn btn-prominent" @click="downloadZip"><Download :size="19" :stroke-width="2.2" /> Download All</button>
            <button v-if="canSaveToPhotos && media.length" type="button" class="btn btn-glass" @click="saveItems = media"><ImageDown :size="19" /> Save to Photos</button>
          </div>
        </section>

        <div v-if="loading" class="state"><Spinner :size="26" /></div>
        <div v-else-if="error" class="state">
          <p class="headline">{{ error }}</p>
          <button type="button" class="btn btn-bordered small" @click="loadDir(currentPath)">Try Again</button>
        </div>
        <div v-else-if="!entries.length" class="state">
          <p class="headline">This folder is empty</p>
        </div>

        <!-- Grid: folders in a list, then photos and videos like Photos, then other files -->
        <template v-else-if="view === 'grid'">
          <section v-if="folders.length" class="block">
            <h2 class="section-header">Folders</h2>
            <div class="card">
              <button
                v-for="f in folders"
                :key="f.name"
                v-long-press="p => onPress(f, p)"
                type="button"
                class="row file-row"
                @click="openFolder(f)"
                @contextmenu="onContext(f, $event)"
              >
                <FileIcon :entry="f" :size="40" />
                <span class="row-body">
                  <span class="row-title">{{ f.name }}</span>
                  <span class="row-subtitle small">Folder</span>
                </span>
                <ChevronRight :size="18" class="chev" />
              </button>
            </div>
          </section>

          <section v-if="media.length" class="block">
            <h2 v-if="folders.length || others.length" class="section-header">Photos & Videos</h2>
            <div class="photo-grid">
              <button
                v-for="(m, i) in media"
                :key="m.name"
                v-long-press="p => onPress(m, p)"
                type="button"
                class="photo"
                :aria-label="m.name"
                @click="viewerIndex = i"
                @auxclick.prevent="$event.button === 1 && openInTab(m)"
                @contextmenu="onContext(m, $event)"
              >
                <ShareGalleryThumb :src="urls.thumbnail(m)" :alt="m.name" :is-video="isVideo(m)" :duration="m.duration" />
              </button>
            </div>
          </section>

          <section v-if="others.length" class="block">
            <h2 v-if="folders.length || media.length" class="section-header">Files</h2>
            <div class="card">
              <button
                v-for="f in others"
                :key="f.name"
                v-long-press="p => onPress(f, p)"
                type="button"
                class="row file-row"
                @click="activate(f)"
                @contextmenu="onContext(f, $event)"
              >
                <FileIcon :entry="f" :size="40" />
                <span class="row-body">
                  <span class="row-title">{{ f.name }}</span>
                  <span class="row-subtitle small">{{ kindName(f) }} · {{ formatSize(f.size) }}</span>
                </span>
                <span class="dl" role="button" tabindex="0" aria-label="Download" @click.stop="download(f)" @keydown.enter.stop="download(f)"><Download :size="19" /></span>
              </button>
            </div>
          </section>
        </template>

        <!-- List: everything in one inset list -->
        <section v-else class="block">
          <div class="card">
            <button
              v-for="e in sorted"
              :key="e.name"
              v-long-press="p => onPress(e, p)"
              type="button"
              class="row file-row"
              @click="activate(e)"
              @auxclick.prevent="$event.button === 1 && e.type === 'file' && openInTab(e)"
              @contextmenu="onContext(e, $event)"
            >
              <span v-if="isMedia(e)" class="mini-thumb"><ShareGalleryThumb :src="urls.thumbnail(e, 160)" :alt="e.name" :is-video="isVideo(e)" /></span>
              <FileIcon v-else :entry="e" :size="40" />
              <span class="row-body">
                <span class="row-title">{{ e.name }}</span>
                <span class="row-subtitle small">{{ e.type === 'directory' ? 'Folder' : `${formatDate(e.modified)} · ${formatSize(e.size)}` }}</span>
              </span>
              <ChevronRight v-if="e.type === 'directory'" :size="18" class="chev" />
              <span v-else class="dl" role="button" tabindex="0" aria-label="Download" @click.stop="download(e)" @keydown.enter.stop="download(e)"><Download :size="19" /></span>
            </button>
          </div>
        </section>

        <footer class="foot footnote tertiary">Shared with TPCloud</footer>
      </div>
    </main>

    <MediaViewer
      v-if="viewerIndex !== null"
      :items="media"
      :start-index="viewerIndex"
      :urls="urls"
      @close="viewerIndex = null"
      @download="download"
      @save="e => (saveItems = [e])"
    />
    <SaveToPhotos v-if="saveItems" :items="saveItems" :download-url="urls.download" @close="saveItems = null" />
  </div>
</template>

<style scoped>
.share { height: 100dvh; display: flex; flex-direction: column; position: relative; }
.bar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  z-index: 20;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  height: calc(60px + env(safe-area-inset-top));
  padding: env(safe-area-inset-top) 14px 0;
}
.bar::before {
  content: "";
  position: absolute;
  inset: 0 0 -22px;
  z-index: -1;
  background: linear-gradient(to bottom, color-mix(in srgb, var(--bg) 85%, transparent) 50%, transparent);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  mask-image: linear-gradient(to bottom, #000 60%, transparent);
  -webkit-mask-image: linear-gradient(to bottom, #000 60%, transparent);
  opacity: 0;
  transition: opacity 0.2s;
}
.bar.scrolled::before { opacity: 1; }
.brand { display: flex; align-items: center; gap: 8px; font-size: 15px; font-weight: 700; padding-left: 4px; }
.bar-title {
  position: absolute;
  left: 50%;
  transform: translate(-50%, 4px);
  max-width: 50%;
  font-size: 17px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  opacity: 0;
  transition: opacity 0.2s, transform 0.2s;
}
.bar.scrolled .bar-title { opacity: 1; transform: translate(-50%, 0); }
.bar-actions { display: flex; gap: 8px; }
.glass-control {
  background: var(--glass-strong);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.08), 0 0 0 0.5px rgba(0, 0, 0, 0.08), inset 0 0.5px 0 var(--glass-edge);
  color: var(--label);
}
@media (prefers-color-scheme: dark) { .glass-control { box-shadow: 0 2px 10px rgba(0, 0, 0, 0.35), 0 0 0 0.5px rgba(255, 255, 255, 0.1); } }
.round { width: 44px; height: 44px; display: grid; place-items: center; border-radius: 50%; transition: transform 0.2s var(--spring); }
.round:active { transform: scale(0.92); }
.segmented-pill { display: flex; align-items: center; height: 44px; padding: 0 4px; border-radius: 999px; }
.segmented-pill button { width: 42px; height: 36px; display: grid; place-items: center; border-radius: 999px; }
.segmented-pill button.on { background: var(--fill-2); }
@media (max-width: 600px) { .desktop { display: none; } }

.share-scroll { flex: 1; overflow-y: auto; overscroll-behavior: contain; }
.inner { max-width: 1080px; margin: 0 auto; padding: calc(70px + env(safe-area-inset-top)) 20px calc(40px + env(safe-area-inset-bottom)); }

.hero { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 10px 0 34px; text-align: center; }
.hero-icon { filter: drop-shadow(0 10px 24px rgba(10, 132, 255, 0.25)); margin-bottom: 4px; }
.hero-title { overflow-wrap: anywhere; max-width: 680px; }
.crumbs { display: flex; flex-wrap: wrap; align-items: center; justify-content: center; gap: 4px; color: var(--label-3); }
.crumbs button { color: var(--accent); }
.hero-actions { display: flex; flex-wrap: wrap; justify-content: center; gap: 10px; margin-top: 14px; }
.hero-actions .btn { gap: 8px; }

.state { display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 40px 20px; text-align: center; }
.block { margin-bottom: 30px; }
.block .section-header { padding-left: 4px; }


.photo-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 4px; }
@media (max-width: 600px) { .photo-grid { grid-template-columns: repeat(3, 1fr); gap: 2px; margin: 0 -20px; } .photo { border-radius: 0; } }
.photo { aspect-ratio: 1; border-radius: 8px; background: var(--fill-3); overflow: hidden; transition: opacity 0.15s, transform 0.25s; -webkit-touch-callout: none; }
.photo:hover { opacity: 0.88; }
.photo.long-pressing { transform: scale(0.95); }

.file-row { --inset: 74px; cursor: pointer; padding-top: 10px; padding-bottom: 10px; -webkit-touch-callout: none; }
.file-row.long-pressing { background: var(--cell-pressed); }
.mini-thumb { width: 40px; height: 40px; flex-shrink: 0; border-radius: 8px; overflow: hidden; background: var(--fill-3); }
.mini-thumb :deep(.thumb-duration) { display: none; }
.dl, .chev { flex-shrink: 0; color: var(--accent); }
.chev { color: var(--label-3); }
.dl { width: 36px; height: 36px; display: grid; place-items: center; border-radius: 50%; }
.dl:hover { background: var(--fill-3); }

.foot { text-align: center; padding: 18px 0 0; }
</style>
