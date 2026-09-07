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
  <div v-if="appReady && user" class="flex flex-col h-screen bg-black overflow-hidden">
    <!-- Header -->
    <header
      class="flex items-center justify-between px-5 py-3 shrink-0"
      style="background: #1c1c1e; border-bottom: 1px solid rgba(255,255,255,0.08);"
    >
      <span class="text-white font-semibold text-sm tracking-wide select-none">TPStore</span>
      <div class="flex items-center gap-4">
        <span class="text-sm" style="color: #636366;">{{ user.display_name }}</span>
        <button
          @click="logout"
          class="text-sm transition-colors"
          style="color: #636366;"
          onmouseover="this.style.color='#fff'"
          onmouseout="this.style.color='#636366'"
        >
          Salir
        </button>
      </div>
    </header>

    <!-- Body -->
    <div class="flex flex-1 overflow-hidden">
      <Sidebar
        :user="user"
        :location="location"
        :current-path="currentPath"
        @navigate="navigate"
      />
      <FileBrowser
        :user="user"
        :location="location"
        :current-path="currentPath"
        @navigate="navigate"
      />
    </div>
  </div>

  <!-- Loading / redirect -->
  <div v-else class="h-screen bg-black flex items-center justify-center">
    <span class="text-sm" style="color: #636366;">Cargando...</span>
  </div>
</template>
