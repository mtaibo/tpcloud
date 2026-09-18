<script setup>
import { ref, onMounted, onUnmounted } from 'vue'

const props = defineProps({ src: String, alt: String })

const wrapRef = ref(null)
const imgRef = ref(null)
const activeSrc = ref('')
const loaded = ref(false)

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
  if (imgRef.value) {
    imgRef.value.src = ''
  }
  activeSrc.value = ''
})
</script>

<template>
  <div ref="wrapRef" class="thumb-wrap">
    <img
      v-if="activeSrc"
      ref="imgRef"
      :src="activeSrc"
      :alt="alt"
      class="thumb-img"
      :class="{ loaded }"
      draggable="false"
      @load="loaded = true"
    />
  </div>
</template>

<style scoped>
.thumb-wrap {
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
</style>
