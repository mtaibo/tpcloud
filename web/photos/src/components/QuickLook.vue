<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { ChevronLeft, ChevronRight, Download, Heart, Trash2, X } from 'lucide-vue-next'
import { kindOf } from '../fileTypes.js'

// Quick Look for photos and videos: the file over a dark blur, its name on top, arrows (and the
// keyboard, and swipes) to move through the folder's media. It shows the light versions (a
// 2048 px JPEG, a 720p MP4) so they open at once and play anywhere, like the share page; the
// download button gets the original.
const props = defineProps({
  items: { type: Array, required: true },
  entry: { type: Object, required: true },
  urls: { type: Object, required: true }, // { preview(entry), poster(entry) }
  subtitle: { type: Function, default: null }, // a line under the name instead of "3 de 40"
  deletable: { type: Boolean, default: false },
  favorite: { type: Function, default: null }, // whether an entry is a favourite; shows the heart
})
const emit = defineEmits(['change', 'close', 'download', 'delete', 'favorite'])

const index = computed(() => {
  const i = props.items.indexOf(props.entry)
  return i >= 0 ? i : props.items.findIndex(e => e.name === props.entry.name)
})
const isVideo = computed(() => kindOf(props.entry) === 'video')
const loaded = ref(false)

function go(step) {
  const next = props.items[index.value + step]
  if (next) emit('change', next)
}

// The neighbours' photos are fetched in the background, so stepping through feels instant.
function preloadAround(i) {
  for (const j of [i + 1, i - 1, i + 2]) {
    const item = props.items[j]
    if (!item) continue
    const img = new Image()
    img.src = kindOf(item) === 'video' ? props.urls.poster(item) : props.urls.preview(item)
  }
}
watch(() => props.entry, () => {
  loaded.value = false
  preloadAround(index.value)
}, { immediate: true })

function onKey(e) {
  if (['ArrowLeft', 'ArrowUp'].includes(e.key)) { e.preventDefault(); go(-1) }
  else if (['ArrowRight', 'ArrowDown'].includes(e.key)) { e.preventDefault(); go(1) }
  else if (e.key === 'Escape' || e.key === ' ') { e.preventDefault(); emit('close') }
  else if (e.key === '.' && props.favorite) { e.preventDefault(); emit('favorite', props.entry) } // as in Photos
}

let startX = null
function onTouchStart(e) { startX = e.touches[0].clientX }
function onTouchEnd(e) {
  if (startX === null) return
  const dx = e.changedTouches[0].clientX - startX
  if (Math.abs(dx) > 50) go(dx < 0 ? 1 : -1)
  startX = null
}

onMounted(() => {
  document.addEventListener('keydown', onKey, true)
  document.documentElement.style.overflow = 'hidden'
})
onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKey, true)
  document.documentElement.style.overflow = ''
})
</script>

<template>
  <Teleport to="body">
    <div class="look" @click.self="emit('close')" @touchstart.passive="onTouchStart" @touchend="onTouchEnd">
      <header class="look-bar">
        <button type="button" class="look-btn" aria-label="Cerrar" @click="emit('close')"><X :size="20" :stroke-width="2.2" /></button>
        <div class="look-title">
          <span class="look-name">{{ entry.name }}</span>
          <span v-if="subtitle" class="look-count">{{ subtitle(entry) }}</span>
          <span v-else-if="items.length > 1" class="look-count">{{ index + 1 }} de {{ items.length }}</span>
        </div>
        <div class="look-actions">
          <button v-if="favorite" type="button" class="look-btn" :class="{ faved: favorite(entry) }" :aria-label="favorite(entry) ? 'Quitar de Favoritos' : 'Añadir a Favoritos'" @click="emit('favorite', entry)">
            <Heart :size="19" :stroke-width="2" :fill="favorite(entry) ? 'currentColor' : 'none'" />
          </button>
          <button v-if="deletable" type="button" class="look-btn" aria-label="Eliminar" @click="emit('delete', entry)"><Trash2 :size="19" :stroke-width="2" /></button>
          <button type="button" class="look-btn" aria-label="Descargar" @click="emit('download', entry)"><Download :size="19" :stroke-width="2" /></button>
        </div>
      </header>

      <div class="stage" @click.self="emit('close')">
        <video
          v-if="isVideo"
          :key="urls.preview(entry)"
          class="media"
          :src="urls.preview(entry)"
          :poster="urls.poster(entry)"
          controls
          autoplay
          playsinline
          preload="auto"
        />
        <template v-else>
          <span v-if="!loaded" class="spinner-wrap"><span class="ring" /></span>
          <img :key="urls.preview(entry)" class="media" :class="{ loaded }" :src="urls.preview(entry)" :alt="entry.name" @load="loaded = true" />
        </template>
      </div>

      <button v-if="index > 0" type="button" class="arrow left" aria-label="Anterior" @click="go(-1)"><ChevronLeft :size="26" :stroke-width="2.2" /></button>
      <button v-if="index < items.length - 1" type="button" class="arrow right" aria-label="Siguiente" @click="go(1)"><ChevronRight :size="26" :stroke-width="2.2" /></button>
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
  background: rgba(0, 0, 0, 0.82);
  backdrop-filter: blur(30px) saturate(140%);
  -webkit-backdrop-filter: blur(30px) saturate(140%);
  animation: look-in 0.25s var(--ease);
}
@keyframes look-in { from { opacity: 0; } }
.look-bar {
  flex-shrink: 0;
  display: grid;
  grid-template-columns: 148px 1fr 148px;
  align-items: center;
  gap: 12px;
  padding: calc(12px + env(safe-area-inset-top)) 14px 10px;
}
.look-actions { display: flex; gap: 8px; justify-content: flex-end; }
.look-btn.faved { color: #ff375f; }
@media (max-width: 767px) { .look-bar { grid-template-columns: 44px 1fr auto; } }
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
.spinner-wrap { position: absolute; inset: 0; display: grid; place-items: center; pointer-events: none; }
.ring { width: 30px; height: 30px; border-radius: 50%; border: 2.5px solid rgba(255, 255, 255, 0.2); border-top-color: #fff; animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.media {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  border-radius: 10px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.5);
  user-select: none;
  -webkit-user-drag: none;
}
img.media { opacity: 0; transform: scale(0.98); transition: opacity 0.2s, transform 0.3s var(--ease); }
img.media.loaded { opacity: 1; transform: none; }
.arrow { position: absolute; top: 50%; margin-top: -22px; }
.arrow.left { left: 16px; }
.arrow.right { right: 16px; }
@media (max-width: 767px) {
  .stage { padding: 8px 0 calc(16px + env(safe-area-inset-bottom)); }
  .media { border-radius: 0; }
  .arrow { display: none; }
}
</style>
