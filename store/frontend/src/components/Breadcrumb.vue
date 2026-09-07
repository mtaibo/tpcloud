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
  <div class="flex items-center gap-1 text-sm min-w-0 overflow-hidden">
    <button
      @click="goTo('')"
      class="flex items-center gap-1 shrink-0 transition-colors"
      style="color: #d1d1d6;"
      @mouseover="(e) => e.currentTarget.style.color = '#fff'"
      @mouseleave="(e) => e.currentTarget.style.color = '#d1d1d6'"
    >
      <HardDrive v-if="location === 'external'" class="w-4 h-4" />
      <Server v-else class="w-4 h-4" />
      <span>{{ location === 'external' ? 'Disco' : 'Servidor' }}</span>
    </button>

    <template v-for="seg in segments" :key="seg.path">
      <span style="color: #636366;" class="shrink-0">/</span>
      <button
        @click="goTo(seg.path)"
        class="transition-colors truncate max-w-[8rem]"
        style="color: #d1d1d6;"
        @mouseover="(e) => e.currentTarget.style.color = '#fff'"
        @mouseleave="(e) => e.currentTarget.style.color = '#d1d1d6'"
      >
        {{ seg.name }}
      </button>
    </template>
  </div>
</template>
