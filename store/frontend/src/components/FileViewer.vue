<script setup>
import { computed, onMounted, onUnmounted } from 'vue'

const props = defineProps({
  filename: String,
  contentType: String,
  fileUrl: String,
})

const isImage = computed(() => props.contentType?.startsWith('image/'))
const isVideo = computed(() => props.contentType?.startsWith('video/'))
const isAudio = computed(() => props.contentType?.startsWith('audio/'))

onMounted(() => { document.title = props.filename })
onUnmounted(() => { document.title = 'TPStore' })
</script>

<template>
  <div class="viewer">
    <img v-if="isImage" :src="fileUrl" :alt="filename" class="viewer-img" />
    <video v-else-if="isVideo" :src="fileUrl" controls autoplay class="viewer-video" />
    <audio v-else-if="isAudio" :src="fileUrl" controls autoplay class="viewer-audio" />
    <iframe v-else :src="fileUrl" class="viewer-frame" />
  </div>
</template>

<style scoped>
.viewer {
  width: 100vw;
  height: 100dvh;
  background: #000;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}
.viewer-img {
  max-width: 100%;
  max-height: 100dvh;
  object-fit: contain;
}
.viewer-video {
  max-width: 100%;
  max-height: 100dvh;
}
.viewer-audio {
  width: 300px;
}
.viewer-frame {
  width: 100vw;
  height: 100dvh;
  border: none;
  background: #fff;
}
</style>
