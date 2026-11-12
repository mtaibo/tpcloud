<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { Play } from 'lucide-vue-next'
import { formatDuration } from '../mediaUrls.js'

// A photo or video's square thumbnail, loaded when it comes near the screen. Videos get a JPEG
// cut from their poster frame too, so a tile never loads the video itself.
const props = defineProps({
  src: String,
  alt: String,
  type: { type: String, default: 'image' },
  duration: { type: Number, default: null },
  small: Boolean,
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

// A folder opened for the first time may still be having its thumbnails made.
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
    <template v-if="type === 'video' && !small">
      <span v-if="duration" class="thumb-duration">{{ formatDuration(duration) }}</span>
      <span v-else class="thumb-play"><Play class="thumb-play-icon" fill="currentColor" /></span>
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
.thumb-play {
  position: absolute;
  right: 6px;
  bottom: 6px;
  width: 22px;
  height: 22px;
  display: grid;
  place-items: center;
  border-radius: 50%;
  color: #fff;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  pointer-events: none;
}
.thumb-play-icon { width: 10px; height: 10px; margin-left: 1px; }
.thumb-duration {
  position: absolute;
  bottom: 5px;
  right: 6px;
  font-size: 11px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  color: #fff;
  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.7);
  pointer-events: none;
}
</style>
