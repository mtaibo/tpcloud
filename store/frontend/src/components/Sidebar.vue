<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { HardDrive, Folder, Server, ChevronRight, User, LogOut } from 'lucide-vue-next'

const LOGIN_URL = 'https://login.migueltaibo.com'

const props = defineProps({
  user: Object,
  location: String,
  currentPath: String,
})

const emit = defineEmits(['navigate'])

const serverOpen = ref(false)
const menuOpen = ref(false)
const cardRef = ref(null)

function isActive(loc, path) {
  return props.location === loc && props.currentPath === path
}

function go(loc, path) {
  emit('navigate', loc, path)
}

function onClickOutside(e) {
  if (cardRef.value && !cardRef.value.contains(e.target)) menuOpen.value = false
}

async function logout() {
  await fetch('/auth/passkey/logout', { method: 'POST' })
  window.location.href = `${LOGIN_URL}/`
}

onMounted(() => document.addEventListener('click', onClickOutside))
onUnmounted(() => document.removeEventListener('click', onClickOutside))
</script>

<template>
  <aside class="sidebar">

    <div class="sidebar-brand">
      <span>TPStore</span>
    </div>

    <nav class="sidebar-nav">

      <div class="separator-row separator-first">
        <p class="section-label">External Disk</p>
      </div>

      <button :class="['nav-item', { active: isActive('external', 'shared') }]" @click="go('external', 'shared')">
        <Folder class="nav-icon" />
        <span>Shared</span>
      </button>

      <button :class="['nav-item', { active: isActive('external', `users/${user.email}`) }]" @click="go('external', `users/${user.email}`)">
        <Folder class="nav-icon" />
        <span>My Storage</span>
      </button>

      <template v-if="user.is_admin">
        <button :class="['nav-item', { active: isActive('external', '') }]" @click="go('external', '')">
          <HardDrive class="nav-icon" />
          <span>Root</span>
        </button>

        <div class="separator-row separator-clickable" @click="serverOpen = !serverOpen">
          <p class="section-label">Server</p>
          <ChevronRight class="chevron" :class="{ open: serverOpen }" />
        </div>

        <template v-if="serverOpen">
          <button :class="['nav-item', { active: isActive('server', '') }]" @click="go('server', '')">
            <Server class="nav-icon" />
            <span>System</span>
          </button>
        </template>
      </template>

    </nav>

    <footer class="sidebar-footer">
      <div ref="cardRef" class="user-card">

        <Transition name="fade">
          <div v-if="menuOpen" class="user-menu">
            <button @click="logout" class="user-menu-item logout-item">
              <LogOut class="menu-icon" />
              Log out
            </button>
          </div>
        </Transition>

        <button @click="menuOpen = !menuOpen" class="user-btn">
          <div class="user-avatar">
            <User class="avatar-icon" />
          </div>
          <span class="user-display-name">{{ user?.display_name ?? '…' }}</span>
        </button>

      </div>
    </footer>

  </aside>
</template>

<style scoped>
.sidebar {
  width: 240px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: #111113;
  box-shadow: 1px 0 0 rgba(255, 255, 255, 0.06);
}

.sidebar-brand {
  height: 56px;
  display: flex;
  align-items: center;
  padding: 0 1.25rem;
  flex-shrink: 0;
}

.sidebar-brand span {
  color: #fff;
  font-weight: 600;
  font-size: 0.875rem;
  letter-spacing: 0.03em;
  user-select: none;
}

.sidebar-nav {
  flex: 1;
  padding: 0.75rem;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.separator-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 0.5rem 0.25rem;
}

.separator-first {
  padding-top: 0.25rem;
}

.separator-clickable {
  cursor: pointer;
}

.separator-clickable:hover .chevron {
  opacity: 1;
}

.section-label {
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #636366;
  user-select: none;
}

.chevron {
  width: 14px;
  height: 14px;
  color: #636366;
  opacity: 0;
  transition: transform 0.2s, opacity 0.2s;
}

.chevron.open {
  transform: rotate(90deg);
  opacity: 1;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 1rem;
  width: 100%;
  padding: 0.625rem 1rem;
  border-radius: 8px;
  font-size: 0.875rem;
  font-weight: 500;
  text-align: left;
  color: #fff;
  background: transparent;
  border: none;
  cursor: pointer;
  transition: background 0.2s;
}

.nav-item:hover:not(.active) {
  background: rgba(255, 255, 255, 0.05);
}

.nav-item.active {
  background: #1c1c1e;
  font-weight: 700;
}

.nav-icon {
  width: 20px;
  height: 20px;
  flex-shrink: 0;
  color: #007AFF;
}

.sidebar-footer {
  padding: 0.5rem 1rem 1.5rem;
  flex-shrink: 0;
}

.user-card {
  position: relative;
}

.user-menu {
  position: absolute;
  bottom: 100%;
  left: 0;
  right: 0;
  margin-bottom: 0.5rem;
  background: #1c1c1e;
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.user-menu-item {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  width: 100%;
  padding: 0.625rem 0.75rem;
  font-size: 0.875rem;
  color: rgba(255, 255, 255, 0.7);
  background: none;
  border: none;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  text-align: left;
}

.user-menu-item:hover {
  background: rgba(255, 255, 255, 0.05);
}

.logout-item:hover {
  color: #f87171;
}

.menu-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

.user-btn {
  display: flex;
  align-items: center;
  width: 100%;
  padding: 0.625rem 1rem;
  border-radius: 8px;
  background: none;
  border: none;
  cursor: pointer;
  transition: background 0.15s;
}

.user-btn:hover {
  background: rgba(255, 255, 255, 0.05);
}

.user-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: #1c1c1e;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.avatar-icon {
  width: 15px;
  height: 15px;
  color: #007AFF;
}

.user-display-name {
  margin-left: 1rem;
  font-size: 0.875rem;
  font-weight: 500;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-align: left;
}

.fade-enter-active, .fade-leave-active {
  transition: opacity 0.15s, transform 0.15s;
}
.fade-enter-from, .fade-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>
