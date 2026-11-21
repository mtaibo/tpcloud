<script setup>
// PDFs drawn with pdf.js instead of the browser's own viewer: inside an iframe Safari on iOS shows
// only the first page and desktop Safari can't fit it, while this looks the same everywhere.
// Pages are drawn only near the screen and dropped when they leave it, so long documents don't
// run out of canvas memory on a phone. Needs pdfjs-dist in the app and pdfAssets() in its vite config.
import { nextTick, onMounted, onUnmounted, ref, shallowRef } from 'vue'
import workerUrl from 'pdfjs-dist/legacy/build/pdf.worker.min.mjs?url'
import Icon from './Icon.vue'
import Spinner from './Spinner.vue'
import { t } from './i18n.js'

const props = defineProps({ url: { type: String, required: true } })

const MAX_WIDTH = 960
const MAX_PIXELS = 16_000_000 // iOS refuses to draw a single canvas bigger than this
const ZOOMS = [0.5, 0.75, 1, 1.25, 1.5, 2, 3, 4]

const scroller = ref(null)
const pages = shallowRef([]) // page sizes at scale 1: { w, h }
const width = ref(0)
const zoomIndex = ref(ZOOMS.indexOf(1))
const current = ref(1)
const loading = ref(true)
const failed = ref(false)

const wrappers = []
const canvases = []
const drawn = new Map() // page number → { width, task }
let doc = null
let task = null
let observer = null
let resizer = null
let redrawTimer = 0
const near = new Set()

const assetUrl = (dir) => new URL(`${import.meta.env.BASE_URL}pdfjs/${dir}/`, location.href).href

function fitWidth() {
  const box = scroller.value?.clientWidth || window.innerWidth
  const gutter = box < 600 ? 8 : 24
  return Math.round(Math.min(box - gutter * 2, MAX_WIDTH) * ZOOMS[zoomIndex.value])
}

async function draw(num) {
  const canvas = canvases[num - 1]
  const size = pages.value[num - 1]
  if (!doc || !canvas || !size) return
  const cssWidth = width.value
  const prev = drawn.get(num)
  if (prev?.width === cssWidth) return
  prev?.task?.cancel()
  const entry = { width: cssWidth, task: null }
  drawn.set(num, entry)

  const page = await doc.getPage(num)
  if (drawn.get(num) !== entry) return
  let scale = (cssWidth / size.w) * Math.min(window.devicePixelRatio || 1, 3)
  const area = size.w * size.h * scale * scale
  if (area > MAX_PIXELS) scale *= Math.sqrt(MAX_PIXELS / area)
  const viewport = page.getViewport({ scale })

  // Draw off screen and swap, so a page being redrawn after a zoom doesn't flash white.
  const off = document.createElement('canvas')
  off.width = Math.floor(viewport.width)
  off.height = Math.floor(viewport.height)
  entry.task = page.render({ canvas: off, viewport })
  try {
    await entry.task.promise
  } catch {
    return // cancelled
  }
  if (drawn.get(num) !== entry) return
  canvas.width = off.width
  canvas.height = off.height
  canvas.getContext('2d').drawImage(off, 0, 0)
  off.width = off.height = 0
}

function release(num) {
  drawn.get(num)?.task?.cancel()
  drawn.delete(num)
  const canvas = canvases[num - 1]
  if (canvas) canvas.width = canvas.height = 0
}

function redrawNear() {
  for (const num of near) draw(num)
}

function onIntersect(entries) {
  for (const e of entries) {
    const num = Number(e.target.dataset.page)
    if (e.isIntersecting) {
      near.add(num)
      draw(num)
    } else {
      near.delete(num)
      release(num)
    }
  }
}

function onScroll() {
  const el = scroller.value
  if (!el) return
  const line = el.scrollTop + el.clientHeight / 3
  let page = 1
  for (let i = 0; i < wrappers.length; i++) {
    if (wrappers[i] && wrappers[i].offsetTop <= line) page = i + 1
    else break
  }
  current.value = page
}

// Changes the width and keeps the same spot of the document on screen.
async function relayout() {
  const el = scroller.value
  const next = fitWidth()
  if (!el || next === width.value) return
  const ratioY = el.scrollHeight ? el.scrollTop / el.scrollHeight : 0
  const ratioX = el.scrollWidth > el.clientWidth ? (el.scrollLeft + el.clientWidth / 2) / el.scrollWidth : 0.5
  width.value = next
  await nextTick()
  el.scrollTop = ratioY * el.scrollHeight
  el.scrollLeft = Math.max(0, ratioX * el.scrollWidth - el.clientWidth / 2)
  clearTimeout(redrawTimer)
  redrawTimer = setTimeout(redrawNear, 120)
}

function zoom(step) {
  const next = Math.min(ZOOMS.length - 1, Math.max(0, zoomIndex.value + step))
  if (next === zoomIndex.value) return
  zoomIndex.value = next
  relayout()
}

function onKey(e) {
  if (!(e.metaKey || e.ctrlKey)) return
  if (e.key === '+' || e.key === '=') zoom(1)
  else if (e.key === '-') zoom(-1)
  else if (e.key === '0') { zoomIndex.value = ZOOMS.indexOf(1); relayout() }
  else return
  e.preventDefault()
}

onMounted(async () => {
  window.addEventListener('keydown', onKey)
  try {
    const pdfjs = await import('pdfjs-dist/legacy/build/pdf.mjs')
    pdfjs.GlobalWorkerOptions.workerSrc = workerUrl
    task = pdfjs.getDocument({
      url: props.url,
      withCredentials: true,
      isEvalSupported: false,
      cMapUrl: assetUrl('cmaps'),
      standardFontDataUrl: assetUrl('standard_fonts'),
      wasmUrl: assetUrl('wasm'),
      iccUrl: assetUrl('iccs'),
    })
    doc = await task.promise
    const sizes = []
    for (let n = 1; n <= doc.numPages; n++) {
      const vp = (await doc.getPage(n)).getViewport({ scale: 1 })
      sizes.push({ w: vp.width, h: vp.height })
    }
    width.value = fitWidth()
    pages.value = sizes
    loading.value = false
    await nextTick()
    observer = new IntersectionObserver(onIntersect, { root: scroller.value, rootMargin: '150% 0px' })
    wrappers.forEach((w) => w && observer.observe(w))
    resizer = new ResizeObserver(relayout)
    resizer.observe(scroller.value)
  } catch (err) {
    if (err?.name !== 'AbortException') {
      console.error(err)
      failed.value = true
      loading.value = false
    }
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', onKey)
  clearTimeout(redrawTimer)
  observer?.disconnect()
  resizer?.disconnect()
  for (const num of [...drawn.keys()]) release(num)
  task?.destroy()
})
</script>

<template>
  <div class="pdf">
    <div v-if="loading" class="pdf-center"><Spinner :size="26" /></div>
    <div v-else-if="failed" class="pdf-center pdf-error">
      <p>{{ t("This PDF can't be shown here.", 'No se puede mostrar este PDF aquí.') }}</p>
      <a :href="url" target="_blank" rel="noopener">{{ t('Open in the browser', 'Abrir en el navegador') }}</a>
    </div>
    <div v-show="!loading && !failed" ref="scroller" class="pdf-scroll" @scroll.passive="onScroll">
      <div
        v-for="(p, i) in pages"
        :key="i"
        :ref="(el) => (wrappers[i] = el)"
        :data-page="i + 1"
        class="pdf-page"
        :style="{ width: `${width}px`, height: `${Math.round((width * p.h) / p.w)}px` }"
      >
        <canvas :ref="(el) => (canvases[i] = el)" />
      </div>
    </div>
    <div v-if="!loading && !failed" class="pdf-bar">
      <span class="pdf-count">{{ current }} / {{ pages.length }}</span>
      <button class="pdf-btn" :disabled="zoomIndex === 0" :aria-label="t('Zoom out', 'Alejar')" @click="zoom(-1)">
        <Icon name="minus" :size="18" />
      </button>
      <button class="pdf-btn" :disabled="zoomIndex === ZOOMS.length - 1" :aria-label="t('Zoom in', 'Acercar')" @click="zoom(1)">
        <Icon name="plus" :size="18" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.pdf {
  position: relative;
  width: 100%;
  height: 100%;
  background: #3a3a3c;
}
.pdf-scroll {
  position: absolute;
  inset: 0;
  overflow: auto;
  -webkit-overflow-scrolling: touch;
  overscroll-behavior: contain;
  padding: max(12px, env(safe-area-inset-top)) 0 calc(72px + env(safe-area-inset-bottom));
}
.pdf-page {
  margin: 0 auto 12px;
  background: #fff;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.35);
}
.pdf-page canvas {
  display: block;
  width: 100%;
  height: 100%;
}
.pdf-center {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: #fff;
}
.pdf-error a { color: var(--blue); }
.pdf-bar {
  position: absolute;
  left: 50%;
  bottom: calc(16px + env(safe-area-inset-bottom));
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 4px 4px 4px 14px;
  border-radius: 999px;
  background: rgba(28, 28, 30, 0.82);
  -webkit-backdrop-filter: blur(20px);
  backdrop-filter: blur(20px);
  color: #fff;
  font: 500 14px var(--font);
  font-variant-numeric: tabular-nums;
}
.pdf-count { padding-right: 8px; }
.pdf-btn {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  border: 0;
  border-radius: 50%;
  background: transparent;
  color: inherit;
  cursor: pointer;
}
.pdf-btn:hover:not(:disabled) { background: rgba(255, 255, 255, 0.14); }
.pdf-btn:disabled { opacity: 0.35; cursor: default; }
</style>
