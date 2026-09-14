<script setup>
import { ref, computed, onMounted } from 'vue'
import Sidebar from './components/Sidebar.vue'
import FileBrowser from './components/FileBrowser.vue'

const LOGIN_URL = 'https://login.migueltaibo.com'

const user = ref(null)
const appReady = ref(false)
const location = ref('external')
const currentPath = ref('')
const viewAsAdmin = ref(false)

const navHistory = ref([])
const navIndex = ref(-1)

const canGoBack = computed(() => navIndex.value > 0)
const canGoForward = computed(() => navIndex.value < navHistory.value.length - 1)

function _applyState(index) {
  const s = navHistory.value[index]
  location.value = s.location
  currentPath.value = s.path
  navIndex.value = index
}

function navigate(loc, path) {
  const current = navHistory.value[navIndex.value]
  if (current && current.location === loc && current.path === path) return

  // Discard any forward history
  const newHistory = navHistory.value.slice(0, navIndex.value + 1)
  newHistory.push({ location: loc, path })
  // Cap at 10
  if (newHistory.length > 10) newHistory.shift()

  navHistory.value = newHistory
  navIndex.value = newHistory.length - 1
  location.value = loc
  currentPath.value = path
}

function goBack() {
  if (canGoBack.value) _applyState(navIndex.value - 1)
}

function goForward() {
  if (canGoForward.value) _applyState(navIndex.value + 1)
}

function toggleAdminView() {
  viewAsAdmin.value = !viewAsAdmin.value
  if (!viewAsAdmin.value) {
    const isAdminOnly = location.value === 'server' ||
      (location.value === 'external' && currentPath.value === '')
    if (isAdminOnly) navigate('external', `users/${user.value.email}`)
  }
}

onMounted(async () => {
  try {
    const res = await fetch('/auth/passkey/me')
    if (res.ok) {
      user.value = await res.json()
      const initialPath = `users/${user.value.email}`
      location.value = 'external'
      currentPath.value = initialPath
      navHistory.value = [{ location: 'external', path: initialPath }]
      navIndex.value = 0
    } else {
      window.location.href = `${LOGIN_URL}/?redirect=store.migueltaibo.com`
      return
    }
  } catch {
    window.location.href = `${LOGIN_URL}/?redirect=store.migueltaibo.com`
    return
  }
  appReady.value = true
})
</script>

<template>
  <div v-if="appReady && user" class="app">
    <Sidebar :user="user" :location="location" :current-path="currentPath" :view-as-admin="viewAsAdmin" @navigate="navigate" @toggle-admin-view="toggleAdminView" />
    <FileBrowser
      :user="user"
      :location="location"
      :current-path="currentPath"
      :can-go-back="canGoBack"
      :can-go-forward="canGoForward"
      @navigate="navigate"
      @go-back="goBack"
      @go-forward="goForward"
    />
  </div>

  <div v-else class="loading-screen">
    <span>Loading…</span>
  </div>
</template>

<style>
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; user-select: none; -webkit-user-select: none; }
html, body, #app { height: 100%; background: #000; color: #fff; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif; -webkit-font-smoothing: antialiased; }
::-webkit-scrollbar { display: none; }
* { -ms-overflow-style: none; scrollbar-width: none; }
input, textarea, [contenteditable] { user-select: text; -webkit-user-select: text; }
</style>

<style scoped>
.app { display: flex; height: 100dvh; background: #000; overflow: hidden; }

.loading-screen {
  height: 100dvh;
  background: #000;
  display: flex;
  align-items: center;
  justify-content: center;
}

.loading-screen span { font-size: 0.875rem; color: #525252; }
</style>
