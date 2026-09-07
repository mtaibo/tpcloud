<script setup>
import { HardDrive, Folder, Server } from 'lucide-vue-next'

const props = defineProps({
  user: Object,
  location: String,
  currentPath: String,
})

const emit = defineEmits(['navigate'])

function isActive(loc, path) {
  return props.location === loc && props.currentPath === path
}

function go(loc, path) {
  emit('navigate', loc, path)
}
</script>

<template>
  <nav
    class="w-52 shrink-0 flex flex-col py-3 overflow-y-auto"
    style="background: #1c1c1e; border-right: 1px solid rgba(255,255,255,0.08);"
  >
    <!-- Disco Externo -->
    <p class="px-4 pt-1 pb-1 text-[10px] font-semibold uppercase tracking-wider" style="color: #636366;">
      Disco Externo
    </p>

    <button
      @click="go('external', 'shared')"
      class="flex items-center gap-2 mx-2 px-2 py-1.5 rounded-lg text-sm text-left transition-colors"
      :style="isActive('external', 'shared')
        ? 'background: #007AFF; color: #fff;'
        : 'color: #d1d1d6;'"
      @mouseover="(e) => { if (!isActive('external', 'shared')) e.currentTarget.style.background = 'rgba(255,255,255,0.06)' }"
      @mouseleave="(e) => { if (!isActive('external', 'shared')) e.currentTarget.style.background = '' }"
    >
      <Folder class="w-4 h-4 shrink-0" />
      <span>Compartido</span>
    </button>

    <button
      @click="go('external', `users/${user.email}`)"
      class="flex items-center gap-2 mx-2 px-2 py-1.5 rounded-lg text-sm text-left transition-colors"
      :style="isActive('external', `users/${user.email}`)
        ? 'background: #007AFF; color: #fff;'
        : 'color: #d1d1d6;'"
      @mouseover="(e) => { if (!isActive('external', `users/${user.email}`)) e.currentTarget.style.background = 'rgba(255,255,255,0.06)' }"
      @mouseleave="(e) => { if (!isActive('external', `users/${user.email}`)) e.currentTarget.style.background = '' }"
    >
      <Folder class="w-4 h-4 shrink-0" />
      <span class="truncate">Mi almacenamiento</span>
    </button>

    <template v-if="user.is_admin">
      <button
        @click="go('external', '')"
        class="flex items-center gap-2 mx-2 px-2 py-1.5 rounded-lg text-sm text-left transition-colors"
        :style="isActive('external', '')
          ? 'background: #007AFF; color: #fff;'
          : 'color: #d1d1d6;'"
        @mouseover="(e) => { if (!isActive('external', '')) e.currentTarget.style.background = 'rgba(255,255,255,0.06)' }"
        @mouseleave="(e) => { if (!isActive('external', '')) e.currentTarget.style.background = '' }"
      >
        <HardDrive class="w-4 h-4 shrink-0" />
        <span>Raíz disco</span>
      </button>

      <!-- Servidor -->
      <p class="px-4 pt-4 pb-1 text-[10px] font-semibold uppercase tracking-wider" style="color: #636366;">
        Servidor
      </p>

      <button
        @click="go('server', '')"
        class="flex items-center gap-2 mx-2 px-2 py-1.5 rounded-lg text-sm text-left transition-colors"
        :style="isActive('server', '')
          ? 'background: #007AFF; color: #fff;'
          : 'color: #d1d1d6;'"
        @mouseover="(e) => { if (!isActive('server', '')) e.currentTarget.style.background = 'rgba(255,255,255,0.06)' }"
        @mouseleave="(e) => { if (!isActive('server', '')) e.currentTarget.style.background = '' }"
      >
        <Server class="w-4 h-4 shrink-0" />
        <span>Sistema</span>
      </button>
    </template>
  </nav>
</template>
