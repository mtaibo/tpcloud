<script setup>
import FileIcon from './FileIcon.vue'
import GalleryThumb from './GalleryThumb.vue'

// A file's picture: its thumbnail for photos and videos (framed like Finder's previews),
// its icon for everything else.
defineProps({
  entry: { type: Object, required: true },
  size: { type: Number, default: 64 },
  thumb: { type: String, default: '' },
  video: { type: Boolean, default: false },
  fill: { type: Boolean, default: false },
})
</script>

<template>
  <span v-if="thumb" class="thumb" :class="{ fill }" :style="fill ? null : { width: `${size}px`, height: `${size}px` }">
    <GalleryThumb :src="thumb" :alt="entry.name" :type="video ? 'video' : 'image'" :small="size < 48 && !fill" />
  </span>
  <FileIcon v-else :entry="entry" :size="size" />
</template>

<style scoped>
.thumb {
  display: block;
  flex-shrink: 0;
  overflow: hidden;
  border-radius: 6px;
  background: var(--fill-3);
  box-shadow: 0 0 0 0.5px rgba(0, 0, 0, 0.12), 0 1px 3px rgba(0, 0, 0, 0.12);
}
.thumb.fill { width: 100%; height: 100%; border-radius: 0; box-shadow: none; }
</style>
