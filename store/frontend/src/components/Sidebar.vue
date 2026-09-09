<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { HardDrive, Folder, Server, ChevronRight, User, LogOut, Cloud, Star, Plus, X } from 'lucide-vue-next'

const LOGIN_URL = 'https://login.migueltaibo.com'

const props = defineProps({
  user: Object,
  location: String,
  currentPath: String,
})

const emit = defineEmits(['navigate'])

const serverOpen = ref(true)
const cloudOpen = ref(true)
const favouritesOpen = ref(true)
const menuOpen = ref(false)
const cardRef = ref(null)
const favourites = ref([])

function loadFavourites() {
  try {
    const stored = localStorage.getItem('tpcloud-favourites')
    if (stored) favourites.value = JSON.parse(stored)
  } catch {}
}

function saveFavourites() {
  localStorage.setItem('tpcloud-favourites', JSON.stringify(favourites.value))
}

function derivLabel(loc, path) {
  if (!path) return loc === 'server' ? 'Server Root' : 'Disk Root'
  const parts = path.split('/').filter(Boolean)
  const last = parts[parts.length - 1]
  if (path === `users/${props.user.email}`) return 'Personal'
  if (last === 'shared') return 'Shared'
  return last.charAt(0).toUpperCase() + last.slice(1)
}

function addFavourite() {
  const loc = props.location
  const path = props.currentPath
  const exists = favourites.value.some(f => f.location === loc && f.path === path)
  if (exists) return
  favourites.value.push({ id: `${loc}:${path}`, label: derivLabel(loc, path), location: loc, path })
  saveFavourites()
}

function removeFavourite(id) {
  favourites.value = favourites.value.filter(f => f.id !== id)
  saveFavourites()
}

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

onMounted(() => {
  document.addEventListener('click', onClickOutside)
  loadFavourites()
})
onUnmounted(() => document.removeEventListener('click', onClickOutside))
</script>

<template>
  <aside class="sidebar">

    <div class="sidebar-brand" />

    <nav class="sidebar-nav">

      <!-- FAVOURITES -->
      <div class="separator-row separator-first separator-clickable" @click="favouritesOpen = !favouritesOpen">
        <p class="section-label">Favourites</p>
        <div class="section-actions">
          <button class="add-btn" @click.stop="addFavourite" title="Add current folder">
            <Plus class="add-icon" />
          </button>
          <ChevronRight class="chevron" :class="{ open: favouritesOpen }" />
        </div>
      </div>

      <template v-if="favouritesOpen">
        <p v-if="favourites.length === 0" class="empty-hint">No favourites yet</p>
        <div v-for="fav in favourites" :key="fav.id" class="fav-row">
          <button :class="['nav-item', { active: isActive(fav.location, fav.path) }]" @click="go(fav.location, fav.path)">
            <Star class="nav-icon fav-star" />
            <span>{{ fav.label }}</span>
          </button>
          <button class="remove-btn" @click="removeFavourite(fav.id)" title="Remove">
            <X class="remove-icon" />
          </button>
        </div>
      </template>

      <!-- CLOUD -->
      <div class="separator-row separator-clickable" @click="cloudOpen = !cloudOpen">
        <p class="section-label">Cloud</p>
        <ChevronRight class="chevron" :class="{ open: cloudOpen }" />
      </div>

      <template v-if="cloudOpen">
        <button :class="['nav-item', { active: isActive('external', 'shared') }]" @click="go('external', 'shared')">
          <Folder class="nav-icon" />
          <span>Shared</span>
        </button>
        <button :class="['nav-item', { active: isActive('external', `users/${user.email}`) }]" @click="go('external', `users/${user.email}`)">
          <Folder class="nav-icon" />
          <span>Personal</span>
        </button>
      </template>

      <!-- SERVER (admin only) -->
      <template v-if="user.is_admin">
        <div class="separator-row separator-clickable" @click="serverOpen = !serverOpen">
          <p class="section-label">Server</p>
          <ChevronRight class="chevron" :class="{ open: serverOpen }" />
        </div>

        <template v-if="serverOpen">
          <button :class="['nav-item', { active: isActive('server', '') }]" @click="go('server', '')">
            <Server class="nav-icon" />
            <span>System</span>
          </button>
          <button :class="['nav-item', { active: isActive('external', '') }]" @click="go('external', '')">
            <HardDrive class="nav-icon" />
            <span>Root</span>
          </button>
        </template>
      </template>

    </nav>

    <footer class="sidebar-footer">
      <div ref="cardRef" class="user-card">

        <Transition name="fade">
          <div v-if="menuOpen" class="user-menu">
            <a href="https://accounts.migueltaibo.com" class="user-menu-item">
              <User class="menu-icon" />
              Account
            </a>
            <a href="https://cloud.migueltaibo.com" class="user-menu-item">
              <Cloud class="menu-icon" />
              TPCloud
            </a>
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
  cursor: default;
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

.section-actions {
  display: flex;
  align-items: center;
  gap: 0.125rem;
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

.add-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2px;
  background: none;
  border: none;
  border-radius: 4px;
  cursor: default;
  opacity: 0;
  transition: opacity 0.15s;
}

.separator-clickable:hover .add-btn {
  opacity: 1;
}

.add-icon {
  width: 12px;
  height: 12px;
  color: #636366;
  transition: color 0.15s;
}

.add-btn:hover .add-icon {
  color: #fff;
}

.empty-hint {
  padding: 0.2rem 1rem 0.25rem;
  font-size: 0.75rem;
  color: #3a3a3c;
  user-select: none;
}

.fav-row {
  position: relative;
  display: flex;
  align-items: center;
}

.fav-row .nav-item {
  flex: 1;
}

.remove-btn {
  position: absolute;
  right: 0.375rem;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 3px;
  background: none;
  border: none;
  border-radius: 4px;
  cursor: default;
  opacity: 0;
  transition: opacity 0.15s;
}

.fav-row:hover .remove-btn {
  opacity: 1;
}

.remove-icon {
  width: 12px;
  height: 12px;
  color: #636366;
  transition: color 0.15s;
}

.remove-btn:hover .remove-icon {
  color: #f87171;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 1rem;
  width: 100%;
  padding: 0.35rem 1rem;
  border-radius: 8px;
  font-size: 0.875rem;
  font-weight: 500;
  text-align: left;
  color: #fff;
  background: transparent;
  border: none;
  cursor: default;
  transition: background 0.2s;
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

.fav-star {
  color: #FFD60A;
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
  cursor: default;
  transition: background 0.15s, color 0.15s;
  text-align: left;
  text-decoration: none;
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
  cursor: default;
  transition: background 0.15s;
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
