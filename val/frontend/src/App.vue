<script setup>
import { ref, computed, onMounted } from 'vue'
import Sidebar from './components/Sidebar.vue'
import AccountSetupModal from './components/AccountSetupModal.vue'
import DashboardView from './views/DashboardView.vue'
import StatsView from './views/StatsView.vue'
import ShopView from './views/ShopView.vue'
import InventoryView from './views/InventoryView.vue'
import EsportsView from './views/EsportsView.vue'

const LOGIN_URL = 'https://login.migueltaibo.com'

const user = ref(null)
const appReady = ref(false)
const linkedAccount = ref(null)
const currentView = ref('dashboard')

const viewComponent = computed(() => {
  switch (currentView.value) {
    case 'stats': return StatsView
    case 'shop': return ShopView
    case 'inventory': return InventoryView
    case 'esports': return EsportsView
    default: return DashboardView
  }
})

async function loadLinkedAccount() {
  try {
    const r = await fetch('/api/val/account')
    if (r.ok) {
      linkedAccount.value = await r.json()
    }
  } catch {
    // ignore
  }
}

function onAccountLinked(acc) {
  linkedAccount.value = acc
}

onMounted(async () => {
  try {
    const res = await fetch('/auth/passkey/me')
    if (res.ok) {
      user.value = await res.json()
    } else {
      window.location.href = `${LOGIN_URL}/?redirect=val.migueltaibo.com`
      return
    }
  } catch {
    window.location.href = `${LOGIN_URL}/?redirect=val.migueltaibo.com`
    return
  }

  await loadLinkedAccount()
  appReady.value = true
})
</script>

<template>
  <AccountSetupModal
    v-if="appReady && user && !linkedAccount"
    @linked="onAccountLinked"
  />

  <div v-else-if="appReady && user && linkedAccount" class="app">
    <Sidebar
      :user="user"
      :current-view="currentView"
      :linked-account="linkedAccount"
      @navigate="currentView = $event"
    />
    <main class="main-content">
      <component :is="viewComponent" :linked-account="linkedAccount" :user="user" />
    </main>
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

.main-content {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
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
