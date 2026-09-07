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
  <nav class="sidebar">
    <p class="section-label">External Disk</p>

    <button :class="['nav-item', { active: isActive('external', 'shared') }]" @click="go('external', 'shared')">
      <Folder class="nav-icon" />
      <span>Shared</span>
    </button>

    <button :class="['nav-item', { active: isActive('external', `users/${user.email}`) }]" @click="go('external', `users/${user.email}`)">
      <Folder class="nav-icon" />
      <span class="label-text">My Storage</span>
    </button>

    <template v-if="user.is_admin">
      <button :class="['nav-item', { active: isActive('external', '') }]" @click="go('external', '')">
        <HardDrive class="nav-icon" />
        <span>Root</span>
      </button>

      <p class="section-label" style="margin-top: 1.25rem;">Server</p>

      <button :class="['nav-item', { active: isActive('server', '') }]" @click="go('server', '')">
        <Server class="nav-icon" />
        <span>System</span>
      </button>
    </template>
  </nav>
</template>

<style scoped>
.sidebar {
  width: 200px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  padding: 0.75rem 0.5rem;
  overflow-y: auto;
  background: rgba(255, 255, 255, 0.04);
  backdrop-filter: blur(20px) saturate(140%);
  -webkit-backdrop-filter: blur(20px) saturate(140%);
  border-right: 0.5px solid rgba(255, 255, 255, 0.1);
}

.section-label {
  padding: 0.25rem 0.75rem 0.35rem;
  font-size: 0.65rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #525252;
  user-select: none;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  width: 100%;
  padding: 0.45rem 0.75rem;
  border-radius: 8px;
  font-size: 0.85rem;
  text-align: left;
  color: #a3a3a3;
  background: none;
  border: 0.5px solid transparent;
  cursor: pointer;
  transition: background 0.15s, color 0.15s, border-color 0.15s;
  margin-bottom: 0.1rem;
}

.nav-item:hover:not(.active) {
  background: rgba(255, 255, 255, 0.06);
  color: #e4e4e7;
}

.nav-item.active {
  color: #fff;
  background: rgba(0, 122, 255, 0.18);
  border-color: rgba(0, 122, 255, 0.3);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.nav-icon {
  width: 15px;
  height: 15px;
  flex-shrink: 0;
}

.label-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
