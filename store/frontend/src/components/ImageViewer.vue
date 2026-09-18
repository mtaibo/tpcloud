<script setup>
import { onMounted, onUnmounted } from 'vue'
import { X } from 'lucide-vue-next'

const props = defineProps({ src: String, name: String })
const emit = defineEmits(['close'])

function onKeydown(e) {
  if (e.key === 'Escape') emit('close')
}

onMounted(() => document.addEventListener('keydown', onKeydown))
onUnmounted(() => document.removeEventListener('keydown', onKeydown))
</script>

<template>
  <Teleport to="body">
    <Transition name="viewer">
      <div class="viewer-backdrop" @click.self="emit('close')">
        <button class="viewer-close" @click="emit('close')" title="Close">
          <X class="close-icon" />
        </button>
        <img class="viewer-img" :src="src" :alt="name" />
        <span class="viewer-name">{{ name }}</span>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.viewer-backdrop {
  position: fixed;
  inset: 0;
  z-index: 10000;
  background: rgba(0, 0, 0, 0.92);
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.viewer-img {
  max-width: 90vw;
  max-height: 88vh;
  object-fit: contain;
  border-radius: 6px;
  box-shadow: 0 20px 80px rgba(0, 0, 0, 0.7);
  user-select: none;
  -webkit-user-drag: none;
}

.viewer-close {
  position: absolute;
  top: 20px;
  right: 20px;
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.1);
  border: 0.5px solid rgba(255, 255, 255, 0.15);
  border-radius: 50%;
  color: rgba(255, 255, 255, 0.8);
  transition: background 0.12s, color 0.12s;
}

.viewer-close:hover {
  background: rgba(255, 255, 255, 0.18);
  color: #fff;
}

.close-icon {
  width: 16px;
  height: 16px;
  stroke-width: 2.5;
}

.viewer-name {
  position: absolute;
  bottom: 24px;
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

.viewer-enter-active {
  transition: opacity 0.18s ease;
}
.viewer-leave-active {
  transition: opacity 0.12s ease;
}
.viewer-enter-from,
.viewer-leave-to {
  opacity: 0;
}
</style>
