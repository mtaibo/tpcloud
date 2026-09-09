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
</script>

<template>
  <div v-if="appReady && user" class="app">
    <Sidebar :user="user" :location="location" :current-path="currentPath" @navigate="navigate" />
    <FileBrowser :user="user" :location="location" :current-path="currentPath" @navigate="navigate" />
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
