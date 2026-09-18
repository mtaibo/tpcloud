<script setup>
import { ref, computed, onMounted } from 'vue'
import PasswordWall from './components/PasswordWall.vue'
import ShareBrowser from './components/ShareBrowser.vue'
import ShareLanding from './components/ShareLanding.vue'

const token = window.location.pathname.replace(/^\/+/, '').split('/')[0]

const shareInfo = ref(null)
const sessionToken = ref(null)
const error = ref('')
const loading = ref(true)

onMounted(async () => {
  if (!token) {
    loading.value = false
    return
  }
  sessionToken.value = localStorage.getItem(`share-session-${token}`)
  try {
    const res = await fetch(`/api/share/${token}`)
    if (res.status === 404) {
      error.value = 'This share link does not exist.'
      loading.value = false
      return
    }
    if (!res.ok) {
      error.value = 'Failed to load share'
      loading.value = false
      return
    }
    shareInfo.value = await res.json()
  } catch {
    error.value = 'Connection error'
  } finally {
    loading.value = false
  }
})

const needsPassword = computed(() => shareInfo.value?.has_password && !sessionToken.value)

function onAuthenticated(st) {
  sessionToken.value = st
}
</script>

<template>
  <ShareLanding v-if="!loading && !token" />
  <div v-else-if="loading" class="center-msg">Loading…</div>
  <div v-else-if="error" class="center-msg err">{{ error }}</div>
  <template v-else-if="shareInfo">
    <PasswordWall v-if="needsPassword" :token="token" @authenticated="onAuthenticated" />
    <ShareBrowser v-else :token="token" :share-info="shareInfo" :session-token="sessionToken" />
  </template>
</template>

<style scoped>
.center-msg {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100dvh;
  font-size: 0.9rem;
  color: #636366;
}
.err { color: #ff453a; }
</style>
