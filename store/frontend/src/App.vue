<script setup>
import { ref, computed, onMounted } from 'vue'
import Sidebar from './components/Sidebar.vue'
import FileBrowser from './components/FileBrowser.vue'
import SharesPanel from './components/SharesPanel.vue'
import MobileBottomNav from './components/MobileBottomNav.vue'
import MobileBrowse from './components/MobileBrowse.vue'
import FileViewer from './components/FileViewer.vue'

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

function openShares() {
  showShares.value = true
}

function navigate(loc, path) {
  showShares.value = false
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

const fileViewerToken = ref(null)
const fileViewerInfo = ref(null)

const showShares = ref(false)

const mobileTab = ref('personal')

const activeTab = computed(() => {
  if (mobileTab.value === 'browse') return 'browse'
  if (location.value === 'external' && (currentPath.value === 'shared' || currentPath.value.startsWith('shared/'))) return 'shared'
  if (location.value === 'external' && user.value && currentPath.value.startsWith(`users/${user.value.email}`)) return 'personal'
  return mobileTab.value
})

function onMobileTabChange(tab) {
  mobileTab.value = tab
  if (tab === 'shared') navigate('external', 'shared')
  if (tab === 'personal') navigate('external', `users/${user.value.email}`)
}

function onMobileNavigate(loc, path) {
  navigate(loc, path)
  mobileTab.value = loc === 'external' && path.startsWith('shared') ? 'shared' : 'personal'
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
    } else {
      window.location.href = `${LOGIN_URL}/?redirect=store.migueltaibo.com`
      return
    }
  } catch {
    window.location.href = `${LOGIN_URL}/?redirect=store.migueltaibo.com`
    return
  }

  const pathParts = window.location.pathname.replace(/^\//, '').split('/')
  const potentialToken = (pathParts.length === 1 && pathParts[0]) ? pathParts[0] : null

  if (potentialToken) {
    const infoRes = await fetch(`/api/files/open/${potentialToken}/info`)
    if (infoRes.ok) {
      fileViewerToken.value = potentialToken
      fileViewerInfo.value = await infoRes.json()
      appReady.value = true
      return
    }
  }

  const initialPath = `users/${user.value.email}`
  location.value = 'external'
  currentPath.value = initialPath
  navHistory.value = [{ location: 'external', path: initialPath }]
  navIndex.value = 0
  appReady.value = true
})
</script>

<template>
  <FileViewer
    v-if="appReady && fileViewerToken && fileViewerInfo"
    :filename="fileViewerInfo.filename"
    :content-type="fileViewerInfo.content_type"
    :file-url="`/api/files/open/${fileViewerToken}`"
  />
  <div v-else-if="appReady && user" class="app">
    <Sidebar :user="user" :location="location" :current-path="currentPath" :view-as-admin="viewAsAdmin" @navigate="navigate" @toggle-admin-view="toggleAdminView" />
    <MobileBrowse v-if="activeTab === 'browse'" class="mobile-only" :user="user" :can-go-back="canGoBack" :can-go-forward="canGoForward" @navigate="onMobileNavigate" @go-back="goBack" @go-forward="goForward" />
    <SharesPanel v-if="showShares" :class="activeTab === 'browse' ? 'mobile-hidden' : ''" @navigate="navigate" />
    <FileBrowser
      v-else
      :class="activeTab === 'browse' ? 'mobile-hidden' : ''"
      :user="user"
      :location="location"
      :current-path="currentPath"
      :can-go-back="canGoBack"
      :can-go-forward="canGoForward"
      :view-as-admin="viewAsAdmin"
      @navigate="navigate"
      @go-back="goBack"
      @go-forward="goForward"
      @open-shares="openShares"
    />
    <MobileBottomNav :active-tab="activeTab" @tab-change="onMobileTabChange" />
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

.mobile-only { display: none; }
@media (max-width: 767px) {
  .mobile-only { display: flex; flex: 1; flex-direction: column; }
  .mobile-hidden { display: none; }
}

.loading-screen {
  height: 100dvh;
  background: #000;
  display: flex;
  align-items: center;
  justify-content: center;
}

.loading-screen span { font-size: 0.875rem; color: #525252; }
</style>
