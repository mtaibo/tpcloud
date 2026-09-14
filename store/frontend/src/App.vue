<script setup>
import { ref, computed, onMounted } from 'vue'
import { ChevronLeft } from 'lucide-vue-next'
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

const canGoBack = computed(() => !!currentPath.value)

function goBack() {
  if (!currentPath.value) return
  const parts = currentPath.value.split('/').filter(Boolean)
  parts.pop()
  currentPath.value = parts.join('/')
}
</script>

<template>
  <div v-if="appReady && user" class="app">
    <button
      class="back-btn"
      :class="{ visible: canGoBack }"
      :disabled="!canGoBack"
      @click="goBack"
      title="Go back"
    >
      <ChevronLeft class="back-icon" />
    </button>
    <Sidebar :user="user" :location="location" :current-path="currentPath" @navigate="navigate" />
    <FileBrowser :user="user" :location="location" :current-path="currentPath" @navigate="navigate" />
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

.back-btn {
  position: fixed;
  top: 12px;
  left: 12px;
  z-index: 500;
  width: 34px;
  height: 34px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  cursor: pointer;
  background: linear-gradient(180deg,
    rgba(255, 255, 255, 0.16) 0%,
    rgba(255, 255, 255, 0.06) 100%
  );
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  box-shadow:
    0 0 0 0.5px rgba(255, 255, 255, 0.2),
    inset 0 1px 0 rgba(255, 255, 255, 0.24),
    inset 0 -0.5px 0 rgba(0, 0, 0, 0.28),
    0 4px 14px rgba(0, 0, 0, 0.32),
    0 1px 4px rgba(0, 0, 0, 0.2);
  color: rgba(255, 255, 255, 0.75);
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.2s, background 0.18s, box-shadow 0.18s, color 0.18s, transform 0.18s;
}

.back-btn.visible {
  opacity: 1;
  pointer-events: auto;
}

.back-btn:hover {
  background: linear-gradient(180deg,
    rgba(255, 255, 255, 0.22) 0%,
    rgba(255, 255, 255, 0.1) 100%
  );
  box-shadow:
    0 0 0 0.5px rgba(255, 255, 255, 0.26),
    inset 0 1px 0 rgba(255, 255, 255, 0.3),
    inset 0 -0.5px 0 rgba(0, 0, 0, 0.28),
    0 6px 20px rgba(0, 0, 0, 0.38),
    0 2px 6px rgba(0, 0, 0, 0.24);
  color: #fff;
  transform: translateY(-0.5px);
}

.back-btn:active {
  transform: translateY(0) scale(0.96);
}

.back-icon {
  width: 15px;
  height: 15px;
  stroke-width: 2.5;
  margin-right: -1px;
}
</style>
