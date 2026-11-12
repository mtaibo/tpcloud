<script setup>
import { onMounted, ref } from 'vue'
import AlertHost from './ui/AlertHost.vue'
import Sheet from './ui/Sheet.vue'
import Spinner from './ui/Spinner.vue'
import Sidebar from './components/Sidebar.vue'
import FileBrowser from './components/FileBrowser.vue'
import SharesPanel from './components/SharesPanel.vue'
import MobileBottomNav from './components/MobileBottomNav.vue'
import MobileBrowse from './components/MobileBrowse.vue'
import FileViewer from './components/FileViewer.vue'
import ContextMenu from './components/ContextMenu.vue'
import TransferPanel from './components/TransferPanel.vue'
import BackupSheet from './components/BackupSheet.vue'
import { loadDisk, navigate, store } from './store.js'

const ready = ref(false)
const viewer = ref(null) // { token, info } for /store/<token>, a file opened in its own tab
const transfersOpen = ref(false)
const backupOpen = ref(false)

onMounted(async () => {
  try {
    const res = await fetch('/auth/passkey/me')
    if (!res.ok) throw new Error()
    store.user = await res.json()
  } catch {
    window.location.href = `/auth/signin?next=${encodeURIComponent(window.location.pathname)}`
    return
  }

  const parts = window.location.pathname.replace(/^\/store\/?/, '').split('/')
  // The gallery used to live here; it's its own section now.
  if (parts[0] === 'gallery') {
    window.location.replace('/photos/')
    return
  }
  const token = parts.length === 1 && parts[0] ? parts[0] : null
  if (token) {
    const info = await fetch(`/store/api/files/open/${token}/info`)
    if (info.ok) {
      viewer.value = { token, info: await info.json() }
      ready.value = true
      return
    }
  }

  navigate('external', `users/${store.user.email}`)
  loadDisk()
  ready.value = true
})
</script>

<template>
  <FileViewer
    v-if="ready && viewer"
    :filename="viewer.info.filename"
    :content-type="viewer.info.content_type"
    :file-url="`/store/api/files/open/${viewer.token}`"
  />

  <div v-else-if="ready" class="app">
    <Sidebar class="desktop-only" />
    <main class="main">
      <MobileBrowse v-if="store.panel === 'browse'" class="phone-only" @backup="backupOpen = true" />
      <SharesPanel v-else-if="store.panel === 'shares'" />
      <FileBrowser v-else />
    </main>
    <MobileBottomNav @transfers="transfersOpen = true" />
    <Sheet v-model:open="transfersOpen" title="Transferencias">
      <TransferPanel inline />
    </Sheet>
    <BackupSheet v-if="backupOpen" @close="backupOpen = false" />
  </div>

  <div v-else class="splash"><Spinner :size="26" /></div>

  <ContextMenu />
  <AlertHost />
</template>

<style>
html, body, #app { height: 100%; }
body { overflow: hidden; }
* { user-select: none; -webkit-user-select: none; -webkit-touch-callout: none; }
input, textarea, [contenteditable], .mono { user-select: text; -webkit-user-select: text; }
</style>

<style scoped>
.app { display: flex; height: 100dvh; background: var(--bg); }
.main { flex: 1; min-width: 0; display: flex; }
.splash { height: 100dvh; display: grid; place-items: center; }
.phone-only { display: none; }
@media (max-width: 767px) {
  .desktop-only { display: none; }
  .phone-only { display: block; }
}
@media (min-width: 768px) {
  .main :deep(.browse) { display: none; }
}
</style>
