<script setup>
// TPCloud's own PDF viewer, drawn with pdf.js instead of the browser's: inside an iframe Safari on
// iOS shows only the first page and desktop Safari can't fit it, while this looks and works the
// same everywhere. Selectable text, search, thumbnails, the outline, pinch and trackpad zoom.
// Pages are drawn only near the screen and dropped when they leave it, so long documents don't
// run out of canvas memory on a phone. Needs pdfjs-dist in the app and pdfAssets() in its vite
// config. Pass onClose to show a close button (as an overlay) and onDownload to handle downloads.
import { computed, nextTick, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue'
import workerUrl from 'pdfjs-dist/legacy/build/pdf.worker.min.mjs?url'
import Icon from './Icon.vue'
import Spinner from './Spinner.vue'
import { t } from './i18n.js'

const props = defineProps({
  url: { type: String, required: true },
  name: { type: String, default: '' },
  onClose: Function,
  onDownload: Function,
})

const CSS_UNITS = 96 / 72 // 100 % = the page at its printed size
const STEPS = [25, 33, 50, 67, 75, 90, 100, 110, 125, 150, 175, 200, 250, 300, 400, 500]
const MIN_SCALE = (STEPS[0] / 100) * CSS_UNITS
const MAX_SCALE = (STEPS[STEPS.length - 1] / 100) * CSS_UNITS
const coarse = matchMedia('(pointer: coarse)').matches
// iOS refuses canvases bigger than ~16.7 MP and kills tabs that hold too much canvas memory.
const MAX_PIXELS = coarse ? 8_000_000 : 16_000_000
const PAGE_GAP = 12
const THUMB_WIDTH = 116

const root = ref(null)
const scroller = ref(null)
const pagesEl = ref(null)
const sideEl = ref(null)
const searchInput = ref(null)
const pageInput = ref(null)

const pages = shallowRef([]) // per page at scale 1: { w, h }
const scale = ref(1)
const mode = ref('auto') // 'auto' | 'width' | 'page' | null (a chosen percentage)
const current = ref(1)
const loading = ref(true)
const error = ref('')
const needsPassword = ref(false)
const wrongPassword = ref(false)
const password = ref('')
const phone = ref(false)
const sidebarOpen = ref(false)
const sideTab = ref('thumbs')
const outline = shallowRef([])
const chromeHidden = ref(false)
const zoomMenu = ref(false)
const editingPage = ref(false)
const pageDraft = ref('')

const searchOpen = ref(false)
const query = ref('')
const matches = shallowRef([]) // { page, start, end } in the page's joined text
const matchIndex = ref(-1)
const searching = ref(false)

const percent = computed(() => Math.round((scale.value / CSS_UNITS) * 100))
const hasOutline = computed(() => outline.value.length > 0)
const title = computed(() => props.name || t('PDF document', 'Documento PDF'))

let pdfjs = null
let doc = null
let loadingTask = null
let passwordCallback = null
let pageObserver = null
let thumbObserver = null
let resizeObserver = null
let redrawTimer = 0
let searchTimer = 0
const wrappers = []
const canvases = []
const textHosts = []
const thumbCanvases = []
const drawn = new Map() // page → { scale, task }
const layers = new Map() // page → { layer, divs, strs, task }
const thumbsDrawn = new Map() // page → task
const near = new Set()
const texts = new Map() // page → { strs, starts, norm, map }
let revealPending = false

const assetUrl = (dir) => new URL(`${import.meta.env.BASE_URL}pdfjs/${dir}/`, location.href).href

// ---- Layout and zoom ------------------------------------------------------------------------

function gutter() {
  return phone.value ? 6 : 28
}

function fitScale(kind) {
  const el = scroller.value
  if (!el || !pages.value.length) return 1
  // The usual page width, so one landscape page doesn't shrink every other one.
  const widths = pages.value.map((p) => p.w).sort((x, y) => x - y)
  const typical = widths[Math.floor(widths.length / 2)]
  const byWidth = (el.clientWidth - gutter() * 2) / typical
  if (kind === 'width') return byWidth
  const first = pages.value[current.value - 1] || pages.value[0]
  const byPage = Math.min(byWidth, (el.clientHeight - PAGE_GAP * 2) / first.h)
  if (kind === 'page') return byPage
  // auto: the width of the window, but no bigger than 125 % on a big screen.
  return Math.min(byWidth, 1.25 * CSS_UNITS)
}

// In the fitting modes a page wider than the rest (a landscape one) is shrunk to fit too.
const avail = ref(0)
function pageScale(i) {
  const p = pages.value[i]
  if (!mode.value || !p || !avail.value) return scale.value
  return Math.min(scale.value, avail.value / p.w)
}

const clampScale = (s) => Math.min(MAX_SCALE, Math.max(MIN_SCALE, s))

// The point of the document under (x, y) in the scroller: a page and where in it.
function anchorAt(x, y) {
  const box = scroller.value.getBoundingClientRect()
  const px = box.left + x
  const py = box.top + y
  let best = 0
  for (let i = 0; i < wrappers.length; i++) {
    const r = wrappers[i]?.getBoundingClientRect()
    if (!r) continue
    best = i
    if (r.bottom + PAGE_GAP / 2 >= py) break
  }
  const r = wrappers[best].getBoundingClientRect()
  return { i: best, fx: (px - r.left) / r.width, fy: (py - r.top) / r.height }
}

// Changes the zoom keeping the point under `from` (or the middle) at `to`.
async function setScale(next, from = null, to = from) {
  const el = scroller.value
  if (!el || !pages.value.length) return
  next = clampScale(next)
  if (Math.abs(next - scale.value) < 0.0005) return
  from ??= { x: el.clientWidth / 2, y: el.clientHeight / 2 }
  to ??= from
  const a = anchorAt(from.x, from.y)
  scale.value = next
  await nextTick()
  const box = el.getBoundingClientRect()
  const r = wrappers[a.i].getBoundingClientRect()
  el.scrollLeft += r.left + a.fx * r.width - (box.left + to.x)
  el.scrollTop += r.top + a.fy * r.height - (box.top + to.y)
  scheduleRedraw()
}

function zoomTo(kind) {
  zoomMenu.value = false
  mode.value = typeof kind === 'string' ? kind : null
  setScale(typeof kind === 'string' ? fitScale(kind) : (kind / 100) * CSS_UNITS)
  scheduleRedraw()
}

function zoomStep(dir) {
  const p = percent.value
  const next = dir > 0 ? STEPS.find((s) => s > p) : [...STEPS].reverse().find((s) => s < p)
  if (next) zoomTo(next)
}

function refit() {
  const el = scroller.value
  if (el) phone.value = el.clientWidth < 700
  if (el) avail.value = el.clientWidth - gutter() * 2
  if (mode.value) setScale(fitScale(mode.value))
}

function scheduleRedraw() {
  clearTimeout(redrawTimer)
  redrawTimer = setTimeout(() => near.forEach(draw), 140)
}

// ---- Drawing --------------------------------------------------------------------------------

async function draw(num) {
  const canvas = canvases[num - 1]
  const size = pages.value[num - 1]
  if (!doc || !canvas || !size) return
  const s = pageScale(num - 1)
  const prev = drawn.get(num)
  if (prev && Math.abs(prev.scale - s) < 0.0005) return
  prev?.task?.cancel()
  const entry = { scale: s, task: null }
  drawn.set(num, entry)

  const page = await doc.getPage(num)
  if (drawn.get(num) !== entry) return
  let px = s * Math.min(window.devicePixelRatio || 1, 3)
  const area = size.w * size.h * px * px
  if (area > MAX_PIXELS) px *= Math.sqrt(MAX_PIXELS / area)
  const viewport = page.getViewport({ scale: px })

  // Drawn off screen and swapped in, so a page redrawn after a zoom doesn't flash white.
  const off = document.createElement('canvas')
  off.width = Math.floor(viewport.width)
  off.height = Math.floor(viewport.height)
  entry.task = page.render({ canvas: off, viewport })
  try {
    await entry.task.promise
  } catch {
    off.width = off.height = 0
    return
  }
  if (drawn.get(num) === entry) {
    canvas.width = off.width
    canvas.height = off.height
    canvas.getContext('2d').drawImage(off, 0, 0)
  }
  off.width = off.height = 0
  if (!layers.has(num)) drawText(num, page)
}

async function drawText(num, page) {
  const host = textHosts[num - 1]
  if (!host || layers.has(num)) return
  const layer = new pdfjs.TextLayer({
    textContentSource: page.streamTextContent(),
    container: host,
    viewport: page.getViewport({ scale: 1 }),
  })
  const entry = { layer, divs: [], strs: [] }
  layers.set(num, entry)
  try {
    await layer.render()
  } catch {
    return
  }
  if (layers.get(num) !== entry) return
  entry.divs = layer.textDivs
  entry.strs = layer.textContentItemsStr
  const end = document.createElement('div')
  end.className = 'endOfContent'
  host.append(end)
  highlightPage(num)
}

function release(num) {
  drawn.get(num)?.task?.cancel()
  drawn.delete(num)
  const canvas = canvases[num - 1]
  if (canvas) canvas.width = canvas.height = 0
  const l = layers.get(num)
  if (l) {
    l.layer.cancel()
    layers.delete(num)
    textHosts[num - 1]?.replaceChildren()
  }
}

function onPageIntersect(entries) {
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

async function drawThumb(num) {
  const canvas = thumbCanvases[num - 1]
  const size = pages.value[num - 1]
  if (!doc || !canvas || thumbsDrawn.has(num)) return
  thumbsDrawn.set(num, null)
  const page = await doc.getPage(num)
  const viewport = page.getViewport({ scale: (THUMB_WIDTH / size.w) * Math.min(window.devicePixelRatio || 1, 2) })
  const off = document.createElement('canvas')
  off.width = Math.floor(viewport.width)
  off.height = Math.floor(viewport.height)
  const task = page.render({ canvas: off, viewport })
  thumbsDrawn.set(num, task)
  try {
    await task.promise
  } catch {
    off.width = off.height = 0
    return
  }
  if (thumbsDrawn.get(num) === task) {
    canvas.width = off.width
    canvas.height = off.height
    canvas.getContext('2d').drawImage(off, 0, 0)
  }
  off.width = off.height = 0
}

function onThumbIntersect(entries) {
  for (const e of entries) {
    const num = Number(e.target.dataset.page)
    if (e.isIntersecting) drawThumb(num)
    else {
      thumbsDrawn.get(num)?.cancel()
      thumbsDrawn.delete(num)
      const c = thumbCanvases[num - 1]
      if (c) c.width = c.height = 0
    }
  }
}

watch(sidebarOpen, async (open) => {
  thumbObserver?.disconnect()
  thumbsDrawn.forEach((task) => task?.cancel())
  thumbsDrawn.clear()
  if (!open) return
  await nextTick()
  observeThumbs()
  sideEl.value?.querySelector('.pv-thumb.on')?.scrollIntoView({ block: 'center' })
})
watch(sideTab, async () => {
  await nextTick()
  observeThumbs()
})

function observeThumbs() {
  thumbObserver?.disconnect()
  if (!sideEl.value) return
  thumbObserver = new IntersectionObserver(onThumbIntersect, { root: sideEl.value, rootMargin: '300px 0px' })
  sideEl.value.querySelectorAll('.pv-thumb').forEach((el) => thumbObserver.observe(el))
}

// ---- Navigation -----------------------------------------------------------------------------

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
  zoomMenu.value = false
}

function goToPage(num, fy = 0) {
  const el = scroller.value
  const w = wrappers[num - 1]
  if (!el || !w) return
  el.scrollTop = w.offsetTop - PAGE_GAP + fy * w.offsetHeight
  const spare = el.clientWidth - w.offsetWidth
  el.scrollLeft = Math.max(0, spare >= 0 ? w.offsetLeft - spare / 2 : w.offsetLeft - PAGE_GAP)
  current.value = num
  if (phone.value) sidebarOpen.value = false
}

function startPageEdit() {
  pageDraft.value = String(current.value)
  editingPage.value = true
  nextTick(() => pageInput.value?.select())
}

function commitPage() {
  const n = parseInt(pageDraft.value, 10)
  editingPage.value = false
  if (n >= 1 && n <= pages.value.length) goToPage(n)
}

async function openOutline(item) {
  try {
    let dest = item.dest
    if (typeof dest === 'string') dest = await doc.getDestination(dest)
    if (!Array.isArray(dest)) return
    const index = typeof dest[0] === 'number' ? dest[0] : await doc.getPageIndex(dest[0])
    const page = await doc.getPage(index + 1)
    const vp = page.getViewport({ scale: 1 })
    const top = dest[1]?.name === 'XYZ' ? dest[3] : ['FitH', 'FitBH'].includes(dest[1]?.name) ? dest[2] : null
    const fy = top == null ? 0 : Math.min(1, Math.max(0, vp.convertToViewportPoint(0, top)[1] / vp.height))
    goToPage(index + 1, fy)
  } catch { /* a broken link in the outline */ }
}

function flatten(items, depth = 0, out = []) {
  for (const item of items || []) {
    out.push({ title: item.title, dest: item.dest, depth })
    flatten(item.items, depth + 1, out)
  }
  return out
}

// ---- Search ---------------------------------------------------------------------------------

const fold = (s) => s.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase()

async function pageText(num) {
  if (texts.has(num)) return texts.get(num)
  const page = await doc.getPage(num)
  const content = await page.getTextContent()
  const strs = []
  const starts = []
  let joined = ''
  for (const item of content.items) {
    if (item.str === undefined) continue
    starts.push(joined.length)
    strs.push(item.str)
    joined += item.str
    if (item.hasEOL) joined += ' '
  }
  let norm = ''
  const map = []
  for (let k = 0; k < joined.length; k++) {
    for (const c of fold(joined[k])) {
      norm += c
      map.push(k)
    }
  }
  const text = { strs, starts, norm, map }
  texts.set(num, text)
  return text
}

async function runSearch() {
  const q = fold(query.value.trim().replace(/\s+/g, ' '))
  const run = Symbol()
  runSearch.current = run
  if (!q) {
    matches.value = []
    matchIndex.value = -1
    highlightAll()
    return
  }
  searching.value = true
  const found = []
  for (let n = 1; n <= pages.value.length; n++) {
    const { norm, map } = await pageText(n)
    if (runSearch.current !== run) return
    let at = norm.indexOf(q)
    while (at !== -1) {
      found.push({ page: n, start: map[at], end: map[at + q.length - 1] + 1 })
      at = norm.indexOf(q, at + q.length)
    }
  }
  searching.value = false
  matches.value = found
  // Start at the first match from the page being read.
  const from = found.findIndex((m) => m.page >= current.value)
  matchIndex.value = found.length ? Math.max(0, from) : -1
  highlightAll()
  reveal()
}

function onQuery() {
  clearTimeout(searchTimer)
  searchTimer = setTimeout(runSearch, 220)
}

function nextMatch(dir) {
  const n = matches.value.length
  if (!n) return
  matchIndex.value = (matchIndex.value + dir + n) % n
  highlightAll()
  reveal()
}

function openSearch() {
  searchOpen.value = true
  chromeHidden.value = false
  nextTick(() => searchInput.value?.select())
}

function closeSearch() {
  searchOpen.value = false
  query.value = ''
  runSearch()
}

function highlightAll() {
  for (const num of layers.keys()) highlightPage(num)
}

function highlightPage(num) {
  const l = layers.get(num)
  const text = texts.get(num)
  if (!l || !l.divs.length) return
  // Put back the plain text of every run marked before.
  for (const i of l.marked || []) l.divs[i].textContent = l.strs[i]
  l.marked = []
  if (!text || text.strs.length !== l.strs.length) return

  const segments = new Map() // run → [[from, to, selected]]
  matches.value.forEach((m, k) => {
    if (m.page !== num) return
    for (let i = 0; i < text.strs.length; i++) {
      const s = text.starts[i]
      const e = s + text.strs[i].length
      if (e <= m.start) continue
      if (s >= m.end) break
      if (!segments.has(i)) segments.set(i, [])
      segments.get(i).push([Math.max(m.start, s) - s, Math.min(m.end, e) - s, k === matchIndex.value])
    }
  })
  for (const [i, segs] of segments) {
    const str = l.strs[i]
    const div = l.divs[i]
    div.textContent = ''
    let at = 0
    for (const [a, b, selected] of segs) {
      if (a > at) div.append(str.slice(at, a))
      const mark = document.createElement('span')
      mark.className = selected ? 'highlight appended selected' : 'highlight appended'
      mark.textContent = str.slice(a, b)
      div.append(mark)
      at = b
    }
    if (at < str.length) div.append(str.slice(at))
    l.marked.push(i)
  }
  if (revealPending) reveal()
}

// Scrolls the selected match into view, once its page has its text drawn.
function reveal() {
  const m = matches.value[matchIndex.value]
  const el = scroller.value
  if (!m || !el) return
  const mark = textHosts[m.page - 1]?.querySelector('.highlight.selected')
  if (!mark) {
    revealPending = true
    if (!near.has(m.page)) goToPage(m.page)
    return
  }
  revealPending = false
  const box = el.getBoundingClientRect()
  const r = mark.getBoundingClientRect()
  if (r.top < box.top + 60 || r.bottom > box.bottom - 80) el.scrollTop += r.top - box.top - el.clientHeight / 3
  if (r.left < box.left || r.right > box.right) el.scrollLeft += r.left - box.left - el.clientWidth / 2
}

// ---- Gestures -------------------------------------------------------------------------------

let pinch = null
let tap = null
let lastTap = null
let tapTimer = 0

const local = (touch) => {
  const box = scroller.value.getBoundingClientRect()
  return { x: touch.clientX - box.left, y: touch.clientY - box.top }
}

function onTouchStart(e) {
  if (e.touches.length === 2) {
    e.preventDefault()
    tap = null
    const [a, b] = [local(e.touches[0]), local(e.touches[1])]
    const center = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 }
    const el = scroller.value
    pinch = { d: Math.hypot(a.x - b.x, a.y - b.y), center, now: center, factor: 1 }
    pagesEl.value.style.transformOrigin = `${el.scrollLeft + center.x - pagesEl.value.offsetLeft}px ${el.scrollTop + center.y - pagesEl.value.offsetTop}px`
  } else if (e.touches.length === 1) {
    tap = { ...local(e.touches[0]), time: Date.now() }
  }
}

function onTouchMove(e) {
  if (pinch && e.touches.length === 2) {
    e.preventDefault()
    const [a, b] = [local(e.touches[0]), local(e.touches[1])]
    pinch.now = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 }
    const f = Math.hypot(a.x - b.x, a.y - b.y) / pinch.d
    pinch.factor = clampScale(scale.value * f) / scale.value
    const dx = pinch.now.x - pinch.center.x
    const dy = pinch.now.y - pinch.center.y
    pagesEl.value.style.transform = `translate(${dx}px, ${dy}px) scale(${pinch.factor})`
  } else if (tap && e.touches.length === 1) {
    const p = local(e.touches[0])
    if (Math.hypot(p.x - tap.x, p.y - tap.y) > 10) tap = null
  }
}

function onTouchEnd(e) {
  if (pinch && e.touches.length < 2) {
    const { factor, center, now } = pinch
    pinch = null
    pagesEl.value.style.transform = ''
    mode.value = null
    setScale(scale.value * factor, center, now)
    return
  }
  if (!tap || e.touches.length || Date.now() - tap.time > 300) return
  if (window.getSelection()?.toString()) return
  const here = tap
  tap = null
  if (lastTap && here.time - lastTap.time < 320 && Math.hypot(here.x - lastTap.x, here.y - lastTap.y) < 40) {
    // Double tap: zoom in on that spot, or back to the width of the screen.
    clearTimeout(tapTimer)
    lastTap = null
    e.preventDefault()
    const fit = fitScale('width')
    if (scale.value > fit * 1.1) {
      mode.value = 'width'
      setScale(fit, here, here)
    } else {
      mode.value = null
      setScale(fit * 2.2, here, here)
    }
    return
  }
  lastTap = here
  clearTimeout(tapTimer)
  tapTimer = setTimeout(() => {
    lastTap = null
    if (phone.value) chromeHidden.value = !chromeHidden.value
  }, 320)
}

function onWheel(e) {
  if (!e.ctrlKey && !e.metaKey) return
  e.preventDefault()
  const box = scroller.value.getBoundingClientRect()
  const at = { x: e.clientX - box.left, y: e.clientY - box.top }
  mode.value = null
  setScale(scale.value * Math.exp(-e.deltaY * (e.deltaMode ? 0.05 : 0.01)), at)
}

// Safari on the Mac reports a trackpad pinch as gesture events instead of ctrl + wheel.
let gestureBase = null
function onGestureStart(e) {
  e.preventDefault()
  if (coarse) return
  gestureBase = scale.value
}
function onGestureChange(e) {
  e.preventDefault()
  if (coarse || gestureBase == null) return
  const box = scroller.value.getBoundingClientRect()
  mode.value = null
  setScale(gestureBase * e.scale, { x: e.clientX - box.left, y: e.clientY - box.top })
}
function onGestureEnd(e) {
  e.preventDefault()
  gestureBase = null
}

function onKey(e) {
  const typing = e.target instanceof HTMLInputElement
  const mod = e.metaKey || e.ctrlKey
  if (mod && e.key.toLowerCase() === 'f') { openSearch(); e.preventDefault(); return }
  if (mod && (e.key === '+' || e.key === '=')) { zoomStep(1); e.preventDefault(); return }
  if (mod && e.key === '-') { zoomStep(-1); e.preventDefault(); return }
  if (mod && e.key === '0') { zoomTo('auto'); e.preventDefault(); return }
  if (e.key === 'Escape') {
    if (zoomMenu.value) zoomMenu.value = false
    else if (searchOpen.value) closeSearch()
    else if (props.onClose) props.onClose()
    return
  }
  if (typing || mod || e.altKey) return
  const el = scroller.value
  const wide = el && el.scrollWidth > el.clientWidth + 1
  if (e.key === 'ArrowRight' && !wide) {
    goToPage(Math.min(pages.value.length, current.value + 1))
    e.preventDefault()
  } else if (e.key === 'ArrowLeft' && !wide) {
    goToPage(Math.max(1, current.value - 1))
    e.preventDefault()
  } else if (e.key === 'Home') {
    goToPage(1)
    e.preventDefault()
  } else if (e.key === 'End') {
    goToPage(pages.value.length)
    e.preventDefault()
  }
}

// ---- Back closes the overlay ----------------------------------------------------------------
// Opened over Files or a share, the phone's back gesture should close the PDF, not leave the page.

let closedByBack = false
function onPopState() {
  // Still on a viewer's entry: the late back() of a viewer closed just before this one opened.
  if (history.state?.tpcloudPdf) return
  closedByBack = true
  props.onClose?.()
}

// ---- Loading --------------------------------------------------------------------------------

function submitPassword() {
  if (!passwordCallback || !password.value) return
  const cb = passwordCallback
  passwordCallback = null
  needsPassword.value = false
  loading.value = true
  cb(password.value)
}

async function load() {
  pdfjs = await import('pdfjs-dist/legacy/build/pdf.mjs')
  pdfjs.GlobalWorkerOptions.workerSrc = workerUrl
  loadingTask = pdfjs.getDocument({
    url: props.url,
    withCredentials: true,
    isEvalSupported: false,
    cMapUrl: assetUrl('cmaps'),
    standardFontDataUrl: assetUrl('standard_fonts'),
    wasmUrl: assetUrl('wasm'),
    iccUrl: assetUrl('iccs'),
  })
  loadingTask.onPassword = (cb, reason) => {
    passwordCallback = cb
    wrongPassword.value = reason === pdfjs.PasswordResponses.INCORRECT_PASSWORD
    password.value = ''
    loading.value = false
    needsPassword.value = true
  }
  doc = await loadingTask.promise
  const sizes = []
  for (let n = 1; n <= doc.numPages; n++) {
    const vp = (await doc.getPage(n)).getViewport({ scale: 1 })
    sizes.push({ w: vp.width, h: vp.height })
  }
  pages.value = sizes
  loading.value = false
  await nextTick()
  phone.value = scroller.value.clientWidth < 700
  avail.value = scroller.value.clientWidth - gutter() * 2
  sidebarOpen.value = !phone.value && doc.numPages > 1 && scroller.value.clientWidth > 1100
  await nextTick()
  scale.value = clampScale(fitScale('auto'))
  await nextTick()
  pageObserver = new IntersectionObserver(onPageIntersect, { root: scroller.value, rootMargin: coarse ? '100% 0px' : '150% 0px' })
  wrappers.forEach((w) => w && pageObserver.observe(w))
  resizeObserver = new ResizeObserver(refit)
  resizeObserver.observe(scroller.value)
  doc.getOutline().then((o) => { outline.value = flatten(o) }).catch(() => {})
}

onMounted(async () => {
  window.addEventListener('keydown', onKey)
  if (props.onClose) {
    history.pushState({ ...history.state, tpcloudPdf: true }, '')
    window.addEventListener('popstate', onPopState)
  }
  const el = root.value
  el.addEventListener('gesturestart', onGestureStart)
  el.addEventListener('gesturechange', onGestureChange)
  el.addEventListener('gestureend', onGestureEnd)
  try {
    await load()
  } catch (err) {
    if (err?.name === 'AbortException') return
    console.error(err)
    error.value = err?.name === 'InvalidPDFException'
      ? t("This file isn't a valid PDF.", 'Este archivo no es un PDF válido.')
      : t("This PDF can't be shown.", 'No se puede mostrar este PDF.')
    loading.value = false
    needsPassword.value = false
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', onKey)
  window.removeEventListener('popstate', onPopState)
  if (props.onClose && !closedByBack && history.state?.tpcloudPdf) history.back()
  clearTimeout(redrawTimer)
  clearTimeout(searchTimer)
  clearTimeout(tapTimer)
  pageObserver?.disconnect()
  thumbObserver?.disconnect()
  resizeObserver?.disconnect()
  for (const num of [...drawn.keys(), ...layers.keys()]) release(num)
  thumbsDrawn.forEach((task) => task?.cancel())
  loadingTask?.destroy()
})

function download() {
  if (props.onDownload) return props.onDownload()
  const a = document.createElement('a')
  a.href = props.url
  a.download = props.name || 'document.pdf'
  document.body.append(a)
  a.click()
  a.remove()
}

const pageStyle = (p, i) => {
  const s = pageScale(i)
  return { width: `${Math.round(p.w * s)}px`, height: `${Math.round(p.h * s)}px`, '--total-scale-factor': s }
}
const matchLabel = computed(() => {
  if (searching.value) return t('Searching…', 'Buscando…')
  if (!query.value.trim()) return ''
  if (!matches.value.length) return t('No results', 'Sin resultados')
  return `${matchIndex.value + 1} / ${matches.value.length}`
})
</script>

<template>
  <Teleport to="body">
    <div
      ref="root"
      class="pv"
      :class="{ 'pv-phone': phone, 'pv-bare': chromeHidden, 'pv-searching': searchOpen }"
      role="dialog"
      :aria-label="title"
    >
      <div class="pv-top">
      <header class="pv-bar">
        <div class="pv-bar-side">
          <button v-if="onClose" type="button" class="pv-btn" :aria-label="t('Close', 'Cerrar')" @click="onClose()">
            <Icon name="xmark" :size="19" :weight="2.2" />
          </button>
          <button
            v-if="pages.length > 1"
            type="button"
            class="pv-btn"
            :class="{ on: sidebarOpen }"
            :aria-label="t('Pages', 'Páginas')"
            @click="sidebarOpen = !sidebarOpen"
          >
            <Icon name="sidebar" :size="19" />
          </button>
        </div>
        <div class="pv-title">
          <span class="pv-name">{{ title }}</span>
          <span v-if="pages.length" class="pv-sub">
            {{ pages.length === 1 ? t('1 page', '1 página') : t(`Page ${current} of ${pages.length}`, `Página ${current} de ${pages.length}`) }}
          </span>
        </div>
        <div class="pv-bar-side pv-bar-end">
          <button v-if="pages.length" type="button" class="pv-btn" :class="{ on: searchOpen }" :aria-label="t('Search', 'Buscar')" @click="searchOpen ? closeSearch() : openSearch()">
            <Icon name="magnifyingglass" :size="19" />
          </button>
          <button type="button" class="pv-btn" :aria-label="t('Download', 'Descargar')" @click="download">
            <Icon name="download" :size="19" />
          </button>
        </div>
      </header>

      <Transition name="pv-drop">
        <div v-if="searchOpen" class="pv-search">
          <div class="pv-field">
            <Icon name="magnifyingglass" :size="16" class="pv-field-icon" />
            <input
              ref="searchInput"
              v-model="query"
              type="search"
              enterkeyhint="search"
              :placeholder="t('Search in document', 'Buscar en el documento')"
              @input="onQuery"
              @keydown.enter.prevent="matches.length ? nextMatch($event.shiftKey ? -1 : 1) : runSearch()"
              @keydown.escape.stop.prevent="closeSearch"
            />
            <span class="pv-count">{{ matchLabel }}</span>
          </div>
          <button type="button" class="pv-btn small" :disabled="matches.length < 2" :aria-label="t('Previous', 'Anterior')" @click="nextMatch(-1)">
            <Icon name="chevron-up" :size="18" :weight="2.2" />
          </button>
          <button type="button" class="pv-btn small" :disabled="matches.length < 2" :aria-label="t('Next', 'Siguiente')" @click="nextMatch(1)">
            <Icon name="chevron-down" :size="18" :weight="2.2" />
          </button>
          <button type="button" class="pv-done" @click="closeSearch">{{ t('Done', 'OK') }}</button>
        </div>
      </Transition>
      </div>

      <div class="pv-body">
        <Transition name="pv-side">
          <aside v-if="sidebarOpen && pages.length" ref="sideEl" class="pv-side">
            <div v-if="hasOutline" class="pv-tabs">
              <button type="button" :class="{ on: sideTab === 'thumbs' }" @click="sideTab = 'thumbs'">
                <Icon name="pages" :size="17" /> {{ t('Pages', 'Páginas') }}
              </button>
              <button type="button" :class="{ on: sideTab === 'outline' }" @click="sideTab = 'outline'">
                <Icon name="list" :size="17" /> {{ t('Contents', 'Índice') }}
              </button>
            </div>
            <div v-if="sideTab === 'thumbs' || !hasOutline" class="pv-thumbs">
              <button
                v-for="(p, i) in pages"
                :key="i"
                type="button"
                class="pv-thumb"
                :class="{ on: current === i + 1 }"
                :data-page="i + 1"
                @click="goToPage(i + 1)"
              >
                <span class="pv-thumb-paper" :style="{ aspectRatio: `${p.w} / ${p.h}` }">
                  <canvas :ref="(el) => (thumbCanvases[i] = el)" />
                </span>
                <span class="pv-thumb-num">{{ i + 1 }}</span>
              </button>
            </div>
            <ul v-else class="pv-outline">
              <li v-for="(item, i) in outline" :key="i">
                <button type="button" :style="{ paddingLeft: `${14 + item.depth * 16}px` }" :class="{ top: item.depth === 0 }" @click="openOutline(item)">
                  {{ item.title }}
                </button>
              </li>
            </ul>
          </aside>
        </Transition>

        <div class="pv-main">
          <div v-if="loading" class="pv-center"><Spinner :size="28" /></div>
          <form v-else-if="needsPassword" class="pv-center pv-card" @submit.prevent="submitPassword">
            <Icon name="lock" :size="34" />
            <h2>{{ t('This PDF is protected', 'Este PDF está protegido') }}</h2>
            <p :class="{ bad: wrongPassword }">
              {{ wrongPassword ? t('Wrong password. Try again.', 'Contraseña incorrecta. Prueba otra vez.') : t('Enter its password to open it.', 'Escribe su contraseña para abrirlo.') }}
            </p>
            <input v-model="password" type="password" autocomplete="off" :placeholder="t('Password', 'Contraseña')" autofocus />
            <button type="submit" class="pv-primary" :disabled="!password">{{ t('Open', 'Abrir') }}</button>
          </form>
          <div v-else-if="error" class="pv-center pv-card">
            <Icon name="warning" :size="34" />
            <p>{{ error }}</p>
            <button type="button" class="pv-primary" @click="download">{{ t('Download', 'Descargar') }}</button>
          </div>

          <div
            v-show="pages.length && !loading && !error"
            ref="scroller"
            class="pv-scroll"
            @scroll.passive="onScroll"
            @pointerdown="zoomMenu = false"
            @wheel="onWheel"
            @touchstart="onTouchStart"
            @touchmove="onTouchMove"
            @touchend="onTouchEnd"
            @touchcancel="onTouchEnd"
          >
            <div ref="pagesEl" class="pv-pages">
              <div
                v-for="(p, i) in pages"
                :key="i"
                :ref="(el) => (wrappers[i] = el)"
                :data-page="i + 1"
                class="pv-page"
                :style="pageStyle(p, i)"
              >
                <canvas :ref="(el) => (canvases[i] = el)" />
                <div :ref="(el) => (textHosts[i] = el)" class="textLayer" />
              </div>
            </div>
          </div>

          <div v-if="pages.length && !loading && !error" class="pv-pill" @click.stop>
            <template v-if="!phone">
              <button type="button" class="pv-pill-btn" :disabled="percent <= STEPS[0]" :aria-label="t('Zoom out', 'Alejar')" @click="zoomStep(-1)">
                <Icon name="minus" :size="17" :weight="2.2" />
              </button>
              <div class="pv-zoom">
                <button type="button" class="pv-pill-text" @click="zoomMenu = !zoomMenu">{{ percent }}%</button>
                <Transition name="pv-pop">
                  <div v-if="zoomMenu" class="pv-menu">
                    <button type="button" :class="{ on: mode === 'auto' }" @click="zoomTo('auto')">{{ t('Automatic', 'Automático') }}</button>
                    <button type="button" :class="{ on: mode === 'width' }" @click="zoomTo('width')">{{ t('Fit width', 'Ajustar al ancho') }}</button>
                    <button type="button" :class="{ on: mode === 'page' }" @click="zoomTo('page')">{{ t('Whole page', 'Página entera') }}</button>
                    <hr />
                    <button v-for="s in [50, 75, 100, 125, 150, 200, 300, 400]" :key="s" type="button" :class="{ on: !mode && percent === s }" @click="zoomTo(s)">{{ s }}%</button>
                  </div>
                </Transition>
              </div>
              <button type="button" class="pv-pill-btn" :disabled="percent >= STEPS[STEPS.length - 1]" :aria-label="t('Zoom in', 'Acercar')" @click="zoomStep(1)">
                <Icon name="plus" :size="17" :weight="2.2" />
              </button>
              <span class="pv-pill-sep" />
            </template>
            <form v-if="editingPage" class="pv-page-form" @submit.prevent="commitPage">
              <input ref="pageInput" v-model="pageDraft" inputmode="numeric" :aria-label="t('Page', 'Página')" @blur="commitPage" />
              <span>/ {{ pages.length }}</span>
            </form>
            <button v-else type="button" class="pv-pill-text" :title="t('Go to page', 'Ir a la página')" @click="phone && pages.length > 1 ? (sidebarOpen = true) : startPageEdit()">
              {{ current }} / {{ pages.length }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.pv {
  --pv-bg: #d9d9de;
  --pv-chrome: rgba(249, 249, 251, 0.82);
  --pv-side-bg: rgba(242, 242, 247, 0.92);
  --pv-hilite: rgba(255, 204, 0, 0.45);
  --pv-hilite-on: rgba(255, 149, 0, 0.75);
  position: fixed;
  inset: 0;
  z-index: 150;
  display: flex;
  flex-direction: column;
  background: var(--pv-bg);
  color: var(--label);
  font-family: var(--font);
  -webkit-tap-highlight-color: transparent;
  animation: pv-in 0.22s var(--ease);
}
@media (prefers-color-scheme: dark) {
  .pv {
    --pv-bg: #161618;
    --pv-chrome: rgba(30, 30, 32, 0.82);
    --pv-side-bg: rgba(28, 28, 30, 0.92);
  }
}
@keyframes pv-in { from { opacity: 0; } }

/* Bar */
.pv-bar {
  position: relative;
  z-index: 3;
  flex-shrink: 0;
  display: grid;
  grid-template-columns: minmax(96px, 1fr) minmax(0, 2.4fr) minmax(96px, 1fr);
  align-items: center;
  gap: 8px;
  padding: calc(8px + env(safe-area-inset-top)) max(12px, env(safe-area-inset-right)) 8px max(12px, env(safe-area-inset-left));
  background: var(--pv-chrome);
  -webkit-backdrop-filter: saturate(180%) blur(20px);
  backdrop-filter: saturate(180%) blur(20px);
  box-shadow: 0 0.5px 0 var(--separator);
  transition: transform 0.3s var(--ease), opacity 0.2s;
}
.pv-bar-side { display: flex; gap: 6px; }
.pv-bar-end { justify-content: flex-end; }
.pv-title { display: flex; flex-direction: column; align-items: center; min-width: 0; text-align: center; line-height: 1.25; }
.pv-name { max-width: 100%; font-size: 15px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.pv-sub { font-size: 12px; color: var(--label-2); font-variant-numeric: tabular-nums; }
.pv-btn {
  width: 38px;
  height: 38px;
  display: grid;
  place-items: center;
  flex-shrink: 0;
  border: 0;
  border-radius: 50%;
  color: var(--label);
  background: var(--fill-3);
  cursor: pointer;
  transition: background 0.15s, transform 0.2s var(--spring), color 0.15s;
}
.pv-btn:hover:not(:disabled) { background: var(--fill-2); }
.pv-btn:active:not(:disabled) { transform: scale(0.9); }
.pv-btn.on { background: var(--accent); color: #fff; }
.pv-btn:disabled { opacity: 0.35; cursor: default; }
.pv-btn.small { width: 34px; height: 34px; background: transparent; }
.pv-btn.small:hover:not(:disabled) { background: var(--fill-3); }

/* Search */
.pv-search {
  position: relative;
  z-index: 2;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px max(12px, env(safe-area-inset-right)) 8px max(12px, env(safe-area-inset-left));
  background: var(--pv-chrome);
  -webkit-backdrop-filter: saturate(180%) blur(20px);
  backdrop-filter: saturate(180%) blur(20px);
  box-shadow: 0 0.5px 0 var(--separator);
}
.pv-field {
  flex: 1;
  max-width: 520px;
  margin: 0 auto 0 0;
  display: flex;
  align-items: center;
  gap: 6px;
  height: 36px;
  padding: 0 10px;
  border-radius: 10px;
  background: var(--fill-3);
}
.pv:not(.pv-phone) .pv-search { justify-content: center; }
.pv:not(.pv-phone) .pv-field { margin: 0; }
.pv-field-icon { color: var(--label-2); flex-shrink: 0; }
.pv-field input {
  flex: 1;
  min-width: 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: inherit;
  font: 16px var(--font);
  -webkit-appearance: none;
  appearance: none;
}
.pv-field input::-webkit-search-cancel-button { display: none; }
.pv-count { flex-shrink: 0; font-size: 13px; color: var(--label-2); font-variant-numeric: tabular-nums; }
.pv-done { border: 0; background: none; color: var(--accent); font: 600 15px var(--font); padding: 0 6px 0 10px; cursor: pointer; }
.pv-drop-enter-active, .pv-drop-leave-active { transition: transform 0.25s var(--ease), opacity 0.2s; }
.pv-drop-enter-from, .pv-drop-leave-to { transform: translateY(-100%); opacity: 0; }

/* Body */
.pv-body { position: relative; flex: 1; min-height: 0; display: flex; }
.pv-main { position: relative; flex: 1; min-width: 0; }
.pv-scroll {
  position: absolute;
  inset: 0;
  overflow: auto;
  overscroll-behavior: contain;
  touch-action: pan-x pan-y;
  -webkit-overflow-scrolling: touch;
}
.pv-pages {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  width: max-content;
  min-width: 100%;
  padding: 12px 28px calc(84px + env(safe-area-inset-bottom));
  box-sizing: border-box;
  transform-origin: 0 0;
  will-change: auto;
}
.pv-phone .pv-pages { padding-left: 6px; padding-right: 6px; gap: 8px; }
.pv-page {
  position: relative;
  flex-shrink: 0;
  background: #fff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12), 0 6px 24px rgba(0, 0, 0, 0.1);
  --scale-round-x: 1px;
  --scale-round-y: 1px;
}
.pv-page canvas { position: absolute; inset: 0; display: block; width: 100%; height: 100%; }
.pv-center {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 24px;
  text-align: center;
  color: var(--label-2);
}
.pv-card h2 { margin: 4px 0 0; font-size: 19px; color: var(--label); }
.pv-card p { margin: 0; max-width: 320px; }
.pv-card p.bad { color: var(--red); }
.pv-card input {
  width: min(280px, 100%);
  height: 44px;
  margin-top: 6px;
  padding: 0 14px;
  border: 0;
  border-radius: 12px;
  background: var(--cell);
  color: var(--label);
  font: 17px var(--font);
  outline: 0;
}
.pv-primary {
  min-width: 140px;
  height: 44px;
  padding: 0 22px;
  border: 0;
  border-radius: 999px;
  background: var(--accent);
  color: #fff;
  font: 600 16px var(--font);
  cursor: pointer;
}
.pv-primary:disabled { opacity: 0.4; cursor: default; }

/* Sidebar */
.pv-side {
  position: relative;
  z-index: 2;
  flex-shrink: 0;
  width: 188px;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  overscroll-behavior: contain;
  background: var(--pv-side-bg);
  -webkit-backdrop-filter: saturate(180%) blur(20px);
  backdrop-filter: saturate(180%) blur(20px);
  box-shadow: 0.5px 0 0 var(--separator);
}
.pv-side-enter-active, .pv-side-leave-active { transition: margin 0.28s var(--ease), opacity 0.2s, transform 0.28s var(--ease); }
.pv-side-enter-from, .pv-side-leave-to { margin-left: -188px; opacity: 0; }
.pv-tabs {
  position: sticky;
  top: 0;
  z-index: 1;
  display: flex;
  gap: 4px;
  margin: 10px 10px 2px;
  padding: 2px;
  border-radius: 9px;
  background: var(--fill-3);
}
.pv-tabs button {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  height: 28px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--label);
  font: 500 12px var(--font);
  cursor: pointer;
}
.pv-tabs button.on { background: var(--bg-elevated); box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12); }
.pv-thumbs { display: flex; flex-direction: column; align-items: center; gap: 14px; padding: 14px 0 calc(24px + env(safe-area-inset-bottom)); }
.pv-thumb { display: flex; flex-direction: column; align-items: center; gap: 6px; border: 0; padding: 0; background: none; color: var(--label-2); cursor: pointer; }
.pv-thumb-paper {
  position: relative;
  display: block;
  width: 116px;
  background: #fff;
  border-radius: 3px;
  overflow: hidden;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.18);
  outline: 3px solid transparent;
  outline-offset: 3px;
  transition: outline-color 0.15s;
}
.pv-thumb-paper canvas { position: absolute; inset: 0; width: 100%; height: 100%; }
.pv-thumb-num { font-size: 12px; font-variant-numeric: tabular-nums; padding: 1px 8px; border-radius: 999px; }
.pv-thumb.on .pv-thumb-paper { outline-color: var(--accent); }
.pv-thumb.on .pv-thumb-num { background: var(--accent); color: #fff; }
.pv-outline { list-style: none; margin: 0; padding: 8px 0 24px; }
.pv-outline button {
  width: 100%;
  border: 0;
  background: none;
  text-align: left;
  padding: 7px 14px;
  color: var(--label);
  font: 13px/1.3 var(--font);
  cursor: pointer;
}
.pv-outline button.top { font-weight: 600; }
.pv-outline button:hover { background: var(--fill-4); }

/* Pill */
.pv-pill {
  position: absolute;
  z-index: 2;
  left: 50%;
  bottom: calc(16px + env(safe-area-inset-bottom));
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 4px;
  border-radius: 999px;
  background: var(--pv-chrome);
  -webkit-backdrop-filter: saturate(180%) blur(20px);
  backdrop-filter: saturate(180%) blur(20px);
  box-shadow: 0 0 0 0.5px var(--separator), 0 6px 24px rgba(0, 0, 0, 0.16);
  font-variant-numeric: tabular-nums;
  transition: transform 0.3s var(--ease), opacity 0.2s;
}
.pv-pill-btn, .pv-pill-text {
  height: 34px;
  border: 0;
  border-radius: 999px;
  background: transparent;
  color: var(--label);
  font: 500 14px var(--font);
  cursor: pointer;
}
.pv-pill-btn { width: 34px; display: grid; place-items: center; }
.pv-pill-text { padding: 0 12px; min-width: 64px; }
.pv-pill-btn:hover:not(:disabled), .pv-pill-text:hover { background: var(--fill-3); }
.pv-pill-btn:disabled { opacity: 0.35; cursor: default; }
.pv-pill-sep { width: 0.5px; height: 20px; margin: 0 4px; background: var(--separator); }
.pv-page-form { display: flex; align-items: center; gap: 6px; padding: 0 10px 0 6px; font: 500 14px var(--font); }
.pv-page-form input {
  width: 44px;
  height: 28px;
  border: 0;
  border-radius: 8px;
  background: var(--fill-3);
  color: inherit;
  font: inherit;
  text-align: center;
  outline: 2px solid var(--accent);
}
.pv-zoom { position: relative; }
.pv-menu {
  position: absolute;
  bottom: calc(100% + 10px);
  left: 50%;
  transform: translateX(-50%);
  min-width: 180px;
  padding: 6px;
  border-radius: 14px;
  background: var(--bg-elevated);
  box-shadow: 0 0 0 0.5px var(--separator), 0 12px 40px rgba(0, 0, 0, 0.22);
  display: flex;
  flex-direction: column;
}
.pv-menu button {
  border: 0;
  background: none;
  text-align: left;
  padding: 7px 10px 7px 28px;
  border-radius: 8px;
  color: var(--label);
  font: 14px var(--font);
  cursor: pointer;
  position: relative;
}
.pv-menu button:hover { background: var(--accent); color: #fff; }
.pv-menu button.on::before { content: '✓'; position: absolute; left: 10px; font-weight: 600; }
.pv-menu hr { border: 0; height: 0.5px; background: var(--separator); margin: 5px 8px; }
.pv-pop-enter-active, .pv-pop-leave-active { transition: opacity 0.15s, transform 0.2s var(--ease); }
.pv-pop-enter-from, .pv-pop-leave-to { opacity: 0; transform: translateX(-50%) translateY(6px) scale(0.97); }

/* Phone: the sidebar is a sheet of pages over the document, and a tap hides the bars. */
.pv-phone .pv-bar { grid-template-columns: auto minmax(0, 1fr) auto; }
.pv-phone .pv-side {
  position: absolute;
  inset: 0;
  width: auto;
  z-index: 4;
  background: var(--bg);
}
.pv-phone .pv-side-enter-from, .pv-phone .pv-side-leave-to { margin-left: 0; transform: translateY(24px); }
.pv-phone .pv-thumbs {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(124px, 1fr));
  justify-items: center;
  gap: 18px 8px;
  padding: 16px 12px calc(32px + env(safe-area-inset-bottom));
}
.pv-phone .pv-tabs { margin: 12px 16px 4px; }
.pv-top { position: relative; z-index: 3; flex-shrink: 0; transition: transform 0.3s var(--ease), opacity 0.2s; }
.pv-phone .pv-top { position: absolute; top: 0; left: 0; right: 0; z-index: 5; }
.pv-phone .pv-pages { padding-top: calc(66px + env(safe-area-inset-top)); }
.pv-phone.pv-searching .pv-pages { padding-top: calc(118px + env(safe-area-inset-top)); }
.pv-phone .pv-side { padding-top: calc(54px + env(safe-area-inset-top)); }
.pv-phone.pv-searching .pv-side { padding-top: calc(106px + env(safe-area-inset-top)); }
.pv-bare .pv-top { transform: translateY(-100%); opacity: 0; pointer-events: none; }
.pv-bare .pv-pill { transform: translateX(-50%) translateY(calc(100% + 40px)); opacity: 0; }

/* pdf.js text layer (from its pdf_viewer.css, trimmed to what this viewer uses) */
.pv-page :deep(.textLayer) {
  color-scheme: only light;
  position: absolute;
  text-align: initial;
  inset: 0;
  overflow: clip;
  opacity: 1;
  line-height: 1;
  letter-spacing: normal;
  word-spacing: normal;
  -webkit-text-size-adjust: none;
  text-size-adjust: none;
  forced-color-adjust: none;
  transform-origin: 0 0;
  caret-color: CanvasText;
  z-index: 0;
  --min-font-size: 1;
  --text-scale-factor: calc(var(--total-scale-factor) * var(--min-font-size));
  --min-font-size-inv: calc(1 / var(--min-font-size));
}
.pv-page :deep(.textLayer :is(span, br)) {
  color: transparent;
  position: absolute;
  white-space: pre;
  cursor: text;
  transform-origin: 0% 0%;
  -webkit-user-select: text;
  user-select: text;
}
.pv-page :deep(.textLayer > :not(.markedContent)),
.pv-page :deep(.textLayer .markedContent span:not(.markedContent)) {
  z-index: 1;
  --font-height: 0;
  font-size: calc(var(--text-scale-factor) * var(--font-height));
  --scale-x: 1;
  --rotate: 0deg;
  transform: rotate(var(--rotate)) scaleX(var(--scale-x)) scale(var(--min-font-size-inv));
}
.pv-page :deep(.textLayer .markedContent) { display: contents; }
.pv-page :deep(.textLayer span[role='img']) { user-select: none; cursor: default; }
.pv-page :deep(.textLayer .highlight) {
  margin: -1px;
  padding: 1px;
  border-radius: 3px;
  background-color: var(--pv-hilite);
}
.pv-page :deep(.textLayer .highlight.appended) { position: initial; }
.pv-page :deep(.textLayer .highlight.selected) { background-color: var(--pv-hilite-on); }
.pv-page :deep(.textLayer ::selection) { background: rgba(0, 122, 255, 0.3); color: transparent; }
.pv-page :deep(.textLayer br::selection) { background: transparent; }
.pv-page :deep(.textLayer .endOfContent) {
  display: block;
  position: absolute;
  inset: 100% 0 0;
  z-index: 0;
  cursor: default;
  user-select: none;
}
.pv-page :deep(.textLayer.selecting .endOfContent) { top: 0; }
</style>
