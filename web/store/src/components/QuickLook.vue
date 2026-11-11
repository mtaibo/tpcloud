<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { ChevronLeft, ChevronRight, Download, X } from 'lucide-vue-next'
import { kindOf } from '../fileTypes.js'

// Quick Look for photos and videos: the file over a dark blur, its name on top, arrows (and the
// keyboard, and swipes) to move through the folder's media.
const props = defineProps({
  items: { type: Array, required: true },
  entry: { type: Object, required: true },
  srcFor: { type: Function, required: true },
})
const emit = defineEmits(['change', 'close', 'download'])

const index = computed(() => props.items.findIndex(e => e.name === props.entry.name))
const isVideo = computed(() => kindOf(props.entry) === 'video')
const loaded = ref(false)

function go(step) {
  const next = props.items[index.value + step]
  if (next) {
    loaded.value = false
    emit('change', next)
  }
}

function onKey(e) {
  if (['ArrowLeft', 'ArrowUp'].includes(e.key)) { e.preventDefault(); go(-1) }
  else if (['ArrowRight', 'ArrowDown'].includes(e.key)) { e.preventDefault(); go(1) }
  else if (e.key === 'Escape' || e.key === ' ') { e.preventDefault(); emit('close') }
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
          <span v-if="items.length > 1" class="look-count">{{ index + 1 }} de {{ items.length }}</span>
        </div>
        <button type="button" class="look-btn" aria-label="Descargar" @click="emit('download', entry)"><Download :size="19" :stroke-width="2" /></button>
      </header>

      <div class="stage" @click.self="emit('close')">
        <video v-if="isVideo" :key="entry.name" class="media" :src="srcFor(entry)" controls autoplay playsinline />
        <img v-else :key="entry.name" class="media" :class="{ loaded }" :src="srcFor(entry)" :alt="entry.name" @load="loaded = true" />
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
  grid-template-columns: 44px 1fr 44px;
  align-items: center;
  gap: 12px;
  padding: calc(12px + env(safe-area-inset-top)) 14px 10px;
}
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
.stage { flex: 1; min-height: 0; display: grid; place-items: center; padding: 8px 72px calc(28px + env(safe-area-inset-bottom)); }
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
