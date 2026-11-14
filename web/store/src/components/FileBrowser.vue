<script setup>
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import {
  Archive, ArchiveRestore, ArrowUpDown, ChevronLeft, ChevronRight, Copy, CopyPlus, Download, ExternalLink,
  Eye, FilePlus, FolderInput, FolderPlus, LayoutGrid, Link, List, MoreHorizontal, Palette, Pencil, Plus,
  Search, Share, Star, Trash2, Upload, X,
} from 'lucide-vue-next'
import Spinner from '../ui/Spinner.vue'
import { confirmDestructive, prompt, toast } from '../ui/dialog.js'
import { openMenu } from '../menu.js'
import { useFavourites } from '../useFavourites.js'
import { useTransfers } from '../useTransfers.js'
import { useCompression } from '../useCompression.js'
import { isMedia, kindName, kindOf } from '../fileTypes.js'
import { formatDate, formatSize } from '../utils/formatSize.js'
import {
  canGoBack, canGoForward, folderTitle, goBack, goForward, homePath, navigate, parentOf, query,
  setSort, setView, store,
} from '../store.js'
import ItemIcon from './ItemIcon.vue'
import FileIcon from './FileIcon.vue'
import QuickLook from './QuickLook.vue'
import { shareMediaUrls, storeMediaUrls } from '../mediaUrls.js'
import MoveSheet from './MoveSheet.vue'
import ShareSheet from './ShareSheet.vue'
import IconSheet from './IconSheet.vue'
import { t as tr } from '../ui/i18n.js'
import { apiError } from '../apiErrors.js'

const phoneQuery = window.matchMedia('(max-width: 767px)')
const phone = ref(phoneQuery.matches)
const onPhoneChange = e => { phone.value = e.matches }

const entries = ref([])
const loading = ref(false)
const error = ref(null)
const isDragOver = ref(false)
const fileInput = ref(null)
const searchInput = ref(null)
const selected = ref(new Set())
const anchor = ref(null)
const scrolled = ref(false)

const { startUpload, resumeUpload, cancelPendingUpload, uploads,
        add: addTransfer, update: updateTransfer, setTotal: setTransferTotal,
        complete: completeTransfer, fail: failTransfer } = useTransfers()
const { startDecompress } = useCompression()
const { add: addFav, updateIconName: updateFavIconName } = useFavourites()

// Sheets and viewers
const moveEntry = ref(null)
const moveMode = ref('move')
const shareEntry = ref(null)
const shareProps = ref(null)
const iconEntry = ref(null)
const lookEntry = ref(null)

// Browsing inside a share link from the Compartido folder
const shareMode = ref(null) // { token, label, path, history, historyIndex } | null
let loadController = null

function joinPath(base, name) {
  return base ? `${base}/${name}` : name
}

// ---- Listing ----

// Server entries plus the uploads running into this folder, shown in place with their progress.
const displayEntries = computed(() => {
  const base = [...entries.value]
  const existing = new Set(base.map(e => e.name))
  for (const t of uploads.value) {
    if (t.status === 'active' && t.name && t.path === store.path && t.location === store.location && !existing.has(t.name)) {
      base.push({ name: t.name, type: 'upload-pending', size: t.totalSize, modified: Date.now() / 1000, upload_id: t.uploadId, bytes_received: t.loaded })
    }
  }
  const doneIds = new Set(uploads.value.filter(t => t.status === 'done' && t.uploadId).map(t => t.uploadId))
  return base
    .filter(e => e.type !== 'upload-pending' || !doneIds.has(e.upload_id))
    .map(e => {
      if (e.type !== 'upload-pending') return e
      const active = uploads.value.find(t => t.uploadId === e.upload_id)
      return active ? { ...e, bytes_received: active.loaded } : e
    })
})

const collator = new Intl.Collator(['es', 'en'], { numeric: true, sensitivity: 'base' })
const visible = computed(() => {
  const q = query.value.trim().toLowerCase()
  const list = q ? displayEntries.value.filter(e => e.name.toLowerCase().includes(q)) : [...displayEntries.value]
  const dir = store.sortAsc ? 1 : -1
  const key = store.sortKey
  const isDir = e => e.type === 'directory' || e.type === 'share-link'
  return list.sort((a, b) => {
    if (isDir(a) !== isDir(b)) return isDir(a) ? -1 : 1
    let r = 0
    if (key === 'size') r = (a.size ?? -1) - (b.size ?? -1)
    else if (key === 'modified') r = (a.modified ?? 0) - (b.modified ?? 0)
    else if (key === 'kind') r = collator.compare(kindName(a), kindName(b))
    if (r === 0) r = collator.compare(a.name, b.name)
    return r * dir
  })
})

const media = computed(() => visible.value.filter(e => e.type === 'file' && isMedia(e)))

// ---- Search: this folder and every folder inside it, on the server ----

const searchResults = ref(null) // null while the first answer for this query is on its way
const searchLoading = ref(false)
const searchTruncated = ref(false)
const searching = computed(() => !!query.value.trim() && !shareMode.value)
let searchTimer = null
let searchController = null

watch([query, () => store.path, () => store.location, shareMode], () => {
  clearTimeout(searchTimer)
  searchController?.abort()
  const q = query.value.trim()
  if (!q || shareMode.value) {
    searchResults.value = null
    searchLoading.value = false
    return
  }
  searchLoading.value = true
  searchTimer = setTimeout(async () => {
    const controller = new AbortController()
    searchController = controller
    try {
      const params = new URLSearchParams({ q, path: store.path, location: store.location })
      const res = await fetch(`/store/api/files/search?${params}`, { signal: controller.signal })
      if (!res.ok) throw new Error(`Error ${res.status}`)
      const data = await res.json()
      searchResults.value = data.results
      searchTruncated.value = data.truncated
    } catch (e) {
      if (e.name === 'AbortError') return
      searchResults.value = []
      searchTruncated.value = false
    }
    if (searchController === controller) searchLoading.value = false
  }, 250)
})

const fold = text => text.normalize('NFKD').replace(/[\u0300-\u036f]/g, '').toLowerCase()

// Folders first, then names that start with what was typed, then the rest by name.
const sortedResults = computed(() => {
  const q = fold(query.value.trim())
  const rank = r => (r.type === 'directory' ? 0 : 2) + (fold(r.name).startsWith(q) ? 0 : 1)
  return [...(searchResults.value || [])].sort((a, b) => rank(a) - rank(b) || collator.compare(a.name, b.name) || collator.compare(a.dir, b.dir))
})

// Where a result is, from the folder being searched: «Fotos › 2024», or this folder's name.
function resultWhere(r) {
  if (r.dir === store.path) return title.value
  const rel = store.path ? r.dir.slice(store.path.length + 1) : r.dir
  return rel.split('/').join(' › ')
}

function resultThumb(r) {
  if (r.type !== 'file' || !isMedia(r)) return ''
  return storeMediaUrls(() => store.location, e => joinPath(e.dir, e.name)).thumbnail(r)
}

// A folder opens; a file opens its folder with it selected (and in Quick Look if it's a photo or video).
let pendingReveal = null
function openResult(r) {
  if (r.type === 'directory') return navigate(store.location, joinPath(r.dir, r.name))
  pendingReveal = r.name
  if (r.dir === store.path) {
    query.value = ''
    revealPending()
  } else {
    navigate(store.location, r.dir)
  }
}

function revealPending() {
  const name = pendingReveal
  pendingReveal = null
  const entry = name && entries.value.find(e => e.name === name)
  if (!entry) return
  selected.value = new Set([name])
  anchor.value = name
  nextTick(() => document.querySelector(`[data-name="${CSS.escape(name)}"]`)?.scrollIntoView({ block: 'center' }))
  if (isMedia(entry)) lookEntry.value = entry
}

async function loadDirectory() {
  loadController?.abort()
  loadController = new AbortController()
  const { signal } = loadController
  selected.value = new Set()
  if (shareMode.value) return loadShareDirectory()

  loading.value = true
  error.value = null
  try {
    const params = new URLSearchParams({ path: store.path, location: store.location })
    const res = await fetch(`/store/api/files/list?${params}`, { signal })
    if (!res.ok) {
      const data = await res.json().catch(() => ({}))
      throw new Error(apiError(data.detail) || `Error ${res.status}`)
    }
    let real = (await res.json()).entries

    // The Compartido folder also lists the share links: yours, and the public ones of others.
    if (store.location === 'external' && store.path === 'shared') {
      let own = new Set()
      const sres = await fetch('/store/api/shares', { signal })
      if (sres.ok) {
        const sdata = await sres.json()
        own = new Set(sdata.map(s => s.token))
        real = [...sdata.map(s => ({
          type: 'share-link', name: s.path.split('/').filter(Boolean).at(-1) || s.token,
          token: s.token, url: s.url, share: s, size: null, modified: null,
        })), ...real]
      }
      const pres = await fetch('/store/api/share', { signal })
      if (pres.ok) {
        const pdata = await pres.json()
        real = [...real, ...pdata.filter(s => !own.has(s.token)).map(s => ({
          type: 'share-link', name: s.label, token: s.token, url: s.url,
          share: { ...s, isOwn: false }, size: null, modified: null,
        }))]
      }
    }

    entries.value = real
    if (pendingReveal) nextTick(revealPending)
    real.forEach(e => {
      if (e.type === 'directory') updateFavIconName(`${store.location}:${joinPath(store.path, e.name)}`, e.icon_name || null)
    })
  } catch (e) {
    if (e.name === 'AbortError') return
    error.value = e.message
    entries.value = []
  } finally {
    if (!signal.aborted) loading.value = false
  }
}

async function loadShareDirectory() {
  loading.value = true
  error.value = null
  try {
    const params = new URLSearchParams({ path: shareMode.value.path })
    const res = await fetch(`/store/api/share/${shareMode.value.token}/files/list?${params}`)
    if (!res.ok) {
      const data = await res.json().catch(() => ({}))
      throw new Error(apiError(data.detail) || `Error ${res.status}`)
    }
    entries.value = (await res.json()).entries
  } catch (e) {
    error.value = e.message
    entries.value = []
  } finally {
    loading.value = false
  }
}

watch(() => [store.location, store.path], () => {
  shareMode.value = null
  query.value = ''
  loadDirectory()
}, { immediate: true })

// ---- Titles and paths ----

const title = computed(() => {
  if (shareMode.value) return shareMode.value.path.split('/').filter(Boolean).at(-1) || shareMode.value.label
  return folderTitle(store.path)
})

const parent = computed(() => {
  if (shareMode.value) {
    if (!shareMode.value.path) return { label: 'Compartido', go: exitShareMode }
    const parts = shareMode.value.path.split('/').filter(Boolean)
    parts.pop()
    return { label: parts.at(-1) || shareMode.value.label, go: () => navigateShareTo(parts.join('/')) }
  }
  const p = parentOf(store.path)
  return p === null ? null : { label: folderTitle(p), go: () => navigate(store.location, p) }
})

const crumbs = computed(() => {
  if (shareMode.value) {
    const parts = shareMode.value.path.split('/').filter(Boolean)
    return [
      { label: tr('Shared', 'Compartido'), go: exitShareMode, home: false },
      { label: shareMode.value.label, go: () => navigateShareTo(''), share: true },
      ...parts.map((p, i) => ({ label: p, go: () => navigateShareTo(parts.slice(0, i + 1).join('/')) })),
    ]
  }
  const parts = store.path.split('/').filter(Boolean)
  const inHome = store.path === homePath.value || store.path.startsWith(`${homePath.value}/`)
  const skip = inHome ? 2 : 1
  const root = { label: inHome ? tr('Home', 'Inicio') : tr('Shared', 'Compartido'), home: inHome, go: () => navigate('external', parts.slice(0, skip).join('/')) }
  return [root, ...parts.slice(skip).map((p, i) => ({ label: p, go: () => navigate('external', parts.slice(0, skip + i + 1).join('/')) }))]
})

const status = computed(() => {
  if (searching.value) {
    if (searchResults.value === null) return tr('Searching…', 'Buscando…')
    const r = sortedResults.value.length
    return r === 1 ? tr('1 result', '1 resultado') : tr(`${r}${searchTruncated.value ? '+' : ''} results`, `${r}${searchTruncated.value ? '+' : ''} resultados`)
  }
  const n = visible.value.length
  const items = n === 1 ? tr('1 item', '1 elemento') : tr(`${n} items`, `${n} elementos`)
  const sel = selected.value.size > 1 ? tr(`${selected.value.size} of ${items} selected`, `${selected.value.size} de ${items} seleccionados`) : items
  return store.disk ? tr(`${sel}, ${formatSize(store.disk.free)} available`, `${sel}, ${formatSize(store.disk.free)} disponibles`) : sel
})

// ---- Thumbnails and viewing ----

// Thumbnails and Quick Look use the light versions the server makes (and caches) for each
// photo and video, as the share page does, never the originals.
const storeUrls = storeMediaUrls(() => store.location, entry => joinPath(store.path, entry.name))
const mediaUrls = computed(() => (shareMode.value
  ? shareMediaUrls(shareMode.value.token, entry => joinPath(shareMode.value.path, entry.name))
  : storeUrls))

function thumbSrc(entry) {
  if (entry.type !== 'file' || !isMedia(entry)) return ''
  return mediaUrls.value.thumbnail(entry)
}

function viewSrc(entry) {
  if (shareMode.value) {
    return `/store/api/share/${shareMode.value.token}/files/view?path=${encodeURIComponent(joinPath(shareMode.value.path, entry.name))}`
  }
  return `/store/api/files/view?${new URLSearchParams({ path: joinPath(store.path, entry.name), location: store.location })}`
}

function open(entry) {
  if (entry.type === 'upload-pending') return
  if (entry.type === 'share-link') return openShareLink(entry)
  if (entry.type === 'directory') {
    if (shareMode.value) return navigateShareTo(joinPath(shareMode.value.path, entry.name))
    return navigate(store.location, joinPath(store.path, entry.name))
  }
  viewItem(entry)
}

async function viewItem(entry) {
  if (isMedia(entry)) {
    lookEntry.value = entry
    return
  }
  if (shareMode.value) {
    window.open(viewSrc(entry), '_blank')
    return
  }
  const res = await fetch('/store/api/files/token', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path: joinPath(store.path, entry.name), location: store.location }),
  })
  const data = await res.json()
  window.open(`/store/${data.token}`, '_blank')
}

function openShareLink(entry) {
  if (entry.share?.is_file) {
    window.open(entry.url, '_blank')
    return
  }
  shareMode.value = { token: entry.token, label: entry.share?.label || entry.name, path: '', history: [''], historyIndex: 0 }
  query.value = ''
  loadDirectory()
}

function navigateShareTo(path) {
  const m = shareMode.value
  m.history = m.history.slice(0, m.historyIndex + 1)
  m.history.push(path)
  m.historyIndex++
  m.path = path
  loadDirectory()
}

function exitShareMode() {
  shareMode.value = null
  loadDirectory()
}

function back() {
  const m = shareMode.value
  if (m) {
    if (m.historyIndex > 0) { m.historyIndex--; m.path = m.history[m.historyIndex]; loadDirectory() }
    else exitShareMode()
    return
  }
  goBack()
}

function forward() {
  const m = shareMode.value
  if (m) {
    if (m.historyIndex < m.history.length - 1) { m.historyIndex++; m.path = m.history[m.historyIndex]; loadDirectory() }
    return
  }
  goForward()
}

const backEnabled = computed(() => !!shareMode.value || canGoBack.value)
const forwardEnabled = computed(() => (shareMode.value ? shareMode.value.historyIndex < shareMode.value.history.length - 1 : canGoForward.value))

// ---- Selection ----

function select(entry, e) {
  const names = visible.value.map(x => x.name)
  const next = new Set(e?.metaKey || e?.ctrlKey ? selected.value : [])
  if (e?.shiftKey && anchor.value && names.includes(anchor.value)) {
    const [a, b] = [names.indexOf(anchor.value), names.indexOf(entry.name)].sort((x, y) => x - y)
    names.slice(a, b + 1).forEach(n => next.add(n))
  } else if ((e?.metaKey || e?.ctrlKey) && next.has(entry.name)) {
    next.delete(entry.name)
  } else {
    next.add(entry.name)
    anchor.value = entry.name
  }
  selected.value = next
}

function onItemClick(entry, e) {
  if (phone.value) return open(entry)
  select(entry, e)
}

const selectedEntries = computed(() => visible.value.filter(e => selected.value.has(e.name)))

function moveSelection(step, extend) {
  const list = visible.value
  if (!list.length) return
  const current = list.findIndex(e => e.name === anchor.value)
  const cols = store.view === 'icons' ? iconColumns() : 1
  const next = Math.max(0, Math.min(list.length - 1, current < 0 ? 0 : current + Math.round(step * cols)))
  const entry = list[next]
  if (extend) {
    const s = new Set(selected.value)
    s.add(entry.name)
    selected.value = s
  } else {
    selected.value = new Set([entry.name])
  }
  anchor.value = entry.name
  nextTick(() => document.querySelector(`[data-name="${CSS.escape(entry.name)}"]`)?.scrollIntoView({ block: 'nearest' }))
}

const grid = ref(null)
function iconColumns() {
  const el = grid.value
  if (!el) return 1
  return getComputedStyle(el).gridTemplateColumns.split(' ').length
}

function onKey(e) {
  if (lookEntry.value || document.querySelector('.overlay, .menu')) return
  if (e.target.closest?.('input, textarea')) {
    if (e.key === 'Escape') { query.value = ''; e.target.blur() }
    return
  }
  const meta = e.metaKey || e.ctrlKey
  if (meta && e.key.toLowerCase() === 'f') { e.preventDefault(); searchInput.value?.focus(); return }
  if (meta && e.key.toLowerCase() === 'a') { e.preventDefault(); selected.value = new Set(visible.value.map(x => x.name)); return }
  if (meta && e.key === 'ArrowUp') { e.preventDefault(); parent.value?.go(); return }
  if (meta && e.key === 'ArrowDown') { e.preventDefault(); const one = selectedEntries.value[0]; if (one) open(one); return }
  if (meta && e.key === '[') { e.preventDefault(); back(); return }
  if (meta && e.key === ']') { e.preventDefault(); forward(); return }
  if (e.key === 'ArrowDown' || (store.view === 'icons' && e.key === 'ArrowRight')) {
    e.preventDefault(); moveSelection(store.view === 'icons' && e.key === 'ArrowDown' ? 1 : 1 / (store.view === 'icons' ? iconColumns() : 1), e.shiftKey)
    return
  }
  if (e.key === 'ArrowUp' || (store.view === 'icons' && e.key === 'ArrowLeft')) {
    e.preventDefault(); moveSelection(store.view === 'icons' && e.key === 'ArrowUp' ? -1 : -1 / (store.view === 'icons' ? iconColumns() : 1), e.shiftKey)
    return
  }
  const one = selectedEntries.value.length === 1 ? selectedEntries.value[0] : null
  if (e.key === ' ' && one) {
    e.preventDefault()
    if (one.type === 'file') {
      if (isMedia(one)) lookEntry.value = one
      else viewItem(one)
    }
    return
  }
  if (e.key === 'Enter' && one && !shareMode.value) { e.preventDefault(); renameItem(one); return }
  if ((e.key === 'Backspace' && meta) || e.key === 'Delete') {
    if (selectedEntries.value.length && !shareMode.value) { e.preventDefault(); deleteItems(selectedEntries.value) }
    return
  }
  if (e.key === 'Escape') selected.value = new Set()
}

// ---- Actions ----

async function post(url, body) {
  const res = await fetch(url, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) })
  if (!res.ok) {
    const data = await res.json().catch(() => ({}))
    throw new Error(apiError(data.detail) || `Error ${res.status}`)
  }
  return res
}

async function run(task, done) {
  try {
    await task()
    if (done) toast(done)
    loadDirectory()
  } catch (e) {
    toast(e.message || tr('Couldn’t finish', 'No se pudo completar'), 'warning')
  }
}

async function createFolder() {
  const name = await prompt({ title: tr('New folder', 'Nueva carpeta'), value: tr('untitled folder', 'Carpeta sin título'), confirm: tr('Create', 'Crear') })
  if (name) run(() => post('/store/api/files/mkdir', { path: joinPath(store.path, name), location: store.location }))
}

async function createFile() {
  const name = await prompt({ title: tr('New file', 'Nuevo archivo'), placeholder: tr('name.txt', 'nombre.txt'), confirm: tr('Create', 'Crear') })
  if (name) run(() => post('/store/api/files/touch', { path: joinPath(store.path, name), location: store.location }))
}

async function renameItem(entry) {
  const dot = entry.type === 'file' ? entry.name.lastIndexOf('.') : -1
  const name = await prompt({ title: tr('Rename', 'Renombrar'), value: entry.name, confirm: tr('Rename', 'Renombrar'), select: dot > 0 ? dot : null })
  if (name && name !== entry.name) {
    run(() => post('/store/api/files/rename', { path: joinPath(store.path, entry.name), new_name: name, location: store.location }))
  }
}

async function deleteItems(list) {
  const one = list.length === 1
  const ok = await confirmDestructive(
    one ? tr(`Delete “${list[0].name}”?`, `¿Eliminar «${list[0].name}»?`) : tr(`Delete ${list.length} items?`, `¿Eliminar ${list.length} elementos?`),
    tr('They’ll be deleted from the server. This can’t be undone.', 'Se borrarán del servidor. No se puede deshacer.'),
  )
  if (!ok) return
  run(async () => {
    for (const entry of list) {
      const params = new URLSearchParams({ path: joinPath(store.path, entry.name), location: store.location })
      const res = await fetch(`/store/api/files?${params}`, { method: 'DELETE' })
      if (!res.ok) {
        const data = await res.json().catch(() => ({}))
        throw new Error(apiError(data.detail) || tr(`Couldn’t delete “${entry.name}”`, `No se pudo eliminar «${entry.name}»`))
      }
    }
  }, one ? tr('Deleted', 'Eliminado') : tr(`${list.length} items deleted`, `${list.length} elementos eliminados`))
}

function duplicateItem(entry) {
  run(() => post('/store/api/files/duplicate', { path: joinPath(store.path, entry.name), location: store.location }))
}

function compressItem(entry) {
  run(() => post('/store/api/files/compress', { path: joinPath(store.path, entry.name), location: store.location }), 'Comprimido')
}

async function decompressItem(entry) {
  const ok = await startDecompress(entry.name, store.path, store.location)
  toast(ok ? tr('Decompressing…', 'Descomprimiendo…') : tr('Couldn’t decompress', 'No se pudo descomprimir'), ok ? 'check-circle' : 'warning')
}

function downloadItem(entry) {
  if (shareMode.value) {
    const p = encodeURIComponent(joinPath(shareMode.value.path, entry.name))
    if (entry.type === 'directory') downloadWithProgress(`/store/api/share/${shareMode.value.token}/files/download-zip?path=${p}`, `${entry.name}.zip`)
    else downloadWithProgress(`/store/api/share/${shareMode.value.token}/files/download?path=${p}`, entry.name)
    return
  }
  const params = new URLSearchParams({ path: joinPath(store.path, entry.name), location: store.location })
  if (entry.type === 'directory') downloadWithProgress(`/store/api/files/download-zip?${params}`, `${entry.name}.zip`)
  else downloadWithProgress(`/store/api/files/download?${params}`, entry.name)
}

async function onIconChosen(iconName) {
  const entry = iconEntry.value
  iconEntry.value = null
  const path = joinPath(store.path, entry.name)
  const idx = entries.value.findIndex(e => e.name === entry.name)
  if (iconName) {
    const res = await fetch('/store/api/files/icon', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path, location: store.location, icon_name: iconName }),
    })
    if (res.ok && idx !== -1) entries.value[idx] = { ...entries.value[idx], icon_name: iconName }
    if (res.ok) updateFavIconName(`${store.location}:${path}`, iconName)
  } else {
    const res = await fetch(`/store/api/files/icon?${new URLSearchParams({ path, location: store.location })}`, { method: 'DELETE' })
    if (res.ok && idx !== -1) {
      const updated = { ...entries.value[idx] }
      delete updated.icon_name
      entries.value[idx] = updated
    }
    if (res.ok) updateFavIconName(`${store.location}:${path}`, null)
  }
}

function addFavourite(path) {
  const added = addFav(store.location, path, store.user.email)
  toast(added ? tr('Added to Favourites', 'Añadida a Favoritos') : tr('Already in Favourites', 'Ya está en Favoritos'), added ? 'check-circle' : 'info')
}

async function deleteShareLink(entry) {
  const ok = await confirmDestructive(tr(`Delete the link to “${entry.name}”?`, `¿Eliminar el enlace de «${entry.name}»?`), tr('Anyone who has it won’t be able to open it any more. The files aren’t deleted.', 'Quien lo tenga dejará de poder abrirlo. Los archivos no se borran.'), tr('Delete link', 'Eliminar enlace'))
  if (ok) run(() => fetch(`/store/api/shares/${entry.token}`, { method: 'DELETE' }), tr('Link deleted', 'Enlace eliminado'))
}

function copyLink(url) {
  navigator.clipboard.writeText(url)
  toast(tr('Link copied', 'Enlace copiado'))
}

// ---- Uploads and downloads ----

function triggerUpload() {
  fileInput.value.click()
}

async function uploadFiles(files) {
  if (!files?.length) return
  for (const file of files) await startUpload(file, store.path, store.location)
}

function onFileInputChange(e) {
  uploadFiles(Array.from(e.target.files))
  e.target.value = ''
}

async function downloadWithProgress(url, filename) {
  if ('showSaveFilePicker' in window) {
    let handle
    try {
      handle = await window.showSaveFilePicker({ suggestedName: filename })
    } catch {
      return
    }
    const id = addTransfer('download', filename, 0)
    try {
      const res = await fetch(url)
      if (!res.ok) { failTransfer(id, `Error ${res.status}`); return }
      const cl = res.headers.get('Content-Length')
      if (cl) setTransferTotal(id, parseInt(cl))
      const writable = await handle.createWritable()
      const reader = res.body.getReader()
      let loaded = 0
      try {
        while (true) {
          const { done, value } = await reader.read()
          if (done) break
          await writable.write(value)
          loaded += value.length
          updateTransfer(id, loaded)
        }
        await writable.close()
      } catch (e) {
        await writable.abort()
        throw e
      }
      completeTransfer(id)
    } catch (e) {
      failTransfer(id, e.message || 'Error')
    }
  } else {
    const id = addTransfer('download', filename, 0)
    try {
      const res = await fetch(url)
      if (!res.ok) { failTransfer(id, `Error ${res.status}`); return }
      const cl = res.headers.get('Content-Length')
      if (cl) setTransferTotal(id, parseInt(cl))
      const reader = res.body.getReader()
      const chunks = []
      let loaded = 0
      while (true) {
        const { done, value } = await reader.read()
        if (done) break
        chunks.push(value)
        loaded += value.length
        updateTransfer(id, loaded)
      }
      const blobUrl = URL.createObjectURL(new Blob(chunks))
      const a = document.createElement('a')
      a.href = blobUrl
      a.download = filename
      a.click()
      setTimeout(() => URL.revokeObjectURL(blobUrl), 60_000)
      completeTransfer(id)
    } catch (e) {
      failTransfer(id, e.message || 'Error')
    }
  }
}

function onDragOver(e) {
  if (shareMode.value || !e.dataTransfer?.types?.includes('Files')) return
  e.preventDefault()
  isDragOver.value = true
}
function onDragLeave(e) {
  if (!e.currentTarget.contains(e.relatedTarget)) isDragOver.value = false
}
function onDrop(e) {
  if (shareMode.value) return
  e.preventDefault()
  isDragOver.value = false
  uploadFiles(Array.from(e.dataTransfer.files))
}

// ---- Menus ----

function viewItems() {
  return [
    { label: tr('Icons', 'Iconos'), icon: LayoutGrid, checked: store.view === 'icons', action: () => setView('icons') },
    { label: tr('List', 'Lista'), icon: List, checked: store.view === 'list', action: () => setView('list') },
    { separator: true },
    { label: tr('Name', 'Nombre'), checked: store.sortKey === 'name', action: () => setSort('name') },
    { label: tr('Date modified', 'Fecha de modificación'), checked: store.sortKey === 'modified', action: () => setSort('modified') },
    { label: tr('Size', 'Tamaño'), checked: store.sortKey === 'size', action: () => setSort('size') },
    { label: tr('Kind', 'Tipo'), checked: store.sortKey === 'kind', action: () => setSort('kind') },
  ]
}

function newItems() {
  return [
    { label: tr('Upload files…', 'Subir archivos…'), icon: Upload, action: triggerUpload },
    { separator: true },
    { label: tr('New folder', 'Nueva carpeta'), icon: FolderPlus, action: createFolder },
    { label: tr('New file', 'Nuevo archivo'), icon: FilePlus, action: createFile },
  ]
}

function folderItems() {
  if (shareMode.value) return [{ label: 'Volver a Compartido', icon: ChevronLeft, action: exitShareMode }]
  return [
    ...newItems(),
    { separator: true },
    { label: tr('Add to Favourites', 'Añadir a Favoritos'), icon: Star, action: () => addFavourite(store.path) },
    { label: tr('Shared links', 'Enlaces compartidos'), icon: Link, action: () => { store.panel = 'shares' } },
  ]
}

function entryItems(entry) {
  if (entry.type === 'upload-pending') {
    return [
      { label: tr('Resume upload…', 'Reanudar subida…'), icon: Upload, action: () => resumeUpload({ ...entry, file_size: entry.size }) },
      { separator: true },
      { label: tr('Cancel upload', 'Cancelar subida'), icon: Trash2, danger: true, action: () => cancelPendingUpload(entry.upload_id) },
    ]
  }
  if (entry.type === 'share-link') {
    return [
      { label: tr('Open', 'Abrir'), icon: Eye, action: () => openShareLink(entry) },
      { label: tr('Open in browser', 'Abrir en el navegador'), icon: ExternalLink, action: () => window.open(entry.url, '_blank') },
      { label: tr('Copy link', 'Copiar enlace'), icon: Copy, action: () => copyLink(entry.url) },
      entry.share?.isOwn !== false && { separator: true },
      entry.share?.isOwn !== false && { label: tr('Link options…', 'Opciones del enlace…'), icon: Pencil, action: () => { shareProps.value = entry.share } },
      entry.share?.isOwn !== false && { label: tr('Delete link', 'Eliminar enlace'), icon: Trash2, danger: true, action: () => deleteShareLink(entry) },
    ]
  }
  if (shareMode.value) {
    return [
      entry.type === 'file' && { label: tr('Open', 'Abrir'), icon: Eye, action: () => viewItem(entry) },
      { label: entry.type === 'directory' ? tr('Download as ZIP', 'Descargar como ZIP') : tr('Download', 'Descargar'), icon: Download, action: () => downloadItem(entry) },
    ]
  }
  const many = selected.value.size > 1 && selected.value.has(entry.name)
  if (many) {
    const list = selectedEntries.value
    return [
      { label: tr(`Download ${list.length} items`, `Descargar ${list.length} elementos`), icon: Download, action: () => list.forEach(downloadItem) },
      { separator: true },
      { label: tr(`Delete ${list.length} items`, `Eliminar ${list.length} elementos`), icon: Trash2, danger: true, action: () => deleteItems(list) },
    ]
  }
  const isDir = entry.type === 'directory'
  const isZip = entry.name.toLowerCase().endsWith('.zip')
  return [
    { label: tr('Open', 'Abrir'), icon: Eye, action: () => open(entry) },
    { separator: true },
    { label: tr('Rename', 'Renombrar'), icon: Pencil, action: () => renameItem(entry) },
    isDir && { label: tr('Change icon…', 'Cambiar icono…'), icon: Palette, action: () => { iconEntry.value = entry } },
    isDir && { label: tr('Add to Favourites', 'Añadir a Favoritos'), icon: Star, action: () => addFavourite(joinPath(store.path, entry.name)) },
    { label: tr('Share…', 'Compartir…'), icon: Share, action: () => { shareEntry.value = entry } },
    { separator: true },
    { label: tr('Move to…', 'Mover a…'), icon: FolderInput, action: () => { moveMode.value = 'move'; moveEntry.value = entry } },
    { label: tr('Copy to…', 'Copiar a…'), icon: Copy, action: () => { moveMode.value = 'copy'; moveEntry.value = entry } },
    { label: tr('Duplicate', 'Duplicar'), icon: CopyPlus, action: () => duplicateItem(entry) },
    { separator: true },
    { label: isDir ? tr('Download as ZIP', 'Descargar como ZIP') : tr('Download', 'Descargar'), icon: Download, action: () => downloadItem(entry) },
    isZip
      ? { label: tr('Decompress', 'Descomprimir'), icon: ArchiveRestore, action: () => decompressItem(entry) }
      : { label: tr('Compress', 'Comprimir'), icon: Archive, action: () => compressItem(entry) },
    { separator: true },
    { label: tr('Delete', 'Eliminar'), icon: Trash2, danger: true, action: () => deleteItems([entry]) },
  ]
}

function onItemMenu(entry, e) {
  e.preventDefault()
  if (!selected.value.has(entry.name)) {
    selected.value = new Set([entry.name])
    anchor.value = entry.name
  }
  openMenu(e, entryItems(entry), phone.value ? entry.name : '')
}

function onBackgroundMenu(e) {
  e.preventDefault()
  selected.value = new Set()
  openMenu(e, folderItems())
}

function onBackgroundClick(e) {
  if (e.target === e.currentTarget || e.target.classList.contains('deselect')) selected.value = new Set()
}

function moreMenu(e) {
  openMenu(e.currentTarget, phone.value ? [...viewItems(), { separator: true }, ...folderItems()] : folderItems())
}

// Long press opens an item's menu on touch screens (iOS sends no contextmenu).
let pressTimer = null
let pressed = false
function onTouchStart(entry, e) {
  pressed = false
  const t = e.touches[0]
  pressTimer = setTimeout(() => {
    pressed = true
    navigator.vibrate?.(10)
    onItemMenu(entry, { clientX: t.clientX, clientY: t.clientY, preventDefault() {} })
  }, 420)
}
function onTouchEnd(e) {
  clearTimeout(pressTimer)
  if (pressed) e.preventDefault()
}
function cancelPress() { clearTimeout(pressTimer) }

function onScroll(e) { scrolled.value = e.target.scrollTop > 4 }

function uploadPercent(entry) {
  return entry.size ? Math.min(100, Math.round((entry.bytes_received / entry.size) * 100)) : 0
}

function subtitle(entry) {
  if (entry.type === 'upload-pending') return tr(`Uploading · ${uploadPercent(entry)}%`, `Subiendo · ${uploadPercent(entry)} %`)
  if (entry.type === 'share-link') return entry.share?.isOwn === false ? tr('Public link', 'Enlace público') : tr('Your shared link', 'Tu enlace compartido')
  if (entry.type === 'directory') return formatDate(entry.modified)
  return `${formatDate(entry.modified)} · ${formatSize(entry.size)}`
}

onMounted(() => {
  document.addEventListener('keydown', onKey)
  window.addEventListener('tpstore:refresh-directory', loadDirectory)
  phoneQuery.addEventListener('change', onPhoneChange)
})
onUnmounted(() => {
  document.removeEventListener('keydown', onKey)
  window.removeEventListener('tpstore:refresh-directory', loadDirectory)
  phoneQuery.removeEventListener('change', onPhoneChange)
})
</script>

<template>
  <section class="browser" :class="{ phone }">
    <input ref="fileInput" type="file" multiple hidden @change="onFileInputChange" />

    <!-- Toolbar -->
    <header class="toolbar" :class="{ scrolled }">
      <template v-if="!phone">
        <div class="nav-pill glass-control">
          <button type="button" :disabled="!backEnabled" :aria-label="tr('Back', 'Atrás')" @click="back"><ChevronLeft :size="20" :stroke-width="2.2" /></button>
          <button type="button" :disabled="!forwardEnabled" aria-label="Adelante" @click="forward"><ChevronRight :size="20" :stroke-width="2.2" /></button>
        </div>
        <h1 class="toolbar-title">{{ title }}</h1>
        <div class="toolbar-spacer" />
        <div class="segmented-pill glass-control" role="group" :aria-label="tr('View', 'Vista')">
          <button type="button" :class="{ on: store.view === 'icons' }" aria-label="Iconos" @click="setView('icons')"><LayoutGrid :size="17" /></button>
          <button type="button" :class="{ on: store.view === 'list' }" aria-label="Lista" @click="setView('list')"><List :size="18" /></button>
        </div>
        <button type="button" class="tool-btn glass-control" :aria-label="tr('Sort', 'Ordenar')" @click="openMenu($event.currentTarget, viewItems().slice(3))"><ArrowUpDown :size="17" /></button>
        <template v-if="!shareMode">
          <button type="button" class="tool-btn glass-control" :aria-label="tr('New', 'Nuevo')" @click="openMenu($event.currentTarget, newItems())"><Plus :size="19" /></button>
        </template>
        <button type="button" class="tool-btn glass-control" :aria-label="tr('More', 'Más')" @click="moreMenu"><MoreHorizontal :size="19" /></button>
        <label class="search glass-control">
          <Search :size="15" />
          <input ref="searchInput" v-model="query" :placeholder="tr('Search', 'Buscar')" :aria-label="tr('Search this folder and its subfolders', 'Buscar en esta carpeta y sus subcarpetas')" />
          <button v-if="query" type="button" class="clear" aria-label="Borrar" @click="query = ''"><X :size="12" :stroke-width="3" /></button>
        </label>
      </template>

      <template v-else>
        <button v-if="parent || backEnabled" type="button" class="round-btn glass-control" :aria-label="tr('Back', 'Atrás')" @click="parent ? parent.go() : back()">
          <ChevronLeft :size="24" :stroke-width="2.2" />
        </button>
        <span v-else />
        <span class="toolbar-title inline">{{ title }}</span>
        <div class="phone-actions">
          <button v-if="!shareMode" type="button" class="round-btn glass-control" :aria-label="tr('New', 'Nuevo')" @click="openMenu($event.currentTarget, newItems())"><Plus :size="22" /></button>
          <button type="button" class="round-btn glass-control" :aria-label="tr('More', 'Más')" @click="moreMenu"><MoreHorizontal :size="22" /></button>
        </div>
      </template>
    </header>

    <!-- Content -->
    <div
      class="content"
      @scroll.passive="onScroll"
      @click="onBackgroundClick"
      @contextmenu="onBackgroundMenu"
      @dragover="onDragOver"
      @dragleave="onDragLeave"
      @drop="onDrop"
    >
      <div v-if="phone" class="phone-header deselect">
        <h1 class="large-title">{{ title }}</h1>
        <label class="phone-search">
          <Search :size="17" />
          <input v-model="query" :placeholder="tr('Search', 'Buscar')" :aria-label="tr('Search this folder and its subfolders', 'Buscar en esta carpeta y sus subcarpetas')" />
          <button v-if="query" type="button" class="clear" aria-label="Borrar" @click="query = ''"><X :size="12" :stroke-width="3" /></button>
        </label>
      </div>

      <Transition name="fade">
        <div v-if="isDragOver" class="drop-zone">
          <FileIcon :entry="{ type: 'directory', name: title }" :size="72" />
          <p class="headline">{{ tr(`Drop to upload to “${title}”`, `Suelta para subir a «${title}»`) }}</p>
        </div>
      </Transition>

      <!-- Searching: this folder and all the folders inside it -->
      <div v-if="searching" class="results deselect">
        <p class="results-scope footnote secondary">{{ tr(`In “${title}” and its subfolders`, `En «${title}» y sus subcarpetas`) }}</p>
        <div v-if="searchResults === null" class="state"><Spinner :size="24" /></div>
        <div v-else-if="!sortedResults.length && !searchLoading" class="state">
          <Search :size="40" class="tertiary" />
          <h3 class="headline">{{ tr('No results', 'Sin resultados') }}</h3>
          <p class="subheadline secondary">{{ tr(`Nothing in “${title}” or its subfolders is called “${query.trim()}”.`, `Nada en «${title}» ni en sus subcarpetas se llama «${query.trim()}».`) }}</p>
        </div>
        <div v-else class="list" role="grid" :class="{ stale: searchLoading }">
          <div
            v-for="r in sortedResults"
            :key="`${r.dir}/${r.name}`"
            class="list-row result"
            role="row"
            tabindex="-1"
            @click.stop="openResult(r)"
          >
            <span class="cell col-name">
              <ItemIcon :entry="r" :size="phone ? 44 : 30" :thumb="resultThumb(r)" :video="kindOf(r) === 'video'" />
              <span class="name-stack">
                <span class="name">{{ r.name }}</span>
                <span class="row-sub where">
                  <FileIcon :entry="{ type: 'directory', name: '' }" :size="13" />{{ resultWhere(r) }}
                </span>
              </span>
              <ChevronRight v-if="phone && r.type === 'directory'" :size="18" class="row-chevron" />
            </span>
            <template v-if="!phone">
              <span class="cell col-modified">{{ formatDate(r.modified) }}</span>
              <span class="cell col-size">{{ r.type === 'file' ? formatSize(r.size) : '—' }}</span>
              <span class="cell col-kind">{{ kindName(r) }}</span>
            </template>
          </div>
        </div>
        <p v-if="searchTruncated" class="results-more footnote secondary">{{ tr('There are more results. Type something more specific to see them.', 'Hay más resultados. Escribe algo más concreto para verlos.') }}</p>
      </div>

      <div v-else-if="loading && !entries.length" class="state"><Spinner :size="24" /></div>

      <div v-else-if="error" class="state">
        <h3 class="headline">{{ tr('Can’t open the folder', 'No se puede abrir la carpeta') }}</h3>
        <p class="subheadline secondary">{{ error }}</p>
        <button type="button" class="btn btn-bordered small" @click="loadDirectory">Reintentar</button>
      </div>

      <div v-else-if="!visible.length" class="state deselect">
        <template v-if="query">
          <Search :size="40" class="tertiary" />
          <h3 class="headline">{{ tr('No results', 'Sin resultados') }}</h3>
          <p class="subheadline secondary">{{ tr(`Nothing in “${title}” is called “${query}”.`, `Nada en «${title}» se llama «${query}».`) }}</p>
        </template>
        <template v-else>
          <FileIcon :entry="{ type: 'directory', name: '' }" :size="84" class="empty-folder" />
          <h3 class="headline">{{ tr('Empty folder', 'Carpeta vacía') }}</h3>
          <p v-if="!shareMode" class="subheadline secondary">{{ phone ? tr('Use + to upload files or create a folder.', 'Usa + para subir archivos o crear una carpeta.') : tr('Drag files here or use + to upload them.', 'Arrastra archivos aquí o usa + para subirlos.') }}</p>
        </template>
      </div>

      <!-- Icons -->
      <div v-else-if="store.view === 'icons'" ref="grid" class="icon-grid deselect">
        <button
          v-for="entry in visible"
          :key="entry.name"
          type="button"
          class="icon-item"
          :class="{ selected: selected.has(entry.name) }"
          :data-name="entry.name"
          @click.stop="onItemClick(entry, $event)"
          @dblclick="open(entry)"
          @contextmenu.stop="onItemMenu(entry, $event)"
          @touchstart.passive="onTouchStart(entry, $event)"
          @touchend="onTouchEnd"
          @touchmove="cancelPress"
          @touchcancel="cancelPress"
        >
          <span class="icon-pic" :class="{ media: !!thumbSrc(entry) }">
            <ItemIcon :entry="entry" :size="phone ? 76 : 84" :thumb="thumbSrc(entry)" :video="kindOf(entry) === 'video'" :duration="entry.duration" />
            <span v-if="entry.type === 'upload-pending'" class="progress"><i :style="{ width: `${uploadPercent(entry)}%` }" /></span>
          </span>
          <span class="icon-name">{{ entry.name }}</span>
          <span v-if="phone" class="icon-meta">{{ entry.type === 'file' ? formatSize(entry.size) : entry.type === 'directory' ? formatDate(entry.modified).split(',')[0] : '' }}</span>
        </button>
      </div>

      <!-- List -->
      <div v-else class="list" role="grid">
        <div v-if="!phone" class="list-head" role="row">
          <button v-for="col in [['name', tr('Name', 'Nombre')], ['modified', tr('Date modified', 'Fecha de modificación')], ['size', tr('Size', 'Tamaño')], ['kind', tr('Kind', 'Tipo')]]" :key="col[0]" type="button" class="col" :class="[`col-${col[0]}`, { sorted: store.sortKey === col[0] }]" @click.stop="setSort(col[0])">
            {{ col[1] }}
            <ChevronRight v-if="store.sortKey === col[0]" :size="12" :stroke-width="2.6" class="sort-arrow" :class="{ asc: store.sortAsc }" />
          </button>
        </div>
        <div
          v-for="entry in visible"
          :key="entry.name"
          class="list-row"
          :class="{ selected: selected.has(entry.name) }"
          role="row"
          tabindex="-1"
          :data-name="entry.name"
          @click.stop="onItemClick(entry, $event)"
          @dblclick="open(entry)"
          @contextmenu.stop="onItemMenu(entry, $event)"
          @touchstart.passive="onTouchStart(entry, $event)"
          @touchend="onTouchEnd"
          @touchmove="cancelPress"
          @touchcancel="cancelPress"
        >
          <span class="cell col-name">
            <ItemIcon :entry="entry" :size="phone ? 44 : 26" :thumb="thumbSrc(entry)" :video="kindOf(entry) === 'video'" :duration="entry.duration" />
            <span class="name-stack">
              <span class="name">{{ entry.name }}</span>
              <span v-if="phone" class="row-sub">{{ subtitle(entry) }}</span>
              <span v-if="entry.type === 'upload-pending'" class="progress"><i :style="{ width: `${uploadPercent(entry)}%` }" /></span>
            </span>
            <ChevronRight v-if="phone && (entry.type === 'directory' || entry.type === 'share-link')" :size="18" class="row-chevron" />
          </span>
          <template v-if="!phone">
            <span class="cell col-modified">{{ formatDate(entry.modified) }}</span>
            <span class="cell col-size">{{ entry.type === 'file' || entry.type === 'upload-pending' ? formatSize(entry.size) : '—' }}</span>
            <span class="cell col-kind">{{ kindName(entry) }}</span>
          </template>
        </div>
      </div>

      <p v-if="phone && visible.length && !searching" class="phone-status footnote secondary">{{ status }}</p>
    </div>

    <!-- Path bar and status -->
    <footer v-if="!phone" class="pathbar">
      <nav class="crumbs">
        <template v-for="(c, i) in crumbs" :key="i">
          <ChevronRight v-if="i > 0" :size="12" :stroke-width="2.4" class="crumb-sep" />
          <button type="button" class="crumb" @click="c.go()">
            <FileIcon :entry="{ type: c.share ? 'share-link' : 'directory', name: c.label, icon_name: c.home ? 'Home' : undefined }" :size="16" />
            <span>{{ c.label }}</span>
          </button>
        </template>
      </nav>
      <span class="status">{{ status }}</span>
    </footer>

    <QuickLook
      v-if="lookEntry"
      :items="media"
      :entry="lookEntry"
      :urls="mediaUrls"
      @change="e => { lookEntry = e; selected = new Set([e.name]); anchor = e.name }"
      @download="downloadItem"
      @close="lookEntry = null"
    />
    <MoveSheet
      v-if="moveEntry"
      :entry="moveEntry"
      :location="store.location"
      :current-path="store.path"
      :mode="moveMode"
      @close="moveEntry = null"
      @moved="moveEntry = null; loadDirectory()"
    />
    <ShareSheet
      v-if="shareEntry || shareProps"
      :entry="shareEntry"
      :share="shareProps"
      :location="store.location"
      :current-path="store.path"
      @close="shareEntry = null; shareProps = null"
      @changed="loadDirectory"
    />
    <IconSheet v-if="iconEntry" :entry="iconEntry" @close="iconEntry = null" @done="onIconChosen" />
  </section>
</template>

<style scoped>
/* ---- Search results ---- */
.results-scope { padding: 10px 20px 6px; }
.results .list.stale { opacity: 0.6; transition: opacity 0.2s; }
.results .result { cursor: default; }
.results .where { display: flex; align-items: center; gap: 4px; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.results .where :deep(svg), .results .where > * { flex-shrink: 0; }
.results-more { padding: 12px 20px 20px; text-align: center; }

.browser { position: relative; flex: 1; min-width: 0; display: flex; flex-direction: column; overflow: hidden; }

/* ---- Toolbar ---- */
.toolbar {
  position: relative;
  z-index: 5;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  height: 64px;
  padding: 0 16px 0 18px;
}
.toolbar::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  bottom: -1px;
  height: 0.5px;
  background: var(--separator);
  opacity: 0;
  transition: opacity 0.2s;
}
.toolbar.scrolled::after { opacity: 1; }
.toolbar-title {
  margin-left: 6px;
  font-size: 17px;
  font-weight: 700;
  letter-spacing: -0.3px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
.toolbar-spacer { flex: 1; }

.glass-control {
  background: var(--glass-strong);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.08), 0 0 0 0.5px rgba(0, 0, 0, 0.08), inset 0 0.5px 0 var(--glass-edge);
  color: var(--label);
}
@media (prefers-color-scheme: dark) {
  .glass-control { box-shadow: 0 2px 10px rgba(0, 0, 0, 0.35), 0 0 0 0.5px rgba(255, 255, 255, 0.1); }
}
.nav-pill, .segmented-pill { display: flex; align-items: center; height: 38px; padding: 0 3px; border-radius: 999px; }
.nav-pill button, .segmented-pill button {
  width: 38px;
  height: 32px;
  display: grid;
  place-items: center;
  border-radius: 999px;
  color: var(--label);
  transition: background 0.15s, opacity 0.15s;
}
.nav-pill button:hover:not(:disabled), .segmented-pill button:hover { background: var(--fill-3); }
.nav-pill button:disabled { opacity: 0.3; cursor: default; }
.segmented-pill button.on { background: var(--fill-2); }
.tool-btn { width: 38px; height: 38px; display: grid; place-items: center; border-radius: 999px; transition: transform 0.2s var(--spring); }
.tool-btn:hover { background: var(--glass); }
.tool-btn:active { transform: scale(0.94); }
.search {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 200px;
  height: 38px;
  padding: 0 12px;
  border-radius: 999px;
  color: var(--label-2);
  transition: width 0.3s var(--ease);
}
.search:focus-within { width: 260px; box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 40%, transparent); }
.search input { flex: 1; min-width: 0; border: none; outline: none; background: none; font-size: 14px; color: var(--label); }
.search input::placeholder, .phone-search input::placeholder { color: var(--label-3); }
.clear { width: 17px; height: 17px; display: grid; place-items: center; border-radius: 50%; color: var(--bg-elevated); background: var(--label-3); flex-shrink: 0; }

/* ---- Content ---- */
.content { position: relative; flex: 1; overflow-y: auto; overscroll-behavior: contain; padding: 4px 12px 24px; }
.state { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; min-height: 60%; padding: 48px 24px; text-align: center; }
.empty-folder { opacity: 0.45; margin-bottom: 6px; }
.drop-zone {
  position: absolute;
  inset: 8px;
  z-index: 4;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  border-radius: 22px;
  border: 2.5px dashed var(--accent);
  background: color-mix(in srgb, var(--accent) 10%, transparent);
  pointer-events: none;
}

/* Icons */
.icon-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(118px, 1fr)); gap: 6px 4px; padding: 8px 4px; align-content: start; min-height: 100%; }
.icon-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 8px 4px 10px;
  border-radius: 12px;
  text-align: center;
  cursor: default;
}
.icon-pic { position: relative; width: 96px; height: 92px; display: grid; place-items: center; border-radius: 10px; }
.icon-item.selected .icon-pic { background: var(--fill-3); }
.icon-pic.media :deep(.thumb) { width: 88px !important; height: 66px !important; }
.icon-name {
  max-width: 112px;
  padding: 1px 6px;
  border-radius: 5px;
  font-size: 13px;
  line-height: 17px;
  letter-spacing: -0.08px;
  overflow-wrap: anywhere;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.icon-item.selected .icon-name { color: #fff; background: var(--accent); }
.icon-meta { font-size: 12px; color: var(--label-2); margin-top: -3px; }
.icon-pic .progress { position: absolute; left: 14px; right: 14px; bottom: 2px; }

/* List */
.list { padding: 0 4px; }
.list-head {
  position: sticky;
  top: -4px;
  z-index: 2;
  display: grid;
  grid-template-columns: minmax(200px, 1fr) 190px 100px 160px;
  height: 30px;
  margin: 0 -4px 2px;
  padding: 0 4px;
  background: color-mix(in srgb, var(--bg) 88%, transparent);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border-bottom: 0.5px solid var(--separator);
}
.col { display: flex; align-items: center; gap: 4px; padding: 0 10px; font-size: 12px; font-weight: 600; color: var(--label-2); text-align: left; white-space: nowrap; }
.col.sorted { color: var(--label); }
.col-size { justify-content: flex-end; }
.sort-arrow { transform: rotate(90deg); transition: transform 0.2s; }
.sort-arrow.asc { transform: rotate(-90deg); }
.list-row {
  display: grid;
  grid-template-columns: minmax(200px, 1fr) 190px 100px 160px;
  align-items: center;
  height: 36px;
  border-radius: 8px;
  outline: none;
  cursor: default;
}
.list-row:nth-child(odd of .list-row) { background: color-mix(in srgb, var(--label) 3.5%, transparent); }
.list-row.selected { background: var(--accent) !important; color: #fff; }
.list-row.selected .cell { color: #fff; }
.cell { min-width: 0; padding: 0 10px; font-size: 13px; letter-spacing: -0.08px; color: var(--label-2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.cell.col-name { display: flex; align-items: center; gap: 10px; color: var(--label); }
.cell.col-size { text-align: right; }
.name-stack { min-width: 0; flex: 1; display: flex; flex-direction: column; justify-content: center; }
.name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.list-row .progress { margin-top: 2px; }

.progress { display: block; height: 3px; border-radius: 2px; background: color-mix(in srgb, var(--orange) 25%, transparent); overflow: hidden; }
.progress i { display: block; height: 100%; background: var(--orange); transition: width 0.4s; }

/* ---- Path bar ---- */
.pathbar {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 12px;
  height: 32px;
  padding: 0 14px;
  border-top: 0.5px solid var(--separator);
  font-size: 12px;
  color: var(--label-2);
}
.crumbs { flex: 1; min-width: 0; display: flex; align-items: center; gap: 2px; overflow: hidden; }
.crumb { display: flex; align-items: center; gap: 5px; padding: 3px 6px; border-radius: 6px; color: var(--label-2); white-space: nowrap; min-width: 0; }
.crumb span { overflow: hidden; text-overflow: ellipsis; }
.crumb:hover { background: var(--fill-3); color: var(--label); }
.crumb:last-child { color: var(--label); }
.crumb-sep { flex-shrink: 0; color: var(--label-3); }
.status { flex-shrink: 0; }

/* ---- Phone: iOS Files ---- */
.phone .toolbar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: calc(56px + env(safe-area-inset-top));
  padding: env(safe-area-inset-top) 12px 0;
  justify-content: space-between;
}
.phone .toolbar::before {
  content: "";
  position: absolute;
  inset: 0 0 -20px;
  z-index: -1;
  background: linear-gradient(to bottom, color-mix(in srgb, var(--bg) 85%, transparent) 50%, transparent);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  mask-image: linear-gradient(to bottom, #000 60%, transparent);
  -webkit-mask-image: linear-gradient(to bottom, #000 60%, transparent);
  opacity: 0;
  transition: opacity 0.2s;
}
.phone .toolbar.scrolled::before { opacity: 1; }
.phone .toolbar::after { display: none; }
.toolbar-title.inline { position: absolute; left: 50%; transform: translateX(-50%); max-width: 50%; margin: 0; font-size: 17px; font-weight: 600; opacity: 0; transition: opacity 0.2s; }
.phone .toolbar.scrolled .toolbar-title.inline { opacity: 1; }
.round-btn { width: 44px; height: 44px; display: grid; place-items: center; border-radius: 50%; transition: transform 0.2s var(--spring); }
.round-btn:active { transform: scale(0.92); }
.phone-actions { display: flex; gap: 8px; }
.phone .content { padding: calc(60px + env(safe-area-inset-top)) 16px calc(110px + env(safe-area-inset-bottom)); }
.phone-header { padding: 0 4px 12px; }
.phone-header .large-title { overflow-wrap: anywhere; }
.phone-search {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 38px;
  margin-top: 10px;
  padding: 0 10px;
  border-radius: 12px;
  color: var(--label-2);
  background: var(--fill-3);
}
.phone-search input { flex: 1; min-width: 0; border: none; outline: none; background: none; font-size: 17px; color: var(--label); }
.phone .list { padding: 0; border-radius: var(--radius-card); overflow: hidden; background: var(--cell); }
.phone .list-row {
  position: relative;
  display: flex;
  height: auto;
  min-height: 64px;
  border-radius: 0;
  background: var(--cell) !important;
  color: var(--label);
}
.phone .list-row:active { background: var(--cell-pressed) !important; }
.phone .list-row + .list-row::before { content: ""; position: absolute; top: 0; left: 72px; right: 0; height: 0.5px; background: var(--separator); }
.phone .list-row.selected .cell { color: var(--label); }
.phone .cell.col-name { flex: 1; gap: 14px; padding: 10px 16px; font-size: 17px; overflow: visible; }
.phone .name { white-space: normal; overflow-wrap: anywhere; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
.row-sub { font-size: 13px; color: var(--label-2); margin-top: 2px; }
.row-chevron { flex-shrink: 0; color: var(--label-3); }
.phone .icon-grid { grid-template-columns: repeat(3, 1fr); gap: 14px 6px; padding: 4px 0; }
.phone .icon-item.selected .icon-pic, .phone .icon-item.selected .icon-name { background: none; color: inherit; }
.phone .icon-pic { width: 100%; height: 86px; }
.phone .icon-name { font-size: 13px; }
.phone-status { text-align: center; padding: 18px 0 0; }
.phone .state { min-height: 50vh; }
</style>
