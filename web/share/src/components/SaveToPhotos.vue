<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { X } from 'lucide-vue-next'
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
  if (photos) parts.push(photos === 1 ? '1 photo' : `${photos} photos`)
  if (videos) parts.push(videos === 1 ? '1 video' : `${videos} videos`)
  return parts.join(' and ')
})

async function fetchFile(item, signal) {
  const res = await fetch(props.downloadUrl(item), { signal })
  if (!res.ok || !res.body) throw new Error(`Couldn't download ${item.name}`)
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
    errorText.value = e.message || 'Download failed'
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
    if (e.name !== 'AbortError') errorText.value = e.message || "Couldn't open the share sheet"
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
    <div class="save-backdrop" @click.self="close">
      <div class="save-sheet" role="dialog" aria-label="Save to Photos">
        <div class="save-header">
          <span class="save-title">Save to Photos</span>
          <button class="save-close" title="Close" @click="close"><X class="save-close-icon" /></button>
        </div>

        <template v-if="phase === 'done'">
          <p class="save-text">
            {{ savedCount === 1 ? 'Saved 1 item' : `Saved ${savedCount} items` }}. They're in your Photos now.
          </p>
          <button class="save-primary" @click="close">Done</button>
        </template>

        <template v-else>
          <p class="save-text">
            <template v-if="batches.length > 1">Part {{ batchIndex + 1 }} of {{ batches.length }} · </template>
            {{ batchLabel }}
            <span class="save-dim"> · {{ formatSize(batchBytes) }}</span>
          </p>
          <p v-if="batchIndex === 0 && batches.length > 1" class="save-hint">
            {{ props.items.length }} items, {{ formatSize(totalBytes) }} in all. They go in parts so your phone
            doesn't run out of memory; tap Save after each one.
          </p>

          <div class="save-bar"><div class="save-bar-fill" :style="{ width: `${progress * 100}%` }" /></div>

          <p v-if="errorText" class="save-error">{{ errorText }}</p>

          <div class="save-actions">
            <button v-if="phase === 'error'" class="save-primary" @click="downloadBatch">Try Again</button>
            <button v-else class="save-primary" :disabled="phase !== 'ready'" @click="save">
              <template v-if="phase === 'downloading'">Downloading… {{ Math.round(progress * 100) }}%</template>
              <template v-else>Save {{ batch.length === 1 ? '1 Item' : `${batch.length} Items` }}</template>
            </button>
            <button v-if="batches.length > 1 && phase !== 'downloading'" class="save-secondary" @click="next">Skip</button>
          </div>
        </template>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.save-backdrop {
  position: fixed;
  inset: 0;
  z-index: 10001;
  background: rgba(0, 0, 0, 0.55);
  display: flex;
  align-items: flex-end;
  justify-content: center;
}
.save-sheet {
  width: min(440px, 100%);
  margin: 0 0 max(16px, env(safe-area-inset-bottom));
  padding: 16px 18px 18px;
  background: rgba(28, 28, 30, 0.96);
  backdrop-filter: blur(40px) saturate(180%);
  -webkit-backdrop-filter: blur(40px) saturate(180%);
  border: 0.5px solid rgba(255, 255, 255, 0.14);
  border-radius: 18px;
  box-shadow: 0 12px 48px rgba(0, 0, 0, 0.6);
}
@media (max-width: 767px) {
  .save-sheet { margin-left: 10px; margin-right: 10px; width: auto; flex: 1; }
}
.save-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; }
.save-title { font-size: 1.0625rem; font-weight: 600; color: #f5f5f7; }
.save-close {
  width: 30px; height: 30px; display: flex; align-items: center; justify-content: center;
  border-radius: 50%; border: none; background: rgba(255, 255, 255, 0.08); color: rgba(255, 255, 255, 0.75);
}
.save-close-icon { width: 14px; height: 14px; stroke-width: 2.5; }
.save-text { font-size: 0.9375rem; color: #e4e4e7; margin: 0 0 6px; }
.save-dim { color: #8e8e93; }
.save-hint { font-size: 0.8125rem; color: #8e8e93; margin: 0 0 10px; line-height: 1.35; }
.save-error { font-size: 0.8125rem; color: #ff453a; margin: 8px 0 0; }
.save-bar { height: 4px; border-radius: 2px; background: rgba(255, 255, 255, 0.1); overflow: hidden; margin: 10px 0 4px; }
.save-bar-fill { height: 100%; background: #0a84ff; transition: width 0.2s ease; }
.save-actions { display: flex; gap: 8px; margin-top: 14px; }
.save-primary {
  flex: 1; height: 44px; border: none; border-radius: 12px; background: #0a84ff; color: #fff;
  font-size: 0.9375rem; font-weight: 600; font-variant-numeric: tabular-nums;
}
.save-primary:disabled { background: rgba(10, 132, 255, 0.35); color: rgba(255, 255, 255, 0.7); }
.save-secondary {
  height: 44px; padding: 0 18px; border: none; border-radius: 12px;
  background: rgba(255, 255, 255, 0.1); color: #e4e4e7; font-size: 0.9375rem;
}
</style>
