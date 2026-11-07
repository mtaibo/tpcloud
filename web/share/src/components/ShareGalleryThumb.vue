<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { Play } from 'lucide-vue-next'
import { formatDuration } from '../media.js'

// Videos get a JPEG cut from their poster frame too, so a tile never loads the video itself.
const props = defineProps({
  src: String,
  alt: String,
  isVideo: { type: Boolean, default: false },
  duration: { type: Number, default: null },
})

const wrapRef = ref(null)
const activeSrc = ref('')
const loaded = ref(false)
let retryTimer = null
let retries = 0
let observer = null

onMounted(() => {
  observer = new IntersectionObserver(
    ([entry]) => {
      if (entry.isIntersecting && !activeSrc.value) {
        activeSrc.value = props.src
        observer.disconnect()
        observer = null
      }
    },
    { rootMargin: '600px 0px' }
  )
  if (wrapRef.value) observer.observe(wrapRef.value)
})

onUnmounted(() => {
  observer?.disconnect()
  observer = null
  clearTimeout(retryTimer)
  activeSrc.value = ''
})

// The first visitor of a fresh folder may arrive while its thumbnails are still being made.
function onError() {
  if (retries >= 3) return
  retries++
  retryTimer = setTimeout(() => {
    if (!activeSrc.value) return
    const url = new URL(activeSrc.value, location.href)
    url.searchParams.set('_r', String(retries))
    activeSrc.value = url.pathname + url.search
  }, 2000 * retries)
}
</script>

<template>
  <div ref="wrapRef" class="thumb-wrap">
    <img
      v-if="activeSrc"
      :src="activeSrc"
      :alt="alt"
      class="thumb-img"
      :class="{ loaded }"
      draggable="false"
      decoding="async"
      @load="loaded = true"
      @error="onError"
    />
    <template v-if="isVideo">
      <span v-if="duration" class="thumb-duration">{{ formatDuration(duration) }}</span>
      <Play v-else class="thumb-play-icon" />
    </template>
  </div>
</template>

<style scoped>
.thumb-wrap {
  position: relative;
  width: 100%;
  height: 100%;
}
.thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  opacity: 0;
  transition: opacity 0.15s ease;
  display: block;
}
.thumb-img.loaded {
  opacity: 1;
}
.thumb-play-icon {
  position: absolute;
  bottom: 6px;
  right: 6px;
  width: 20px;
  height: 20px;
  padding: 3px;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.55);
  color: #fff;
  pointer-events: none;
}
.thumb-duration {
  position: absolute;
  bottom: 5px;
  right: 6px;
  font-size: 0.6875rem;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  color: #fff;
  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.7);
  pointer-events: none;
}
</style>
