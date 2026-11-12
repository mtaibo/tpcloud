<script setup>
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { ChevronLeft, Download, Heart, HeartOff, Images, Trash2 } from 'lucide-vue-next'
import Spinner from '../ui/Spinner.vue'
import Segmented from '../ui/Segmented.vue'
import { openMenu } from '../menu.js'
import { confirmDestructive, toast } from '../ui/dialog.js'
import { vLongPress } from '../longPress.js'
import { downloadLink, photoMediaUrls } from '../mediaUrls.js'
import { kindOf } from '../fileTypes.js'
import GalleryThumb from './GalleryThumb.vue'
import QuickLook from './QuickLook.vue'
import { locale, t } from '../ui/i18n.js'

// The photos backed up from the phone, grouped by month like Photos: newest at the bottom, where
// the view starts, and older ones loading in above as you scroll up. Same light thumbnails and
// previews as a share page so they come up at once.
const PAGE = 200

// Which photos: "all", "camera", "screenshots" or "album:<id>", picked in the sidebar on a
// computer and with the segmented control on a phone.
const section = defineModel('section', { type: String, default: 'all' })
const props = defineProps({ albums: { type: Array, default: () => [] } })
const emit = defineEmits(['albums-changed'])

const titles = () => ({ all: t('Photos', 'Galería'), camera: t('Camera', 'Cámara'), screenshots: t('Screenshots', 'Capturas') })
const albumId = computed(() => (section.value.startsWith('album:') ? Number(section.value.slice(6)) : null))
const album = computed(() => props.albums.find(a => a.id === albumId.value) || null)
const favoritesId = ref(null)
const inFavorites = computed(() => albumId.value !== null && albumId.value === favoritesId.value)
const title = computed(() => (album.value?.kind === 'favorites' ? t('Favourites', 'Favoritos') : album.value?.name) || titles()[section.value] || t('Photos', 'Galería'))

const filters = computed(() => [
  { value: 'all', label: t('All', 'Todas') },
  { value: 'camera', label: t('Camera', 'Cámara') },
  { value: 'screenshots', label: t('Screenshots', 'Capturas') },
  ...(favoritesId.value ? [{ value: `album:${favoritesId.value}`, label: t('Favourites', 'Favoritos') }] : []),
])

const photos = ref([])
const cursor = ref(null)
const done = ref(false)
const loading = ref(false)
const error = ref('')
const look = ref(null)
const sentinel = ref(null)
const scroller = ref(null)
const current = ref('') // the month at the top of the view, shown under the title
let generation = 0
let observer = null

const monthName = new Intl.DateTimeFormat(locale(), { month: 'long' })
const dayFormat = new Intl.DateTimeFormat(locale(), { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric', hour: '2-digit', minute: '2-digit' })

function toItem(p) {
  const date = p.taken_at ? new Date(p.taken_at) : null
  return {
    ...p,
    type: 'file',
    // Its readable name on the server (ID123_20240229_1755.heic), which downloads keep too.
    name: p.path.split('/').at(-1),
    date,
    month: date ? monthName.format(date).replace(/^./, c => c.toUpperCase()) : t('No date', 'Sin fecha'),
    year: date ? String(date.getFullYear()) : '',
  }
}

// The API pages newest first; on screen it's oldest first, so the newest photo is the last tile.
const ordered = computed(() => photos.value.slice().reverse())

const months = computed(() => {
  const groups = []
  for (const p of ordered.value) {
    const key = `${p.year}-${p.month}`
    const last = groups.at(-1)
    if (last && last.key === key) last.items.push(p)
    else groups.push({ key, month: p.month, year: p.year, items: [p] })
  }
  return groups
})

async function loadMore() {
  if (loading.value || done.value) return
  loading.value = true
  error.value = ''
  const mine = generation
  try {
    const params = new URLSearchParams({ limit: String(PAGE) })
    if (albumId.value !== null) params.set('album', String(albumId.value))
    else params.set('filter', section.value)
    if (cursor.value) params.set('cursor', cursor.value)
    const res = await fetch(`/store/api/photos?${params}`)
    if (mine !== generation) return
    if (!res.ok) { error.value = t('The photos couldn’t be loaded.', 'No se han podido cargar las fotos.'); return }
    const data = await res.json()
    favoritesId.value = data.favorites_album ?? favoritesId.value
    // Older photos go in above what's on screen: keep the distance to the bottom so the view
    // doesn't move. The first page instead starts at the bottom, on the newest.
    const el = scroller.value
    const first = !photos.value.length
    const fromBottom = el ? el.scrollHeight - el.scrollTop : 0
    photos.value = photos.value.concat(data.items.map(toItem))
    cursor.value = data.next_cursor
    done.value = !data.next_cursor
    await nextTick()
    if (el) el.scrollTop = first ? el.scrollHeight : el.scrollHeight - fromBottom
    updateCurrent()
  } catch {
    if (mine === generation) error.value = t('No connection. Try again.', 'No hay conexión. Vuelve a intentarlo.')
  } finally {
    if (mine === generation) {
      loading.value = false
      // A short first page may leave the sentinel on screen: keep going until it isn't.
      nextTick(checkSentinel)
    }
  }
}

// The sentinel sits above the oldest month: load more once it comes within 800 px of the top.
function checkSentinel() {
  const el = sentinel.value
  if (!el || !scroller.value || done.value || loading.value || error.value || !photos.value.length) return
  if (el.getBoundingClientRect().bottom > scroller.value.getBoundingClientRect().top - 800) loadMore()
}

// Which month is at the top of the view (under the header on a phone).
let frame = 0
function updateCurrent() {
  cancelAnimationFrame(frame)
  frame = requestAnimationFrame(() => {
    const el = scroller.value
    if (!el) return
    const top = el.getBoundingClientRect().top + (parseFloat(getComputedStyle(el).scrollPaddingTop) || 0)
    let label = ''
    for (const s of el.querySelectorAll('.month')) {
      if (s.getBoundingClientRect().bottom > top + 1) { label = s.dataset.label; break }
    }
    current.value = label
  })
}

function reset() {
  generation++
  photos.value = []
  cursor.value = null
  done.value = false
  loading.value = false
  current.value = ''
  loadMore()
}
watch(section, reset)

function isVideo(p) {
  return (p.mime_type || '').startsWith('video/') || kindOf(p) === 'video'
}

function subtitle(p) {
  return p.date ? dayFormat.format(p.date).replace(/^./, c => c.toUpperCase()) : ''
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
  if (!res.ok) { toast('No se ha podido eliminar', 'x-circle'); return }
  dropFromList(p)
  toast('Foto eliminada')
  if (p.favorite) emit('albums-changed')
}

function dropFromList(p) {
  const i = photos.value.indexOf(p)
  if (look.value === p) look.value = photos.value[i + 1] || photos.value[i - 1] || null
  photos.value = photos.value.filter(x => x !== p)
}

// The heart, as in Photos: adds it to Favoritos or takes it out (and out of view, when looking
// at Favoritos). The tile changes straight away and goes back if the server says no.
async function toggleFavorite(p) {
  if (!favoritesId.value) return
  const on = !p.favorite
  p.favorite = on
  const res = await fetch(`/store/api/albums/${favoritesId.value}/photos/${p.id}`, { method: on ? 'PUT' : 'DELETE' }).catch(() => null)
  if (!res?.ok) {
    p.favorite = !on
    toast('No se ha podido cambiar', 'x-circle')
    return
  }
  if (!on && inFavorites.value) dropFromList(p)
  emit('albums-changed')
}

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

function onContext(p, e) {
  e.preventDefault()
  menu(p, e)
}

onMounted(() => {
  observer = new IntersectionObserver(([e]) => { if (e.isIntersecting && photos.value.length) loadMore() }, { root: scroller.value, rootMargin: '800px 0px' })
  if (sentinel.value) observer.observe(sentinel.value)
  loadMore()
})
onUnmounted(() => { observer?.disconnect(); cancelAnimationFrame(frame) })
</script>

<template>
  <section class="gallery">
    <header class="bar">
      <h1 class="bar-title">{{ title }}</h1>
      <span v-if="current" class="bar-sub">{{ current }}</span>
    </header>

    <!-- On a phone: the title floats over the photos, like Photos, and the filters at the bottom. -->
    <div class="page-head">
      <a href="/" class="back"><ChevronLeft :size="20" :stroke-width="2.4" /> TPCloud</a>
      <h1 class="large-title">{{ title }}</h1>
      <p class="head-sub">{{ current || '\u00a0' }}</p>
    </div>
    <div class="filters glass"><Segmented v-model="section" :options="filters" /></div>

    <div ref="scroller" class="content" @scroll.passive="updateCurrent">
      <div ref="sentinel" class="sentinel">
        <Spinner v-if="photos.length && loading" :size="20" />
        <button v-else-if="photos.length && error" type="button" class="btn btn-bordered small" @click="loadMore">{{ t('Try again', 'Reintentar') }}</button>
      </div>

      <div v-if="!photos.length && loading" class="state"><Spinner :size="26" /></div>
      <div v-else-if="!photos.length && error" class="state">
        <p class="headline">{{ error }}</p>
        <button type="button" class="btn btn-bordered small" @click="loadMore">{{ t('Try again', 'Reintentar') }}</button>
      </div>
      <div v-else-if="!photos.length && done" class="state">
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

      <section v-for="m in months" :key="m.key" class="month" :data-label="`${m.month} ${m.year}`.trim()">
        <h2 class="month-title">{{ m.month }} <span>{{ m.year }}</span></h2>
        <div class="grid">
          <button
            v-for="p in m.items"
            :key="p.id"
            v-long-press="pt => menu(p, { clientX: pt.x, clientY: pt.y })"
            type="button"
            class="photo"
            :aria-label="p.name"
            @click="look = p"
            @contextmenu="onContext(p, $event)"
          >
            <GalleryThumb :src="photoMediaUrls.thumbnail(p)" :alt="p.name" :type="isVideo(p) ? 'video' : 'image'" />
            <Heart v-if="p.favorite" class="fav-badge" :size="15" fill="currentColor" :stroke-width="0" />
          </button>
        </div>
      </section>
    </div>

    <QuickLook
      v-if="look"
      :items="ordered"
      :entry="look"
      :urls="photoMediaUrls"
      :subtitle="subtitle"
      deletable
      :favorite="favoritesId ? (p => p.favorite) : null"
      @favorite="toggleFavorite"
      @delete="remove"
      @change="p => { look = p; if (ordered.indexOf(p) < 10) loadMore() }"
      @download="download"
      @close="look = null"
    />
  </section>
</template>

<style scoped>
.gallery { position: relative; flex: 1; min-width: 0; display: flex; flex-direction: column; overflow: hidden; }
.bar { height: 64px; flex-shrink: 0; display: flex; align-items: center; gap: 10px; padding: 0 24px; }
.bar-title { font-size: 17px; font-weight: 700; letter-spacing: -0.3px; }
.bar-sub { font-size: 13px; color: var(--label-2); font-variant-numeric: tabular-nums; }
/* overflow-anchor off: loadMore keeps the place itself when older months go in above. */
.content { flex: 1; overflow-y: auto; overflow-anchor: none; padding: 0 24px 24px; }
.page-head, .filters { display: none; }
.state { display: flex; flex-direction: column; align-items: center; gap: 10px; padding: 80px 24px; text-align: center; }
.empty-icon { width: 72px; height: 72px; display: grid; place-items: center; border-radius: 22px; color: #fff; background: linear-gradient(180deg, #ffb340, #ff7a00); margin-bottom: 6px; }

.month + .month { margin-top: 22px; }
.month-title { margin: 0 0 8px 2px; font-size: 20px; font-weight: 700; letter-spacing: -0.4px; }
.month-title span { font-weight: 600; color: var(--label-3); }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 3px; }
.photo { position: relative; aspect-ratio: 1; border-radius: 6px; background: var(--fill-3); overflow: hidden; transition: opacity 0.15s, transform 0.25s; -webkit-touch-callout: none; }
.photo:hover { opacity: 0.88; }
.photo.long-pressing { transform: scale(0.95); }
.fav-badge { position: absolute; left: 6px; bottom: 6px; color: #fff; filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.6)); pointer-events: none; }
.empty-icon.favorites { background: linear-gradient(180deg, #ff5f8f, #ff2d55); }
.sentinel { min-height: 48px; display: grid; place-items: center; }

@media (max-width: 767px) {
  .bar { display: none; }
  .content {
    --head: calc(126px + env(safe-area-inset-top));
    padding: var(--head) 0 calc(84px + env(safe-area-inset-bottom));
    scroll-padding-top: var(--head);
  }
  .page-head {
    position: absolute;
    inset: 0 0 auto;
    z-index: 3;
    display: flex;
    flex-direction: column;
    padding: calc(10px + env(safe-area-inset-top)) 20px 30px;
    pointer-events: none;
  }
  /* Photos' top edge: the photos blur and darken behind the title and fade back in below it. */
  .page-head::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    background: color-mix(in srgb, var(--bg) 88%, transparent);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    mask-image: linear-gradient(to bottom, #000 72%, transparent);
    -webkit-mask-image: linear-gradient(to bottom, #000 72%, transparent);
  }
  .page-head > * { pointer-events: auto; }
  .back { align-self: flex-start; display: inline-flex; align-items: center; gap: 2px; margin-left: -6px; font-size: 17px; color: var(--accent); }
  .page-head .large-title { margin-top: 4px; }
  .head-sub { font-size: 15px; font-weight: 600; color: var(--label-2); font-variant-numeric: tabular-nums; }
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
  .month + .month { margin-top: 18px; }
  .month-title { margin: 0 0 6px; padding: 0 20px; font-size: 18px; }
  .grid { grid-template-columns: repeat(3, 1fr); gap: 2px; }
  .photo { border-radius: 0; }
}
</style>
