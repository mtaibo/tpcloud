<script setup>
import { computed } from 'vue'
import { Link2, ArrowUp } from 'lucide-vue-next'
import { FOLDER_ICONS_MAP } from '../folderIcons.js'
import { KIND_COLORS, extOf, kindOf } from '../fileTypes.js'

// Finder-style icons: a blue folder (with its custom symbol pressed into the front), or a page
// with a folded corner and the file's extension in its kind's colour.
const props = defineProps({
  entry: { type: Object, required: true },
  size: { type: Number, default: 64 },
})

const kind = computed(() => kindOf(props.entry))
const isFolder = computed(() => kind.value === 'folder' || kind.value === 'share')
const glyph = computed(() => {
  if (kind.value === 'share') return Link2
  return FOLDER_ICONS_MAP[props.entry.icon_name] || null
})
const label = computed(() => {
  const ext = extOf(props.entry)
  return ext.length <= 4 ? ext.toUpperCase() : ext.slice(0, 4).toUpperCase()
})
const color = computed(() => KIND_COLORS[kind.value] || 'var(--gray)')
const uid = Math.random().toString(36).slice(2, 8)
</script>

<template>
  <span class="file-icon" :style="{ width: `${size}px`, height: `${size}px` }">
    <svg v-if="isFolder" viewBox="0 0 96 96" aria-hidden="true">
      <defs>
        <linearGradient :id="`front-${uid}`" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" class="folder-front-top" />
          <stop offset="1" class="folder-front-bottom" />
        </linearGradient>
      </defs>
      <path class="folder-back" d="M8 22a6 6 0 0 1 6-6h22.5a6 6 0 0 1 4.2 1.7l4.6 4.6a6 6 0 0 0 4.2 1.7H82a6 6 0 0 1 6 6V74a6 6 0 0 1-6 6H14a6 6 0 0 1-6-6z" />
      <rect x="8" y="31" width="80" height="51" rx="6" :fill="`url(#front-${uid})`" />
      <rect x="8" y="31" width="80" height="1.2" rx="0.6" class="folder-shine" />
    </svg>
    <svg v-else-if="kind !== 'upload'" viewBox="0 0 96 96" aria-hidden="true">
      <path class="page" d="M22 6h34l22 22v56a6 6 0 0 1-6 6H22a6 6 0 0 1-6-6V12a6 6 0 0 1 6-6z" />
      <path class="page-fold" d="M56 6v16a6 6 0 0 0 6 6h16z" />
      <text v-if="label" x="47" y="74" text-anchor="middle" class="page-label" :style="{ fill: color }">{{ label }}</text>
      <rect v-else x="30" y="52" width="34" height="5" rx="2.5" class="page-line" />
    </svg>
    <span v-else class="upload">
      <ArrowUp :size="size * 0.42" :stroke-width="2.4" />
    </span>
    <component :is="glyph" v-if="isFolder && glyph" class="glyph" :size="size * 0.34" :stroke-width="2.2" />
  </span>
</template>

<style scoped>
.file-icon { position: relative; display: inline-grid; place-items: center; flex-shrink: 0; }
.file-icon svg { width: 100%; height: 100%; display: block; overflow: visible; }
.folder-back { fill: #5aa7ea; }
.folder-front-top { stop-color: #8ccbff; }
.folder-front-bottom { stop-color: #5fb2f6; }
.folder-shine { fill: rgba(255, 255, 255, 0.55); }
.glyph { position: absolute; top: 58%; left: 50%; transform: translate(-50%, -50%); color: #2f86d6; opacity: 0.85; }
.page { fill: #fff; stroke: rgba(0, 0, 0, 0.12); stroke-width: 1.2; }
.page-fold { fill: #e6e6eb; stroke: rgba(0, 0, 0, 0.1); stroke-width: 1; }
.page-label { font-family: var(--font); font-size: 15px; font-weight: 700; letter-spacing: 0.3px; }
.page-line { fill: #d1d1d6; }
.upload {
  width: 82%;
  height: 82%;
  display: grid;
  place-items: center;
  border-radius: 22%;
  color: #fff;
  background: var(--orange);
}
@media (prefers-color-scheme: dark) {
  .folder-back { fill: #3f8fd6; }
  .folder-front-top { stop-color: #6db8f5; }
  .folder-front-bottom { stop-color: #4a9fe8; }
  .folder-shine { fill: rgba(255, 255, 255, 0.35); }
  .glyph { color: #1f6db5; }
  .page { fill: #f2f2f7; stroke: rgba(255, 255, 255, 0.08); }
  .page-fold { fill: #c7c7cc; }
}
</style>
