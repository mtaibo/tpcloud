<script setup>
import { computed, markRaw, nextTick, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue'
import { ChevronLeft, Download, Heart, HeartOff, Images, Minus, Plus, Trash2 } from 'lucide-vue-next'
import Spinner from '@ui/Spinner.vue'
import Segmented from '@ui/Segmented.vue'
import { openMenu } from '../menu.js'
import { confirmDestructive, toast } from '@ui/dialog.js'
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
const props = defineProps({ albums: { type: Array, default: () => [] } })
const emit = defineEmits(['albums-changed'])

const titles = () => ({ all: t('Library', 'Biblioteca'), camera: t('Camera', 'Cámara'), screenshots: t('Screenshots', 'Capturas') })
const albumId = computed(() => (section.value.startsWith('album:') ? Number(section.value.slice(6)) : null))
const album = computed(() => props.albums.find(a => a.id === albumId.value) || null)
const favoritesId = ref(null)
const inFavorites = computed(() => albumId.value !== null && albumId.value === favoritesId.value)
const title = computed(() => (album.value?.kind === 'favorites' ? t('Favourites', 'Favoritos') : album.value?.name) || titles()[section.value] || t('Library', 'Biblioteca'))

const filters = computed(() => [
  { value: 'all', label: t('All', 'Todas') },
  { value: 'camera', label: t('Camera', 'Cámara') },
  { value: 'screenshots', label: t('Screenshots', 'Capturas') },
  ...(favoritesId.value ? [{ value: `album:${favoritesId.value}`, label: t('Favourites', 'Favoritos') }] : []),
])

// Oldest first, as on screen. Plain objects (not reactive): there are thousands of them and the
// grid reads them every frame.
const items = shallowRef([])
const loading = ref(false)
const loaded = ref(false)
const error = ref('')
const look = shallowRef(null)
const favVersion = ref(0) // bumped when a heart changes, so Quick Look shows it
const level = ref(readLevel())
const view = ref({ first: -1, last: -1, atNewest: true })
const maxLevel = ref(LEVELS.length - 1) // 61 columns only on a big screen

const scroller = ref(null)
const head = ref(null)
const filtersEl = ref(null)
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
watch(section, load)

// The title floats over the photos, and on a phone the filters do too: the grid leaves room for
// them at its ends.
function measure() {
  if (!grid || !gallery.value) return
  if (scroller.value) maxLevel.value = maxLevelFor(scroller.value.clientWidth)
  const top = head.value ? head.value.offsetHeight : 0
  let bottom = 4
  if (filtersEl.value && getComputedStyle(filtersEl.value).display !== 'none') {
    bottom = gallery.value.getBoundingClientRect().bottom - filtersEl.value.getBoundingClientRect().top + 8
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
  if (look.value || e.metaKey || e.ctrlKey || e.altKey) return
  if (['INPUT', 'TEXTAREA', 'SELECT'].includes(document.activeElement?.tagName)) return
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
  const list = items.value
  const i = list.indexOf(p)
  if (i < 0) return
  if (look.value === p) look.value = list[i + 1] || list[i - 1] || null
  const next = list.slice(0, i).concat(list.slice(i + 1))
  items.value = next
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

function menu(p, at) {
  openMenu(at, [
    favoritesId.value && (p.favorite
      ? { label: t('Remove from Favourites', 'Quitar de Favoritos'), icon: HeartOff, action: () => toggleFavorite(p) }
      : { label: t('Add to Favourites', 'Añadir a Favoritos'), icon: Heart, action: () => toggleFavorite(p) }),
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
  if (i >= 0) grid?.reveal(i)
}

let resizeObserver = null
onMounted(() => {
  grid = new PhotoGrid(scroller.value, {
    level: level.value,
    onOpen: i => { look.value = items.value[i] },
    onMenu: (i, at) => menu(items.value[i], at),
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

    <div ref="filtersEl" class="filters glass"><Segmented v-model="section" :options="filters" /></div>

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
      :items="items"
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
