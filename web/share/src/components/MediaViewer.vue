<script setup>
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { X, ChevronLeft, ChevronRight, Download, ImageDown } from 'lucide-vue-next'
import { isVideo, canSaveToPhotos } from '../media.js'

/*
 * Full-screen viewer for a folder's photos and videos, stepping through them with the arrows,
 * the keyboard or a swipe. It shows the light versions (a 2048 px JPEG, a 720p MP4) so they open
 * at once and play anywhere; Download and Save to Photos get the original.
 */
const props = defineProps({
  items: Array,
  startIndex: { type: Number, default: 0 },
  urls: Object,
})
const emit = defineEmits(['close', 'download', 'save'])

const index = ref(props.startIndex)
const imageLoaded = ref(false)

const current = computed(() => props.items[index.value])
const currentIsVideo = computed(() => current.value && isVideo(current.value))
const hasPrev = computed(() => index.value > 0)
const hasNext = computed(() => index.value < props.items.length - 1)

function prev() { if (hasPrev.value) index.value-- }
function next() { if (hasNext.value) index.value++ }

// The neighbours' photos are fetched in the background, so stepping through feels instant.
function preloadAround(i) {
  for (const j of [i + 1, i - 1, i + 2]) {
    const item = props.items[j]
    if (!item) continue
    const img = new Image()
    img.src = isVideo(item) ? props.urls.poster(item) : props.urls.preview(item)
  }
}

watch(index, (i) => {
  imageLoaded.value = false
  preloadAround(i)
}, { immediate: true })

function close() {
  emit('close')
}

function onKeydown(e) {
  if (e.key === 'Escape') close()
  else if (e.key === 'ArrowLeft') prev()
  else if (e.key === 'ArrowRight') next()
}

// Horizontal swipes step through; ones that start on the video are left to its controls.
let touchStartX = null
let touchStartY = null
function onTouchStart(e) {
  if (e.target.tagName === 'VIDEO' || e.touches.length !== 1) { touchStartX = null; return }
  touchStartX = e.touches[0].clientX
  touchStartY = e.touches[0].clientY
}
function onTouchEnd(e) {
  if (touchStartX === null) return
  const dx = e.changedTouches[0].clientX - touchStartX
  const dy = e.changedTouches[0].clientY - touchStartY
  touchStartX = null
  if (Math.abs(dx) < 50 || Math.abs(dx) < Math.abs(dy)) return
  dx < 0 ? next() : prev()
}

onMounted(() => document.addEventListener('keydown', onKeydown))
onUnmounted(() => document.removeEventListener('keydown', onKeydown))
</script>

<template>
  <Teleport to="body">
    <div class="look" @click.self="close" @touchstart.passive="onTouchStart" @touchend="onTouchEnd">
      <header class="look-bar">
        <button type="button" class="look-btn" aria-label="Close" @click="close"><X :size="20" :stroke-width="2.2" /></button>
        <div class="look-title">
          <span class="look-name">{{ current?.name }}</span>
          <span v-if="items.length > 1" class="look-count">{{ index + 1 }} of {{ items.length }}</span>
        </div>
        <div class="look-actions">
          <button v-if="canSaveToPhotos" type="button" class="look-btn" aria-label="Save to Photos" @click="emit('save', current)"><ImageDown :size="19" /></button>
          <button type="button" class="look-btn" aria-label="Download Original" @click="emit('download', current)"><Download :size="19" /></button>
        </div>
      </header>

      <div class="stage" @click.self="close">
        <video
          v-if="currentIsVideo"
          :key="current.name"
          class="media"
          :src="urls.preview(current)"
          :poster="urls.poster(current)"
          controls
          autoplay
          playsinline
          preload="auto"
        />
        <template v-else-if="current">
          <span v-if="!imageLoaded" class="spinner-wrap"><span class="ring" /></span>
          <img
            :key="current.name"
            class="media"
            :class="{ loaded: imageLoaded }"
            :src="urls.preview(current)"
            :alt="current.name"
            @load="imageLoaded = true"
          />
        </template>
      </div>

      <button v-if="hasPrev" type="button" class="arrow left" aria-label="Previous" @click="prev"><ChevronLeft :size="26" :stroke-width="2.2" /></button>
      <button v-if="hasNext" type="button" class="arrow right" aria-label="Next" @click="next"><ChevronRight :size="26" :stroke-width="2.2" /></button>
    </div>
  </Teleport>
</template>

<style scoped>
.look {
  position: fixed;
  inset: 0;
  z-index: 150;
  display: flex;
  flex-direction: column;
  color: #fff;
  background: rgba(0, 0, 0, 0.92);
  backdrop-filter: blur(30px);
  -webkit-backdrop-filter: blur(30px);
  animation: look-in 0.25s var(--ease);
}
@keyframes look-in { from { opacity: 0; } }
.look-bar {
  flex-shrink: 0;
  display: grid;
  grid-template-columns: 96px 1fr 96px;
  align-items: center;
  gap: 8px;
  padding: calc(12px + env(safe-area-inset-top)) 14px 10px;
}
.look-actions { display: flex; gap: 8px; justify-content: flex-end; }
.look-title { display: flex; flex-direction: column; align-items: center; min-width: 0; text-align: center; }
.look-name { max-width: 100%; font-size: 15px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.look-count { font-size: 12px; color: rgba(255, 255, 255, 0.6); }
.look-btn, .arrow {
  width: 44px;
  height: 44px;
  display: grid;
  place-items: center;
  border-radius: 50%;
  color: #fff;
  background: rgba(255, 255, 255, 0.14);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  box-shadow: inset 0 0.5px 0 rgba(255, 255, 255, 0.25);
  transition: background 0.15s, transform 0.2s var(--spring);
}
.look-btn:hover, .arrow:hover { background: rgba(255, 255, 255, 0.24); }
.look-btn:active, .arrow:active { transform: scale(0.92); }
/* Fixed tracks so the media's max-width/max-height resolve against the stage and a tall video
   fits on screen instead of growing the row past it. */
.stage { position: relative; flex: 1; min-height: 0; display: grid; grid-template: minmax(0, 1fr) / minmax(0, 1fr); place-items: center; padding: 8px 72px calc(28px + env(safe-area-inset-bottom)); }
.media { max-width: 100%; max-height: 100%; object-fit: contain; border-radius: 10px; box-shadow: 0 24px 80px rgba(0, 0, 0, 0.5); user-select: none; -webkit-user-drag: none; }
img.media { opacity: 0; transition: opacity 0.25s; }
img.media.loaded { opacity: 1; }
.spinner-wrap { position: absolute; inset: 0; display: grid; place-items: center; pointer-events: none; }
.ring { width: 30px; height: 30px; border-radius: 50%; border: 2.5px solid rgba(255, 255, 255, 0.2); border-top-color: #fff; animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.arrow { position: absolute; top: 50%; margin-top: -22px; }
.arrow.left { left: 16px; }
.arrow.right { right: 16px; }
@media (max-width: 767px) {
  .stage { padding: 8px 0 calc(16px + env(safe-area-inset-bottom)); }
  .media { border-radius: 0; }
  .arrow { display: none; }
}
</style>
