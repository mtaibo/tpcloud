<script setup>
import { computed } from 'vue'
import { HardDrive, Server } from 'lucide-vue-next'

const props = defineProps({
  location: String,
  path: String,
})

const emit = defineEmits(['navigate'])

const segments = computed(() => {
  const parts = (props.path || '').split('/').filter(Boolean)
  return parts.map((part, i) => ({
    name: part,
    path: parts.slice(0, i + 1).join('/'),
  }))
})

function goTo(path) {
  emit('navigate', props.location, path)
}
</script>

<template>
  <div class="breadcrumb">
    <button class="seg root-seg" @click="goTo('')">
      <HardDrive v-if="location === 'external'" class="seg-icon" />
      <Server v-else class="seg-icon" />
      <span>{{ location === 'external' ? 'Disk' : 'Server' }}</span>
    </button>

    <template v-for="seg in segments" :key="seg.path">
      <span class="sep">/</span>
      <button class="seg" @click="goTo(seg.path)">{{ seg.name }}</button>
    </template>
  </div>
</template>

<style scoped>
.breadcrumb {
  display: flex;
  align-items: center;
  gap: 0.2rem;
  font-size: 0.8rem;
  min-width: 0;
  overflow: hidden;
}

.seg {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  background: none;
  border: none;
  color: #737373;
  cursor: pointer;
  transition: color 0.15s;
  max-width: 8rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-shrink: 1;
  padding: 0;
}

.root-seg { flex-shrink: 0; }

.seg:hover { color: #fff; }

.seg-icon { width: 14px; height: 14px; flex-shrink: 0; }

.sep {
  color: #404040;
  flex-shrink: 0;
  padding: 0 0.1rem;
}
</style>
