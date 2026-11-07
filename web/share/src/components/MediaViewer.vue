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

function onKeydown(e) {
  if (e.key === 'Escape') emit('close')
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
    <div class="viewer-backdrop" @click.self="emit('close')" @touchstart.passive="onTouchStart" @touchend="onTouchEnd">
      <div class="viewer-top">
        <span class="viewer-counter">{{ index + 1 }} / {{ items.length }}</span>
        <div class="viewer-actions">
          <button v-if="canSaveToPhotos" class="viewer-btn" title="Save to Photos" @click="emit('save', current)">
            <ImageDown class="viewer-icon" />
          </button>
          <button class="viewer-btn" title="Download original" @click="emit('download', current)">
            <Download class="viewer-icon" />
          </button>
          <button class="viewer-btn" title="Close" @click="emit('close')">
            <X class="viewer-icon" />
          </button>
        </div>
      </div>

      <video
        v-if="currentIsVideo"
        :key="current.name"
        class="viewer-media"
        :src="urls.preview(current)"
        :poster="urls.poster(current)"
        controls
        autoplay
        playsinline
        preload="auto"
      />
      <template v-else-if="current">
        <div v-if="!imageLoaded" class="viewer-spinner" />
        <img
          :key="current.name"
          class="viewer-media"
          :class="{ loaded: imageLoaded }"
          :src="urls.preview(current)"
          :alt="current.name"
          @load="imageLoaded = true"
          @click.self="emit('close')"
        />
      </template>

      <button v-if="hasPrev" class="viewer-nav viewer-nav--prev" title="Previous" @click="prev">
        <ChevronLeft class="viewer-icon" />
      </button>
      <button v-if="hasNext" class="viewer-nav viewer-nav--next" title="Next" @click="next">
        <ChevronRight class="viewer-icon" />
      </button>

      <span class="viewer-name">{{ current?.name }}</span>
    </div>
  </Teleport>
</template>

<style scoped>
.viewer-backdrop {
  position: fixed;
  inset: 0;
  z-index: 10000;
  background: rgba(0, 0, 0, 0.94);
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.viewer-media {
  max-width: 92vw;
  max-height: calc(100dvh - 140px);
  object-fit: contain;
  border-radius: 6px;
  user-select: none;
  -webkit-user-drag: none;
}
img.viewer-media { opacity: 0; transition: opacity 0.15s ease; }
img.viewer-media.loaded { opacity: 1; }

.viewer-spinner {
  position: absolute;
  width: 28px;
  height: 28px;
  border: 2.5px solid rgba(255, 255, 255, 0.15);
  border-top-color: rgba(255, 255, 255, 0.7);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }

.viewer-top {
  position: absolute;
  top: max(16px, env(safe-area-inset-top));
  left: 20px;
  right: 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.viewer-counter {
  font-size: 0.8125rem;
  font-variant-numeric: tabular-nums;
  color: rgba(255, 255, 255, 0.6);
}
.viewer-actions { display: flex; gap: 8px; }

.viewer-btn,
.viewer-nav {
  width: 38px;
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.1);
  border: 0.5px solid rgba(255, 255, 255, 0.15);
  border-radius: 50%;
  color: rgba(255, 255, 255, 0.85);
  transition: background 0.12s, color 0.12s;
}
.viewer-btn:hover,
.viewer-nav:hover { background: rgba(255, 255, 255, 0.18); color: #fff; }

.viewer-nav {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
}
.viewer-nav--prev { left: 16px; }
.viewer-nav--next { right: 16px; }

.viewer-icon { width: 17px; height: 17px; stroke-width: 2.25; }

.viewer-name {
  position: absolute;
  bottom: max(20px, env(safe-area-inset-bottom));
  left: 50%;
  transform: translateX(-50%);
  font-size: 0.8125rem;
  color: rgba(255, 255, 255, 0.5);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 80vw;
  pointer-events: none;
}

@media (max-width: 767px) {
  .viewer-media { max-width: 100vw; border-radius: 0; }
  .viewer-nav { display: none; }
}
</style>
