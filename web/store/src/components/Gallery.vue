<script setup>
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { Download, Images } from 'lucide-vue-next'
import Spinner from '../ui/Spinner.vue'
import Segmented from '../ui/Segmented.vue'
import { openMenu } from '../menu.js'
import { vLongPress } from '../longPress.js'
import { downloadLink, photoMediaUrls } from '../mediaUrls.js'
import { kindOf } from '../fileTypes.js'
import GalleryThumb from './GalleryThumb.vue'
import QuickLook from './QuickLook.vue'

// The photos backed up from the phone, newest first and grouped by month like Photos, with the
// same light thumbnails and previews as a share page so they come up at once.
const PAGE = 200

const filter = ref('all')
const filters = [
  { value: 'all', label: 'Todas' },
  { value: 'camera', label: 'Cámara' },
  { value: 'screenshots', label: 'Capturas' },
]

const photos = ref([])
const cursor = ref(null)
const done = ref(false)
const loading = ref(false)
const error = ref('')
const look = ref(null)
const sentinel = ref(null)
const scroller = ref(null)
let generation = 0
let observer = null

const monthFormat = new Intl.DateTimeFormat('es-ES', { month: 'long', year: 'numeric' })
const dayFormat = new Intl.DateTimeFormat('es-ES', { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric', hour: '2-digit', minute: '2-digit' })

function toItem(p) {
  const date = p.taken_at ? new Date(p.taken_at) : null
  return {
    ...p,
    type: 'file',
    name: p.original_filename || p.path.split('/').at(-1),
    date,
    month: date ? monthFormat.format(date).replace(/^./, c => c.toUpperCase()) : 'Sin fecha',
  }
}

const months = computed(() => {
  const groups = []
  for (const p of photos.value) {
    const last = groups.at(-1)
    if (last && last.label === p.month) last.items.push(p)
    else groups.push({ label: p.month, items: [p] })
  }
  return groups
})

async function loadMore() {
  if (loading.value || done.value) return
  loading.value = true
  error.value = ''
  const mine = generation
  try {
    const params = new URLSearchParams({ filter: filter.value, limit: String(PAGE) })
    if (cursor.value) params.set('cursor', cursor.value)
    const res = await fetch(`/store/api/photos?${params}`)
    if (mine !== generation) return
    if (!res.ok) { error.value = 'No se han podido cargar las fotos.'; return }
    const data = await res.json()
    photos.value = photos.value.concat(data.items.map(toItem))
    cursor.value = data.next_cursor
    done.value = !data.next_cursor
  } catch {
    if (mine === generation) error.value = 'No hay conexión. Vuelve a intentarlo.'
  } finally {
    if (mine === generation) {
      loading.value = false
      // A short first page may leave the sentinel on screen: keep going until it isn't.
      nextTick(checkSentinel)
    }
  }
}

function checkSentinel() {
  const el = sentinel.value
  if (!el || done.value || loading.value || error.value) return
  if (el.getBoundingClientRect().top < window.innerHeight + 800) loadMore()
}

function reset() {
  generation++
  photos.value = []
  cursor.value = null
  done.value = false
  loading.value = false
  scroller.value?.scrollTo({ top: 0 })
  loadMore()
}
watch(filter, reset)

function isVideo(p) {
  return (p.mime_type || '').startsWith('video/') || kindOf(p) === 'video'
}

function subtitle(p) {
  return p.date ? dayFormat.format(p.date).replace(/^./, c => c.toUpperCase()) : ''
}

function download(p) {
  downloadLink(photoMediaUrls.download(p), p.name)
}

function menu(p, at) {
  openMenu(at, [
    { label: 'Descargar el original', icon: Download, action: () => download(p) },
  ], p.name)
}

function onContext(p, e) {
  e.preventDefault()
  menu(p, e)
}

onMounted(() => {
  observer = new IntersectionObserver(([e]) => { if (e.isIntersecting) loadMore() }, { root: scroller.value, rootMargin: '800px 0px' })
  if (sentinel.value) observer.observe(sentinel.value)
  loadMore()
})
onUnmounted(() => observer?.disconnect())
</script>

<template>
  <section class="gallery">
    <header class="bar">
      <h1 class="bar-title">Galería</h1>
      <Segmented v-model="filter" :options="filters" class="filters" />
    </header>

    <div ref="scroller" class="content">
      <div class="page-head">
        <h1 class="large-title">Galería</h1>
        <Segmented v-model="filter" :options="filters" class="filters" />
      </div>

      <div v-if="!photos.length && loading" class="state"><Spinner :size="26" /></div>
      <div v-else-if="!photos.length && error" class="state">
        <p class="headline">{{ error }}</p>
        <button type="button" class="btn btn-bordered small" @click="loadMore">Reintentar</button>
      </div>
      <div v-else-if="!photos.length && done" class="state">
        <span class="empty-icon"><Images :size="34" /></span>
        <p class="headline">{{ filter === 'all' ? 'Aún no hay fotos' : 'No hay fotos aquí' }}</p>
        <p v-if="filter === 'all'" class="subheadline secondary">Activa la copia de seguridad en la app del iPhone y aparecerán aquí.</p>
      </div>

      <section v-for="m in months" :key="m.label" class="month">
        <h2 class="month-title">{{ m.label }}</h2>
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
          </button>
        </div>
      </section>

      <div ref="sentinel" class="sentinel">
        <Spinner v-if="photos.length && loading" :size="20" />
        <button v-else-if="photos.length && error" type="button" class="btn btn-bordered small" @click="loadMore">Reintentar</button>
      </div>
    </div>

    <QuickLook
      v-if="look"
      :items="photos"
      :entry="look"
      :urls="photoMediaUrls"
      :subtitle="subtitle"
      @change="p => { look = p; if (photos.indexOf(p) > photos.length - 10) loadMore() }"
      @download="download"
      @close="look = null"
    />
  </section>
</template>

<style scoped>
.gallery { flex: 1; min-width: 0; display: flex; flex-direction: column; overflow: hidden; }
.bar { height: 64px; flex-shrink: 0; display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 0 24px; }
.bar-title { font-size: 17px; font-weight: 700; letter-spacing: -0.3px; }
.bar .filters { width: 300px; }
.content { flex: 1; overflow-y: auto; padding: 0 24px 40px; }
.page-head { display: none; }
.state { display: flex; flex-direction: column; align-items: center; gap: 10px; padding: 80px 24px; text-align: center; }
.empty-icon { width: 72px; height: 72px; display: grid; place-items: center; border-radius: 22px; color: #fff; background: linear-gradient(180deg, #ffb340, #ff7a00); margin-bottom: 6px; }

.month { margin-bottom: 26px; }
.month-title {
  position: sticky;
  top: 0;
  z-index: 2;
  margin: 0 -24px 8px;
  padding: 8px 28px;
  font-size: 20px;
  font-weight: 700;
  letter-spacing: -0.4px;
  background: color-mix(in srgb, var(--bg) 82%, transparent);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
}
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 3px; }
.photo { aspect-ratio: 1; border-radius: 6px; background: var(--fill-3); overflow: hidden; transition: opacity 0.15s, transform 0.25s; -webkit-touch-callout: none; }
.photo:hover { opacity: 0.88; }
.photo.long-pressing { transform: scale(0.95); }
.sentinel { min-height: 60px; display: grid; place-items: center; }

@media (max-width: 767px) {
  .bar { display: none; }
  .content { padding: calc(20px + env(safe-area-inset-top)) 0 calc(110px + env(safe-area-inset-bottom)); }
  .page-head { display: flex; flex-direction: column; gap: 14px; padding: 24px 20px 18px; }
  .month-title { margin: 0 0 6px; padding: 8px 20px; font-size: 18px; }
  .grid { grid-template-columns: repeat(3, 1fr); gap: 2px; }
  .photo { border-radius: 0; }
}
</style>
