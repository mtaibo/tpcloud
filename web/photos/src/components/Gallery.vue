<script setup>
import { computed, markRaw, nextTick, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue'
import { CheckCircle, ChevronLeft, Circle, Download, Heart, HeartOff, Images, ListMinus, ListPlus, Minus, MoreHorizontal, Plus, Trash2 } from 'lucide-vue-next'
import Spinner from '@ui/Spinner.vue'
import Segmented from '@ui/Segmented.vue'
import { menuState, openMenu } from '../menu.js'
import { confirmDestructive, toast } from '@ui/dialog.js'
import { albumIcon, albumMenuItems, albumName, createAlbum, newAlbumItem } from '../albums.js'
import { vLongPress } from '../longPress.js'
import { downloadLink, photoMediaUrls } from '../mediaUrls.js'
import { LEVELS, PhotoGrid, maxLevelFor } from '../grid/PhotoGrid.js'
import QuickLook from './QuickLook.vue'
import { locale, t } from '@ui/i18n.js'

// The photos backed up from the phone, laid out like the iPhone app's Photos tab: one grid with
// the newest photo at the bottom, where the view starts, at 3, 5, 9, 15, 31 or 61 columns (pinch,
// the zoom bar or + and −), the section's title large over the photos with the dates on screen
// under it, and at 15 columns the month, at 31 and 61 the year, where each begins. The grid itself is
// `PhotoGrid` (canvas rows and a thumbnail worker), so this only feeds it and handles the rest.

// Which photos: "all", "camera", "screenshots" or "album:<id>", picked in the sidebar on a
// computer and with the segmented control on a phone.
const section = defineModel('section', { type: String, default: 'all' })
const props = defineProps({ albums: { type: Array, default: () => [] }, libraryVersion: { type: Number, default: 0 } })
const emit = defineEmits(['albums-changed'])

const titles = () => ({ all: t('Library', 'Biblioteca'), camera: t('Camera', 'Cámara'), screenshots: t('Screenshots', 'Capturas') })
const albumId = computed(() => (section.value.startsWith('album:') ? Number(section.value.slice(6)) : null))
const album = computed(() => props.albums.find(a => a.id === albumId.value) || null)
const favoritesId = ref(null)
const inFavorites = computed(() => albumId.value !== null && albumId.value === favoritesId.value)
const title = computed(() => (album.value?.kind === 'favorites' ? t('Favourites', 'Favoritos') : album.value?.name) || titles()[section.value] || t('Library', 'Biblioteca'))

// Albums of your own (made here or in the iPhone app), hidden ones included, for «Añadir a».
const userAlbums = computed(() => props.albums.filter(a => a.kind !== 'favorites'))

// On a phone the last segment is «Álbumes»: a menu of every album (Favoritos, yours, then the
// hidden ones) and «Nuevo álbum…».
const filters = computed(() => [
  { value: 'all', label: t('All', 'Todas') },
  { value: 'camera', label: t('Camera', 'Cámara') },
  { value: 'screenshots', label: t('Screenshots', 'Capturas') },
  ...(props.albums.length ? [{ value: 'albums', label: t('Albums', 'Álbumes') }] : []),
])
const segment = computed({
  get: () => (albumId.value !== null && props.albums.length ? 'albums' : section.value),
  set: v => { if (v === 'albums') albumsMenu(); else section.value = v },
})

function albumsMenu() {
  const pick = a => ({ label: albumName(a), icon: albumIcon(a), checked: albumId.value === a.id, action: () => { section.value = `album:${a.id}` } })
  const hidden = props.albums.filter(a => a.hidden)
  openMenu(filtersEl.value, [
    ...props.albums.filter(a => !a.hidden).map(pick),
    hidden.length && { separator: true },
    ...hidden.map(pick),
    { separator: true },
    newAlbumItem(async () => {
      const a = await createAlbum()
      if (!a) return
      emit('albums-changed')
      section.value = `album:${a.id}`
    }),
  ], t('Albums', 'Álbumes'))
}

// The open album's own menu (rename, hide, keep out of the library, delete), over the title.
function openAlbumMenu(e) {
  const a = album.value
  if (!a) return
  openMenu(e.currentTarget, albumMenuItems(a, {
    changed: change => emit('albums-changed', change),
    deleted: () => { section.value = 'all' },
  }), a.name)
}

watch(() => props.libraryVersion, () => { if (albumId.value === null) load() })

// Oldest first, as on screen. Plain objects (not reactive): there are thousands of them and the
// grid reads them every frame.
const items = shallowRef([])
const loading = ref(false)
const loaded = ref(false)
const error = ref('')
const look = shallowRef(null)
const lookItems = shallowRef(null) // Quick Look over the selected photos only ("Show Selected")
const favVersion = ref(0) // bumped when a heart changes, so Quick Look shows it
const level = ref(readLevel())
const view = ref({ first: -1, last: -1, atNewest: true })
const maxLevel = ref(LEVELS.length - 1) // 61 columns only on a big screen

const scroller = ref(null)
const head = ref(null)
const filtersEl = ref(null)
const selectBar = ref(null)
const gallery = ref(null)
let grid = null
let generation = 0

const LEVEL_KEY = 'tpphotos-level'
function readLevel() {
  try {
    const saved = Number(localStorage.getItem(LEVEL_KEY))
    if (Number.isInteger(saved) && saved >= 0 && saved < LEVELS.length && localStorage.getItem(LEVEL_KEY) !== null) return saved
  } catch { /* private window */ }
  return window.innerWidth < 768 ? 0 : 2
}
let saveTimer = 0
function saveLevel(l) {
  level.value = l
  clearTimeout(saveTimer)
  saveTimer = setTimeout(() => { try { localStorage.setItem(LEVEL_KEY, String(l)) } catch { /* fine */ } }, 400)
}

const dense = computed(() => LEVELS[level.value] >= 15)

const dayFormat = new Intl.DateTimeFormat(locale(), { day: 'numeric', month: 'short', year: 'numeric' })
const longFormat = new Intl.DateTimeFormat(locale(), { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric', hour: '2-digit', minute: '2-digit' })
const monthFormat = new Intl.DateTimeFormat(locale(), { month: 'short' })
const numberFormat = new Intl.NumberFormat(locale())

// Under the title, like Photos: how many there are at the newest end ("11,881 Items"), anywhere
// else the days on screen ("4–7 Nov 2026"). The dense grids have their chips instead.
const subtitle = computed(() => {
  const list = items.value
  if (!list.length || dense.value) return ''
  const { first, last, atNewest } = view.value
  if (atNewest || first < 0) {
    const n = numberFormat.format(list.length)
    return list.length === 1 ? t('1 Item', '1 elemento') : t(`${n} Items`, `${n} elementos`)
  }
  let from = null
  let to = null
  for (let i = first; i <= last && !from; i++) from = list[i]?.date
  for (let i = last; i >= first && !to; i--) to = list[i]?.date
  if (!from || !to) return ''
  try { return dayFormat.formatRange(from, to) } catch { return dayFormat.format(to) }
})

function chipLabel(p, kind) {
  const year = String(p.date.getFullYear())
  if (kind === 'year') return year
  const month = monthFormat.format(p.date).replace('.', '')
  return `${month.charAt(0).toUpperCase()}${month.slice(1)} ${year}`
}

function toItem([id, checksum, path, takenAt, mime, favorite]) {
  const date = takenAt != null ? new Date(takenAt) : null
  const name = path.split('/').at(-1) // its readable name on the server, which downloads keep
  return markRaw({
    id, checksum, path, name, date, mime_type: mime,
    taken_at: date ? date.toISOString() : null,
    favorite: !!favorite,
    isVideo: (mime || '').startsWith('video/'),
    type: 'file',
    location: 'external',
  })
}

async function load() {
  const mine = ++generation
  loading.value = true
  loaded.value = false
  error.value = ''
  look.value = null
  items.value = []
  grid?.setItems([])
  try {
    const params = new URLSearchParams()
    if (albumId.value !== null) params.set('album', String(albumId.value))
    else params.set('filter', section.value)
    const res = await fetch(`/store/api/photos/index?${params}`)
    if (mine !== generation) return
    if (!res.ok) { error.value = t('The photos couldn’t be loaded.', 'No se han podido cargar las fotos.'); return }
    const data = await res.json()
    if (mine !== generation) return
    favoritesId.value = data.favorites_album ?? favoritesId.value
    // The API lists newest first; on screen it's oldest first, newest last.
    const list = new Array(data.items.length)
    for (let i = 0, n = data.items.length; i < n; i++) list[n - 1 - i] = toItem(data.items[i])
    items.value = list
    loaded.value = true
    await nextTick()
    measure()
    grid?.setItems(list, { toNewest: true })
  } catch {
    if (mine === generation) error.value = t('No connection. Try again.', 'No hay conexión. Vuelve a intentarlo.')
  } finally {
    if (mine === generation) loading.value = false
  }
}
watch(section, () => { endSelect(); load() })

// The title floats over the photos, and on a phone the filters do too: the grid leaves room for
// them at its ends.
function measure() {
  if (!grid || !gallery.value) return
  if (scroller.value) maxLevel.value = maxLevelFor(scroller.value.clientWidth)
  const top = head.value ? head.value.offsetHeight : 0
  let bottom = 4
  const bar = selecting.value ? selectBar.value : filtersEl.value
  if (bar && getComputedStyle(bar).display !== 'none') {
    bottom = gallery.value.getBoundingClientRect().bottom - bar.getBoundingClientRect().top + 8
  }
  grid.setInsets(top, bottom)
}

function zoom(step) {
  grid?.zoomBy(step)
}

function onSlider(e) {
  grid?.zoomTo(maxLevel.value - Number(e.target.value))
}

function onKey(e) {
  if (look.value || menuState.open) return
  if (['INPUT', 'TEXTAREA', 'SELECT'].includes(document.activeElement?.tagName)) return
  if (selecting.value && e.key === 'Escape') { e.preventDefault(); endSelect(); return }
  if (selecting.value && (e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'a') { e.preventDefault(); selectAll(); return }
  if (e.metaKey || e.ctrlKey || e.altKey) return
  if (e.key === '+' || e.key === '=') { e.preventDefault(); zoom(-1) }
  else if (e.key === '-' || e.key === '_') { e.preventDefault(); zoom(1) }
}

// Pinching anywhere else on the page would zoom the page itself in Safari.
const stopPageZoom = e => e.preventDefault()

function photoSubtitle(p) {
  return p.date ? longFormat.format(p.date).replace(/^./, c => c.toUpperCase()) : ''
}

function download(p) {
  downloadLink(photoMediaUrls.download(p), p.name)
}

// Deletes it from the server. The iPhone app backs up whatever is in the phone's library, so a
// photo still on the phone comes back with the next backup.
async function remove(p) {
  const ok = await confirmDestructive(t('Delete this photo?', '¿Eliminar esta foto?'), t('It will be deleted from the server. If it’s still on the iPhone, the next backup will upload it again.', 'Se borrará del servidor. Si sigue en el iPhone, la próxima copia de seguridad la volverá a subir.'), t('Delete', 'Eliminar'))
  if (!ok) return
  const res = await fetch(`/store/api/photos/${p.id}`, { method: 'DELETE' })
  if (!res.ok) { toast(t('It couldn’t be deleted', 'No se ha podido eliminar'), 'x-circle'); return }
  dropFromList(p)
  toast(t('Photo deleted', 'Foto eliminada'))
  if (p.favorite) emit('albums-changed')
}

function dropFromList(p) {
  dropMany(new Set([p.id]))
}

// Takes photos out of view (deleted, or out of the album on screen), and out of the selection and
// Quick Look's list too.
function dropMany(ids) {
  if (!ids.size) return
  const shown = lookItems.value || items.value
  if (look.value && ids.has(look.value.id)) {
    const i = shown.indexOf(look.value)
    look.value = shown.slice(i + 1).find(x => !ids.has(x.id)) || shown.slice(0, i).reverse().find(x => !ids.has(x.id)) || null
  }
  if (lookItems.value) lookItems.value = lookItems.value.filter(x => !ids.has(x.id))
  const next = items.value.filter(x => !ids.has(x.id))
  items.value = next
  if (selectedIds.value.size) setSelected(new Set([...selectedIds.value].filter(id => !ids.has(id))))
  grid?.setItems(next)
}

// The heart, as in Photos: adds it to Favoritos or takes it out (and out of view, when looking
// at Favoritos). The tile changes straight away and goes back if the server says no.
async function toggleFavorite(p) {
  if (!favoritesId.value) return
  const on = !p.favorite
  setFavorite(p, on)
  const res = await fetch(`/store/api/albums/${favoritesId.value}/photos/${p.id}`, { method: on ? 'PUT' : 'DELETE' }).catch(() => null)
  if (!res?.ok) {
    setFavorite(p, !on)
    toast(t('It couldn’t be changed', 'No se ha podido cambiar'), 'x-circle')
    return
  }
  if (!on && inFavorites.value) dropFromList(p)
  emit('albums-changed')
}

function setFavorite(p, on) {
  p.favorite = on
  favVersion.value++
  grid?.refresh(p.id)
}

const isFavorite = computed(() => {
  favVersion.value // a new function whenever a heart changes, so Quick Look draws it again
  return favoritesId.value ? (p => p.favorite) : null
})

// The same albums the iPhone app has: add a photo to one of yours, take it out of the one you're
// looking at, or start a new one with it.
async function addToAlbum(p, a) {
  const res = await fetch(`/store/api/albums/${a.id}/photos`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ ids: [p.id] }) }).catch(() => null)
  if (!res?.ok) { toast(t('It couldn’t be added', 'No se ha podido añadir'), 'warning'); return }
  toast(t(`Added to “${a.name}”`, `Añadida a «${a.name}»`))
  if (a.hides_from_library && albumId.value === null) dropFromList(p)
  emit('albums-changed')
}

async function removeFromAlbum(p) {
  const a = album.value
  if (!a) return
  const res = await fetch(`/store/api/albums/${a.id}/photos/remove`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ ids: [p.id] }) }).catch(() => null)
  if (!res?.ok) { toast(t('It couldn’t be removed', 'No se ha podido quitar'), 'warning'); return }
  dropFromList(p)
  emit('albums-changed')
}

async function newAlbum(p) {
  const a = await createAlbum([p.id])
  if (!a) return
  toast(t(`Added to “${a.name}”`, `Añadida a «${a.name}»`))
  emit('albums-changed')
}

// ---- Select mode, like the app's: tap photos (or drag across them with a mouse, shift-click for a
// run) and act on all of them from the bar at the bottom: download, see them, add them to an
// album, take them out of the one on screen, delete them.

const selecting = ref(false)
const selectedIds = shallowRef(new Set())
const selectedCount = computed(() => selectedIds.value.size)
let lastToggled = -1
let dragBase = null
let dragSelects = true

function setSelected(ids) {
  selectedIds.value = ids
  grid?.setSelection(selecting.value ? ids : null)
}

async function startSelect() {
  selecting.value = true
  lastToggled = -1
  setSelected(new Set())
  await nextTick()
  measure()
}

async function endSelect() {
  if (!selecting.value) return
  selecting.value = false
  setSelected(new Set())
  await nextTick()
  measure()
}

function selectedPhotos() {
  const ids = selectedIds.value
  return items.value.filter(p => ids.has(p.id))
}

function onToggle(i, e) {
  const next = new Set(selectedIds.value)
  if (e?.shiftKey && lastToggled >= 0 && lastToggled < items.value.length) {
    const [a, b] = [lastToggled, i].sort((x, y) => x - y)
    for (let k = a; k <= b; k++) next.add(items.value[k].id)
  } else {
    const id = items.value[i].id
    if (next.has(id)) next.delete(id)
    else next.add(id)
  }
  lastToggled = i
  setSelected(next)
}

function onDrag(from, to, first) {
  if (first) {
    dragBase = selectedIds.value
    dragSelects = !dragBase.has(items.value[from].id)
  }
  const next = new Set(dragBase)
  const [a, b] = [from, to].sort((x, y) => x - y)
  for (let k = a; k <= b; k++) {
    const id = items.value[k].id
    if (dragSelects) next.add(id)
    else next.delete(id)
  }
  lastToggled = to
  setSelected(next)
}

function selectAll() {
  setSelected(new Set(items.value.map(p => p.id)))
}

function showSelected() {
  const list = selectedPhotos()
  if (!list.length) return
  lookItems.value = list
  look.value = list[0]
}

// The originals, one after another (the browser may ask once to allow several downloads).
function downloadSelected() {
  selectedPhotos().forEach((p, i) => setTimeout(() => download(p), i * 350))
}

async function deleteSelected() {
  const list = selectedPhotos()
  const n = list.length
  if (!n) return
  const ok = await confirmDestructive(
    n === 1 ? t('Delete this photo?', '¿Eliminar esta foto?') : t(`Delete ${n} photos?`, `¿Eliminar ${n} fotos?`),
    t('They’ll be deleted from the server. The ones still on the iPhone will come back with the next backup.', 'Se borrarán del servidor. Las que sigan en el iPhone volverán con la próxima copia de seguridad.'),
    n === 1 ? t('Delete Photo', 'Eliminar foto') : t(`Delete ${n} Photos`, `Eliminar ${n} fotos`),
  )
  if (!ok) return
  const done = new Set()
  const queue = [...list]
  await Promise.all(Array.from({ length: 4 }, async () => {
    for (let p = queue.shift(); p; p = queue.shift()) {
      const res = await fetch(`/store/api/photos/${p.id}`, { method: 'DELETE' }).catch(() => null)
      if (res?.ok) done.add(p.id)
    }
  }))
  endSelect()
  dropMany(done)
  if (done.size < n) toast(t('Some photos couldn’t be deleted', 'Algunas fotos no se han podido eliminar'), 'warning')
  else toast(n === 1 ? t('Photo deleted', 'Foto eliminada') : t(`${n} photos deleted`, `${n} fotos eliminadas`))
  if (list.some(p => p.favorite && done.has(p.id))) emit('albums-changed')
}

async function addSelectedTo(a) {
  const list = selectedPhotos()
  if (!list.length) return
  const res = await fetch(`/store/api/albums/${a.id}/photos`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ ids: list.map(p => p.id) }) }).catch(() => null)
  if (!res?.ok) { toast(t('They couldn’t be added', 'No se han podido añadir'), 'warning'); return }
  endSelect()
  const name = albumName(a)
  toast(list.length === 1 ? t(`Added to “${name}”`, `Añadida a «${name}»`) : t(`${list.length} added to “${name}”`, `${list.length} añadidas a «${name}»`))
  if (a.kind === 'favorites') list.forEach(p => setFavorite(p, true))
  if (a.hides_from_library && albumId.value === null) dropMany(new Set(list.map(p => p.id)))
  emit('albums-changed')
}

async function newAlbumWithSelected() {
  const list = selectedPhotos()
  if (!list.length) return
  const a = await createAlbum(list.map(p => p.id))
  if (!a) return
  endSelect()
  toast(t(`Added to “${a.name}”`, `Añadidas a «${a.name}»`))
  emit('albums-changed')
}

async function removeSelectedFromAlbum() {
  const a = album.value
  const list = selectedPhotos()
  if (!a || !list.length) return
  const res = await fetch(`/store/api/albums/${a.id}/photos/remove`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ ids: list.map(p => p.id) }) }).catch(() => null)
  if (!res?.ok) { toast(t('They couldn’t be removed', 'No se han podido quitar'), 'warning'); return }
  endSelect()
  if (a.kind === 'favorites') list.forEach(p => setFavorite(p, false))
  dropMany(new Set(list.map(p => p.id)))
  emit('albums-changed')
}

// The bar's •••: Select All / Deselect All, then the albums, as in the app's menu.
function selectionMenu(e) {
  const none = !selectedCount.value
  openMenu(e.currentTarget, [
    { label: t('Select All', 'Seleccionar todo'), icon: CheckCircle, action: selectAll },
    { label: t('Deselect All', 'Deseleccionar todo'), icon: Circle, disabled: none, action: () => setSelected(new Set()) },
    { separator: true },
    ...props.albums.filter(a => a.id !== albumId.value).map(a => ({ label: t(`Add to “${albumName(a)}”`, `Añadir a «${albumName(a)}»`), icon: a.kind === 'favorites' || a.hidden ? albumIcon(a) : ListPlus, disabled: none, action: () => addSelectedTo(a) })),
    { label: t('New Album…', 'Nuevo álbum…'), icon: Plus, disabled: none, action: newAlbumWithSelected },
    album.value && { label: t('Remove from Album', 'Quitar del álbum'), icon: ListMinus, disabled: none, action: removeSelectedFromAlbum },
  ], none ? t('Select Items', 'Seleccionar') : selectionLabel.value)
}

const selectionLabel = computed(() => {
  const n = selectedCount.value
  return n === 1 ? t('1 Selected', '1 seleccionada') : t(`${numberFormat.format(n)} Selected`, `${numberFormat.format(n)} seleccionadas`)
})

function menu(p, at) {
  const inOwnAlbum = album.value && album.value.kind !== 'favorites'
  openMenu(at, [
    favoritesId.value && (p.favorite
      ? { label: t('Remove from Favourites', 'Quitar de Favoritos'), icon: HeartOff, action: () => toggleFavorite(p) }
      : { label: t('Add to Favourites', 'Añadir a Favoritos'), icon: Heart, action: () => toggleFavorite(p) }),
    inOwnAlbum && { label: t('Remove from this album', 'Quitar de este álbum'), icon: ListMinus, action: () => removeFromAlbum(p) },
    { separator: true },
    ...userAlbums.value.filter(a => a.id !== albumId.value).map(a => ({ label: t(`Add to “${a.name}”`, `Añadir a «${a.name}»`), icon: a.hidden ? albumIcon(a) : ListPlus, action: () => addToAlbum(p, a) })),
    { label: t('New album…', 'Nuevo álbum…'), icon: Plus, action: () => newAlbum(p) },
    { separator: true },
    { label: t('Download the original', 'Descargar el original'), icon: Download, action: () => download(p) },
    { separator: true },
    { label: t('Delete', 'Eliminar'), icon: Trash2, danger: true, action: () => remove(p) },
  ], p.name)
}

function onLongPress(pt) {
  const i = grid?.indexAt(pt.x, pt.y) ?? -1
  if (i >= 0) menu(items.value[i], { clientX: pt.x, clientY: pt.y })
}

function closeLook() {
  const i = items.value.indexOf(look.value)
  look.value = null
  lookItems.value = null
  if (i >= 0) grid?.reveal(i)
}

let resizeObserver = null
onMounted(() => {
  grid = new PhotoGrid(scroller.value, {
    level: level.value,
    onOpen: i => { look.value = items.value[i] },
    onMenu: (i, at) => menu(items.value[i], at),
    onToggle,
    onDrag,
    onLevel: saveLevel,
    onView: v => { view.value = v },
    chipLabel,
  })
  resizeObserver = new ResizeObserver(measure)
  if (head.value) resizeObserver.observe(head.value)
  if (gallery.value) resizeObserver.observe(gallery.value)
  window.addEventListener('keydown', onKey)
  document.addEventListener('gesturestart', stopPageZoom)
  load()
})
onUnmounted(() => {
  resizeObserver?.disconnect()
  window.removeEventListener('keydown', onKey)
  document.removeEventListener('gesturestart', stopPageZoom)
  grid?.destroy()
  grid = null
})
</script>

<template>
  <section ref="gallery" class="gallery">
    <div ref="scroller" v-long-press="onLongPress" class="content" />

    <!-- The title over the photos, like Photos: they blur and darken behind it. -->
    <header ref="head" class="head">
      <a href="/" class="back"><ChevronLeft :size="20" :stroke-width="2.4" /> TPCloud</a>
      <div class="head-row">
        <h1 class="head-title">{{ title }}</h1>
        <button v-if="items.length" type="button" class="select-btn glass" :class="{ on: selecting }" @click="selecting ? endSelect() : startSelect()">{{ selecting ? t('Cancel', 'Cancelar') : t('Select', 'Seleccionar') }}</button>
        <button v-if="album && album.kind !== 'favorites'" type="button" class="album-more glass" :title="t('Album options', 'Opciones del álbum')" @click="openAlbumMenu"><MoreHorizontal :size="18" :stroke-width="2.2" /></button>
        <div v-if="items.length" class="zoom glass" role="group" :aria-label="t('Zoom', 'Zoom')">
          <button type="button" class="zoom-btn" :disabled="level >= maxLevel" :aria-label="t('Smaller', 'Más pequeñas')" @click="zoom(1)">
            <Minus :size="16" :stroke-width="2.4" />
          </button>
          <input
            class="zoom-slider"
            type="range"
            min="0"
            :max="maxLevel"
            step="1"
            :value="maxLevel - level"
            :aria-label="t('Photo size', 'Tamaño de las fotos')"
            @input="onSlider"
          />
          <button type="button" class="zoom-btn" :disabled="level <= 0" :aria-label="t('Bigger', 'Más grandes')" @click="zoom(-1)">
            <Plus :size="16" :stroke-width="2.4" />
          </button>
        </div>
      </div>
      <p class="head-sub">{{ subtitle || ' ' }}</p>
    </header>

    <div v-show="!selecting" ref="filtersEl" class="filters glass"><Segmented v-model="segment" :options="filters" /></div>

    <Transition name="bar">
      <div v-if="selecting" ref="selectBar" class="select-bar">
        <button type="button" class="bar-round glass" :disabled="!selectedCount" :title="t('Download the originals', 'Descargar los originales')" @click="downloadSelected"><Download :size="19" /></button>
        <button type="button" class="bar-count glass" :disabled="!selectedCount" @click="showSelected">
          {{ selectedCount ? t(`Show Selected (${numberFormat.format(selectedCount)})`, `Ver seleccionadas (${numberFormat.format(selectedCount)})`) : t('Select Items', 'Selecciona fotos') }}
        </button>
        <button type="button" class="bar-round glass" :title="t('More', 'Más')" @click="selectionMenu"><MoreHorizontal :size="19" /></button>
        <button type="button" class="bar-round glass danger" :disabled="!selectedCount" :title="t('Delete', 'Eliminar')" @click="deleteSelected"><Trash2 :size="18" /></button>
      </div>
    </Transition>

    <div v-if="loading" class="state"><Spinner :size="26" /></div>
    <div v-else-if="error" class="state">
      <p class="headline">{{ error }}</p>
      <button type="button" class="btn btn-bordered small" @click="load">{{ t('Try again', 'Reintentar') }}</button>
    </div>
    <div v-else-if="loaded && !items.length" class="state">
      <template v-if="inFavorites">
        <span class="empty-icon favorites"><Heart :size="34" fill="currentColor" /></span>
        <p class="headline">{{ t('No favourites yet', 'Aún no hay favoritos') }}</p>
        <p class="subheadline secondary">{{ t('Tap the heart on a photo to keep it here.', 'Toca el corazón en una foto para tenerla aquí.') }}</p>
      </template>
      <template v-else>
        <span class="empty-icon"><Images :size="34" /></span>
        <p class="headline">{{ section === 'all' ? t('No photos yet', 'Aún no hay fotos') : t('No photos here', 'No hay fotos aquí') }}</p>
        <p v-if="section === 'all'" class="subheadline secondary">{{ t('Turn on backup in the iPhone app and they’ll show up here.', 'Activa la copia de seguridad en la app del iPhone y aparecerán aquí.') }}</p>
      </template>
    </div>

    <QuickLook
      v-if="look"
      :items="lookItems || items"
      :entry="look"
      :urls="photoMediaUrls"
      :subtitle="photoSubtitle"
      deletable
      :favorite="isFavorite"
      @favorite="toggleFavorite"
      @delete="remove"
      @change="p => { look = p }"
      @download="download"
      @close="closeLook"
    />
  </section>
</template>

<style scoped>
.gallery { position: relative; flex: 1; min-width: 0; overflow: hidden; touch-action: pan-x pan-y; }
.content {
  position: absolute;
  inset: 0;
  overflow-x: hidden;
  overflow-y: auto;
  overflow-anchor: none;
  overscroll-behavior: contain;
  touch-action: pan-y;
  -webkit-user-select: none;
  user-select: none;
  -webkit-touch-callout: none;
}

.head {
  position: absolute;
  inset: 0 0 auto;
  z-index: 3;
  padding: 18px 24px 22px;
  pointer-events: none;
}
/* Photos' top edge: the photos blur and darken behind the title and fade back in below it. */
.head::before {
  content: '';
  position: absolute;
  inset: 0 0 -18px;
  z-index: -1;
  background: linear-gradient(to bottom, color-mix(in srgb, var(--bg) 82%, transparent), color-mix(in srgb, var(--bg) 40%, transparent));
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
  mask-image: linear-gradient(to bottom, #000 55%, transparent);
  -webkit-mask-image: linear-gradient(to bottom, #000 55%, transparent);
}
.head > *, .head-row > * { pointer-events: auto; }
.head-row { display: flex; align-items: center; gap: 16px; pointer-events: none; }
.head-title { flex: 1; min-width: 0; font-family: var(--font-display); font-size: 30px; font-weight: 800; letter-spacing: -0.6px; line-height: 1.15; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.head-sub { margin-top: 2px; font-size: 15px; font-weight: 600; color: var(--label-2); font-variant-numeric: tabular-nums; }
.back { display: none; }

.select-btn { flex-shrink: 0; height: 38px; padding: 0 16px; border-radius: 999px; font-size: 15px; font-weight: 600; color: var(--label); }
.select-btn.on { color: #fff; background: var(--accent); }

.select-bar {
  position: absolute;
  left: 50%;
  bottom: calc(18px + env(safe-area-inset-bottom));
  z-index: 3;
  display: flex;
  align-items: center;
  gap: 10px;
  transform: translateX(-50%);
}
.bar-round { width: 46px; height: 46px; flex-shrink: 0; display: grid; place-items: center; border-radius: 50%; color: var(--accent); }
.bar-round.danger { color: var(--red); }
.bar-count { height: 46px; padding: 0 20px; border-radius: 999px; font-size: 15px; font-weight: 600; color: var(--label); white-space: nowrap; font-variant-numeric: tabular-nums; }
.select-bar button:disabled { opacity: 0.45; }
.bar-count:disabled { opacity: 1; color: var(--label-2); }
.bar-enter-active { transition: transform 0.35s var(--spring), opacity 0.2s; }
.bar-leave-active { transition: transform 0.2s, opacity 0.15s; }
.bar-enter-from, .bar-leave-to { transform: translate(-50%, 16px); opacity: 0; }

.album-more { flex-shrink: 0; width: 38px; height: 38px; display: grid; place-items: center; border-radius: 50%; color: var(--label); }
.zoom { display: flex; align-items: center; gap: 2px; padding: 4px; border-radius: 999px; }
.zoom-btn { width: 30px; height: 30px; display: grid; place-items: center; border-radius: 50%; color: var(--label); }
.zoom-btn:hover:not(:disabled) { background: var(--fill-3); }
.zoom-btn:disabled { color: var(--label-3); }
.zoom-slider { width: 120px; margin: 0 4px; accent-color: var(--accent); }

.filters { display: none; }
.state { position: absolute; inset: 0; z-index: 2; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; padding: 24px; text-align: center; pointer-events: none; }
.state > * { pointer-events: auto; }
.empty-icon { width: 72px; height: 72px; display: grid; place-items: center; border-radius: 22px; color: #fff; background: linear-gradient(180deg, #ffb340, #ff7a00); margin-bottom: 6px; }
.empty-icon.favorites { background: linear-gradient(180deg, #ff5f8f, #ff2d55); }

@media (max-width: 767px) {
  .head { padding: calc(10px + env(safe-area-inset-top)) 16px 22px 20px; }
  .back { display: inline-flex; align-self: flex-start; align-items: center; gap: 2px; margin-left: -6px; font-size: 17px; color: var(--accent); }
  .head-title { font-size: 34px; }
  .zoom-slider { display: none; }
  .zoom-btn { width: 34px; height: 34px; }
  .select-btn { height: 42px; }
  .select-bar { bottom: calc(14px + env(safe-area-inset-bottom)); gap: 8px; }
  .bar-count { padding: 0 14px; }
  .filters {
    position: absolute;
    left: 50%;
    bottom: calc(14px + env(safe-area-inset-bottom));
    z-index: 3;
    display: block;
    width: min(360px, calc(100% - 40px));
    padding: 4px;
    border-radius: 14px;
    transform: translateX(-50%);
  }
}
</style>

<!-- The grid's own elements are made in JavaScript, so their styles can't be scoped. -->
<style>
.pg-spacer { position: relative; width: 100%; overflow: hidden; overflow: clip; }
/* No events on the rows: a touch's events all go to the element it began on, and a row of a grid
   left behind by a zoom is removed mid-pinch. The scroller handles taps by position anyway. */
.pg-layer { position: absolute; left: 0; top: 0; will-change: transform; pointer-events: none; }
.pg-row { position: absolute; left: 0; top: 0; display: block; pointer-events: none; }
.pg-chips { position: absolute; inset: 0; z-index: 2; pointer-events: none; }
.pg-chip {
  position: absolute;
  left: 0;
  top: 0;
  height: 26px;
  padding: 0 11px;
  display: flex;
  align-items: center;
  border-radius: 13px;
  font-size: 13px;
  font-weight: 400;
  color: #fff;
  white-space: pre;
  background: rgba(30, 30, 30, 0.55);
  backdrop-filter: blur(14px) saturate(1.6);
  -webkit-backdrop-filter: blur(14px) saturate(1.6);
  pointer-events: none;
}
.pg-chip b { font-weight: 700; }
.pg-pinned { z-index: 4; }
</style>
