<script setup>
import { ref, onMounted } from 'vue'
import Sidebar from './components/Sidebar.vue'
import FileBrowser from './components/FileBrowser.vue'

const LOGIN_URL = 'https://login.migueltaibo.com'

const user = ref(null)
const appReady = ref(false)
const location = ref('external')
const currentPath = ref('')

onMounted(async () => {
  try {
    const res = await fetch('/auth/passkey/me')
    if (res.ok) {
      user.value = await res.json()
      currentPath.value = `users/${user.value.email}`
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

function navigate(loc, path) {
  location.value = loc
  currentPath.value = path
}

async function logout() {
  await fetch('/auth/passkey/logout', { method: 'POST' })
  window.location.href = `${LOGIN_URL}/`
}
</script>

<template>
  <div v-if="appReady && user" class="app">
    <header class="app-header">
      <span class="brand">TPStore</span>
      <div class="header-right">
        <span class="user-name">{{ user.display_name }}</span>
        <button class="logout-btn" @click="logout">Log out</button>
      </div>
    </header>
    <div class="app-body">
      <Sidebar :user="user" :location="location" :current-path="currentPath" @navigate="navigate" />
      <FileBrowser :user="user" :location="location" :current-path="currentPath" @navigate="navigate" />
    </div>
  </div>

  <div v-else class="loading-screen">
    <span>Loading…</span>
  </div>
</template>

<style>
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
html, body, #app { height: 100%; background: #000; color: #fff; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif; -webkit-font-smoothing: antialiased; }
::-webkit-scrollbar { display: none; }
* { -ms-overflow-style: none; scrollbar-width: none; }
</style>

<style scoped>
.app { display: flex; flex-direction: column; height: 100dvh; background: #000; overflow: hidden; }

.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.7rem 1.25rem;
  flex-shrink: 0;
  background: rgba(255, 255, 255, 0.055);
  backdrop-filter: blur(24px) saturate(160%);
  -webkit-backdrop-filter: blur(24px) saturate(160%);
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.12);
}

.brand {
  color: #fff;
  font-weight: 600;
  font-size: 0.875rem;
  letter-spacing: 0.03em;
  user-select: none;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 1.25rem;
}

.user-name {
  font-size: 0.8rem;
  color: #525252;
}

.logout-btn {
  background: none;
  border: none;
  font-size: 0.8rem;
  color: #525252;
  cursor: pointer;
  transition: color 0.2s;
}

.logout-btn:hover { color: #fff; }

.app-body { display: flex; flex: 1; overflow: hidden; }

.loading-screen {
  height: 100dvh;
  background: #000;
  display: flex;
  align-items: center;
  justify-content: center;
}

.loading-screen span { font-size: 0.875rem; color: #525252; }
</style>
