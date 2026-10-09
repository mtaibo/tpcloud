<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { Play } from 'lucide-vue-next'

const props = defineProps({ src: String, alt: String, type: { type: String, default: 'image' } })

const wrapRef = ref(null)
const imgRef = ref(null)
const activeSrc = ref('')
const loaded = ref(false)
let retryTimer = null

function onVideoMeta() {
  const el = imgRef.value
  if (el) el.currentTime = Math.min(0.5, (el.duration || 0) / 2)
}

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
    { rootMargin: '400px 0px' }
  )
  if (wrapRef.value) observer.observe(wrapRef.value)
})

onUnmounted(() => {
  observer?.disconnect()
  observer = null
  clearTimeout(retryTimer)
  if (imgRef.value) {
    imgRef.value.src = ''
  }
  activeSrc.value = ''
})

function onError() {
  retryTimer = setTimeout(() => {
    if (activeSrc.value) {
      const url = new URL(activeSrc.value, location.href)
      url.searchParams.set('_r', Date.now())
      activeSrc.value = url.pathname + url.search
    }
  }, 3000)
}
</script>

<template>
  <div ref="wrapRef" class="thumb-wrap">
    <video
      v-if="activeSrc && type === 'video'"
      ref="imgRef"
      :src="activeSrc"
      class="thumb-img"
      :class="{ loaded }"
      muted
      preload="metadata"
      playsinline
      @loadedmetadata="onVideoMeta"
      @seeked="loaded = true"
      @error="onError"
    />
    <img
      v-else-if="activeSrc"
      ref="imgRef"
      :src="activeSrc"
      :alt="alt"
      class="thumb-img"
      :class="{ loaded }"
      draggable="false"
      @load="loaded = true"
      @error="onError"
    />
    <Play v-if="activeSrc && type === 'video'" class="thumb-play-icon" />
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
</style>
