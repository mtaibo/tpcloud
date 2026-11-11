<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { Check, ImageDown, X } from 'lucide-vue-next'
import { mimeType, isVideo } from '../media.js'
import { formatSize } from '../utils/formatSize.js'

/*
 * Saves originals into the phone's photo gallery through the system share sheet ("Save 12
 * Images"), which is the only door a web page has into it. Two things shape how:
 *
 * - The files have to be downloaded before the sheet can open, and a browser only opens it
 *   straight after a tap. A download takes longer than that tap stays valid, so each batch is
 *   downloaded first and then waits for its own "Save" tap.
 * - Every file in a batch sits in memory until it's handed over, so batches are capped in size;
 *   a folder of phone videos is several gigabytes, more than a mobile browser can hold at once.
 */
const props = defineProps({
  items: Array,
  downloadUrl: Function,
})
const emit = defineEmits(['close'])

const MAX_BATCH_BYTES = 300 * 1024 * 1024
const MAX_BATCH_FILES = 30
const PARALLEL = 3

const batches = computed(() => {
  const result = []
  let current = []
  let bytes = 0
  for (const item of props.items) {
    const size = item.size || 0
    if (current.length && (bytes + size > MAX_BATCH_BYTES || current.length >= MAX_BATCH_FILES)) {
      result.push(current)
      current = []
      bytes = 0
    }
    current.push(item)
    bytes += size
  }
  if (current.length) result.push(current)
  return result
})

const batchIndex = ref(0)
// 'downloading' → 'ready' (waiting for the tap) → 'sharing' → next batch, or 'done' / 'error'.
const phase = ref('downloading')
const loadedBytes = ref(0)
const errorText = ref('')
const savedCount = ref(0)
let files = []
let abort = null

const batch = computed(() => batches.value[batchIndex.value] || [])
const batchBytes = computed(() => batch.value.reduce((sum, item) => sum + (item.size || 0), 0))
const progress = computed(() => (batchBytes.value ? Math.min(1, loadedBytes.value / batchBytes.value) : 0))
const totalBytes = computed(() => props.items.reduce((sum, item) => sum + (item.size || 0), 0))

const batchLabel = computed(() => {
  const videos = batch.value.filter(isVideo).length
  const photos = batch.value.length - videos
  const parts = []
  if (photos) parts.push(photos === 1 ? '1 foto' : `${photos} fotos`)
  if (videos) parts.push(videos === 1 ? '1 vídeo' : `${videos} vídeos`)
  return parts.join(' y ')
})

async function fetchFile(item, signal) {
  const res = await fetch(props.downloadUrl(item), { signal })
  if (!res.ok || !res.body) throw new Error(`No se pudo descargar «${item.name}»`)
  const reader = res.body.getReader()
  const chunks = []
  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    chunks.push(value)
    loadedBytes.value += value.length
  }
  return new File(chunks, item.name, { type: mimeType(item.name) })
}

async function downloadBatch() {
  phase.value = 'downloading'
  loadedBytes.value = 0
  errorText.value = ''
  files = []
  abort = new AbortController()
  const queue = [...batch.value]
  const result = new Array(queue.length)
  try {
    await Promise.all(
      Array.from({ length: Math.min(PARALLEL, queue.length) }, async () => {
        while (queue.length) {
          const index = batch.value.length - queue.length
          const item = queue.shift()
          result[index] = await fetchFile(item, abort.signal)
        }
      })
    )
    files = result
    phase.value = 'ready'
  } catch (e) {
    if (e.name === 'AbortError') return
    errorText.value = e.message || 'La descarga ha fallado'
    phase.value = 'error'
  }
}

async function save() {
  phase.value = 'sharing'
  try {
    await navigator.share({ files })
    savedCount.value += files.length
    next()
  } catch (e) {
    // Closing the sheet without saving: keep the batch ready to try again or skip.
    phase.value = 'ready'
    if (e.name !== 'AbortError') errorText.value = e.message || 'No se pudo abrir el menú de compartir'
  }
}

function next() {
  files = []
  if (batchIndex.value + 1 >= batches.value.length) {
    phase.value = 'done'
    return
  }
  batchIndex.value++
  downloadBatch()
}

function close() {
  abort?.abort()
  files = []
  emit('close')
}

onMounted(downloadBatch)
onUnmounted(() => abort?.abort())
</script>

<template>
  <Teleport to="body">
    <Transition name="sheet" appear>
      <div class="overlay" @click.self="close">
        <div class="sheet" role="dialog" aria-label="Guardar en Fotos">
          <div class="sheet-bar">
            <button type="button" class="icon-btn" aria-label="Cerrar" @click="close"><X :size="20" :stroke-width="2.2" /></button>
            <div class="navbar-title">Guardar en Fotos</div>
            <span />
          </div>
          <div class="sheet-body">
            <div class="hero">
              <span class="hero-icon" :class="{ done: phase === 'done' }">
                <Check v-if="phase === 'done'" :size="38" :stroke-width="2.6" />
                <ImageDown v-else :size="34" />
              </span>
              <template v-if="phase === 'done'">
                <p class="title-3">{{ savedCount === 1 ? '1 elemento guardado' : `${savedCount} elementos guardados` }}</p>
                <p class="subheadline secondary">Ya están en tu app Fotos.</p>
              </template>
              <template v-else>
                <p class="title-3">{{ batchLabel }}</p>
                <p class="subheadline secondary">
                  <template v-if="batches.length > 1">Parte {{ batchIndex + 1 }} de {{ batches.length }} · </template>{{ formatSize(batchBytes) }}
                </p>
              </template>
            </div>

            <template v-if="phase !== 'done'">
              <p v-if="batchIndex === 0 && batches.length > 1" class="footnote secondary hint">
                Son {{ props.items.length }} elementos ({{ formatSize(totalBytes) }}). Van por partes para que el móvil no se quede sin memoria: pulsa «Guardar» al terminar cada una.
              </p>
              <div class="bar"><i :style="{ width: `${progress * 100}%` }" /></div>
              <p v-if="errorText" class="footnote danger err">{{ errorText }}</p>
              <div class="actions">
                <button v-if="phase === 'error'" type="button" class="btn btn-prominent block" @click="downloadBatch">Reintentar</button>
                <button v-else type="button" class="btn btn-prominent block" :disabled="phase !== 'ready'" @click="save">
                  <template v-if="phase === 'downloading'">Preparando… {{ Math.round(progress * 100) }} %</template>
                  <template v-else>Guardar {{ batch.length === 1 ? '1 elemento' : `${batch.length} elementos` }}</template>
                </button>
                <button v-if="batches.length > 1 && phase !== 'downloading'" type="button" class="btn btn-plain block" @click="next">Saltar esta parte</button>
              </div>
            </template>
            <button v-else type="button" class="btn btn-prominent block" @click="close">Hecho</button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.hero { display: flex; flex-direction: column; align-items: center; gap: 6px; text-align: center; padding: 4px 0 22px; }
.hero-icon { width: 76px; height: 76px; display: grid; place-items: center; border-radius: 22px; color: #fff; background: linear-gradient(180deg, #ffb340, #ff7a00); margin-bottom: 8px; }
.hero-icon.done { background: var(--green); border-radius: 50%; }
.hint { text-align: center; margin-bottom: 16px; }
.bar { height: 6px; border-radius: 3px; background: var(--fill-3); overflow: hidden; margin-bottom: 18px; }
.bar i { display: block; height: 100%; background: var(--accent); border-radius: 3px; transition: width 0.3s; }
.err { text-align: center; margin: -8px 0 14px; }
.actions { display: grid; gap: 6px; }
</style>
