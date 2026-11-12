<script setup>
import { ref } from 'vue'
import { Lock } from 'lucide-vue-next'
import Spinner from '../ui/Spinner.vue'

const props = defineProps({ token: String, label: String })
const emit = defineEmits(['authenticated'])

const password = ref('')
const error = ref('')
const loading = ref(false)

async function submit() {
  if (!password.value) return
  loading.value = true
  error.value = ''
  try {
    const res = await fetch(`/share/api/share/${props.token}/auth`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ password: password.value }),
    })
    if (!res.ok) {
      error.value = 'Incorrect password'
      return
    }
    const { session_token } = await res.json()
    localStorage.setItem(`share-session-${props.token}`, session_token)
    emit('authenticated', session_token)
  } catch {
    error.value = 'No connection. Try again.'
  } finally {
    loading.value = false
  }
}

function onKeydown(e) {
  if (e.key === 'Enter') submit()
}
</script>

<template>
  <main class="wall">
    <form class="content" :class="{ shake: !!error }" @submit.prevent="submit">
      <span class="lock"><Lock :size="34" :stroke-width="2" /></span>
      <h1 class="title-1">{{ label || 'Protected Folder' }}</h1>
      <p class="subheadline secondary">Whoever shared this protected it with a password. Enter it to continue.</p>
      <div class="card field">
        <input
          v-model="password"
          class="row-input"
          type="password"
          placeholder="Password"
          autocomplete="current-password"
          autofocus
          @input="error = ''"
        />
      </div>
      <p v-if="error" class="footnote danger">{{ error }}</p>
      <button type="submit" class="btn btn-prominent block" :disabled="loading || !password">
        <Spinner v-if="loading" :size="18" style="color: #fff" />
        <span v-else>Continue</span>
      </button>
    </form>
  </main>
</template>

<style scoped>
.wall { min-height: 100dvh; display: grid; place-items: center; padding: 24px 20px; }
.content { width: 100%; max-width: 380px; display: flex; flex-direction: column; align-items: center; gap: 12px; text-align: center; }
.lock { width: 84px; height: 84px; display: grid; place-items: center; border-radius: 50%; color: #fff; background: linear-gradient(180deg, #8e8e93, #636366); margin-bottom: 8px; }
.field { width: 100%; padding: 6px 20px; margin-top: 14px; }
.field input { width: 100%; border: none; outline: none; background: none; font-size: 17px; padding: 10px 0; color: var(--label); }
.content .btn { margin-top: 6px; }
</style>
