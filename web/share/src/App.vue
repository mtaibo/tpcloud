<script setup>
import { ref, computed, onMounted } from 'vue'
import PasswordWall from './components/PasswordWall.vue'
import ShareBrowser from './components/ShareBrowser.vue'
import ShareLanding from './components/ShareLanding.vue'
import FileViewer from './components/FileViewer.vue'
import ContextMenu from './components/ContextMenu.vue'
import Spinner from './ui/Spinner.vue'
import FileIcon from './components/FileIcon.vue'

const token = window.location.pathname.replace(/^\/share\/?/, '').split('/')[0]

const shareInfo = ref(null)
const sessionToken = ref(null)
const fileInfo = ref(null)
const error = ref('')
const loading = ref(true)

onMounted(async () => {
  if (!token) {
    loading.value = false
    return
  }
  sessionToken.value = localStorage.getItem(`share-session-${token}`)
  try {
    const res = await fetch(`/share/api/share/${token}`)
    if (res.status === 404) {
      try {
        const openRes = await fetch(`/share/api/open/${token}/info`)
        if (openRes.ok) {
          const ct = openRes.headers.get('content-type') || ''
          if (ct.includes('application/json')) {
            fileInfo.value = await openRes.json()
            loading.value = false
            return
          }
        }
      } catch {}
      error.value = 'Este enlace no existe o ya no está compartido.'
      loading.value = false
      return
    }
    if (!res.ok) {
      error.value = 'No se ha podido abrir el enlace.'
      loading.value = false
      return
    }
    shareInfo.value = await res.json()
  } catch {
    error.value = 'No hay conexión. Vuelve a intentarlo.'
  } finally {
    loading.value = false
  }
})

const needsPassword = computed(() => shareInfo.value?.has_password && !sessionToken.value)

const shareFileUrl = computed(() => {
  const params = new URLSearchParams()
  if (sessionToken.value) params.set('session', sessionToken.value)
  const qs = params.toString()
  return `/share/api/share/${token}/files/view${qs ? `?${qs}` : ''}`
})

function onAuthenticated(st) {
  sessionToken.value = st
}
</script>

<template>
  <FileViewer
    v-if="!loading && fileInfo"
    :filename="fileInfo.filename"
    :content-type="fileInfo.content_type"
    :file-url="`/share/api/open/${token}`"
  />
  <ShareLanding v-else-if="!loading && !token" />
  <div v-else-if="loading" class="center"><Spinner :size="26" /></div>
  <div v-else-if="error" class="center">
    <FileIcon :entry="{ type: 'share-link', name: '' }" :size="84" class="faded" />
    <h1 class="title-2">Enlace no disponible</h1>
    <p class="subheadline secondary">{{ error }}</p>
  </div>
  <template v-else-if="shareInfo">
    <PasswordWall v-if="needsPassword" :token="token" :label="shareInfo.label" @authenticated="onAuthenticated" />
    <FileViewer
      v-else-if="shareInfo.is_file"
      :filename="shareInfo.label"
      :content-type="shareInfo.content_type"
      :file-url="shareFileUrl"
    />
    <ShareBrowser v-else :token="token" :share-info="shareInfo" :session-token="sessionToken" />
  </template>
  <ContextMenu />
</template>

<style>
html, body, #app { height: 100%; }
.photo, .folder-tile, .file-row, .tile { -webkit-user-select: none; user-select: none; }
</style>

<style scoped>
.center { min-height: 100dvh; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; padding: 24px; text-align: center; }
.faded { opacity: 0.5; margin-bottom: 8px; }
</style>
