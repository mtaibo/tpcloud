<script setup>
import { ref } from 'vue'
import { Lock } from 'lucide-vue-next'

const props = defineProps({ token: String })
const emit = defineEmits(['authenticated'])

const password = ref('')
const error = ref('')
const loading = ref(false)

async function submit() {
  if (!password.value) return
  loading.value = true
  error.value = ''
  try {
    const res = await fetch(`/api/share/${props.token}/auth`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ password: password.value }),
    })
    if (!res.ok) {
      error.value = 'Incorrect password'
      return
    }
    const { session_token } = await res.json()
    sessionStorage.setItem(`share-session-${props.token}`, session_token)
    emit('authenticated', session_token)
  } catch {
    error.value = 'Connection error'
  } finally {
    loading.value = false
  }
}

function onKeydown(e) {
  if (e.key === 'Enter') submit()
}
</script>

<template>
  <div class="wall">
    <div class="card">
      <div class="icon-wrap">
        <Lock class="lock-icon" />
      </div>
      <h1 class="title">Password required</h1>
      <p class="subtitle">This shared folder is password protected.</p>

      <input
        v-model="password"
        class="pw-input"
        type="password"
        placeholder="Enter password"
        autocomplete="current-password"
        @keydown="onKeydown"
      />

      <p v-if="error" class="error-msg">{{ error }}</p>

      <button class="btn-unlock" :disabled="loading || !password" @click="submit">
        {{ loading ? 'Verifying…' : 'Unlock' }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.wall {
  min-height: 100dvh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  background: #000;
}

.card {
  width: 100%;
  max-width: 320px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
  padding: 2rem;
  background: rgba(28, 28, 30, 0.97);
  border: 0.5px solid rgba(255, 255, 255, 0.12);
  border-radius: 18px;
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.6), inset 0 0.5px 0 rgba(255, 255, 255, 0.08);
}

.icon-wrap {
  width: 52px;
  height: 52px;
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.07);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 0.25rem;
}

.lock-icon { width: 24px; height: 24px; color: #007AFF; }

.title {
  font-size: 1rem;
  font-weight: 600;
  color: #fff;
  letter-spacing: -0.01em;
  text-align: center;
}

.subtitle {
  font-size: 0.8rem;
  color: #636366;
  text-align: center;
  line-height: 1.4;
}

.pw-input {
  width: 100%;
  padding: 10px 14px;
  background: rgba(255, 255, 255, 0.06);
  border: 0.5px solid rgba(255, 255, 255, 0.12);
  border-radius: 10px;
  color: #fff;
  font-size: 0.9rem;
  font-family: inherit;
  outline: none;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.pw-input::placeholder { color: #3a3a3c; }

.pw-input:focus {
  border-color: #007AFF;
  box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.2);
}

.error-msg {
  font-size: 0.8rem;
  color: #ff453a;
  text-align: center;
}

.btn-unlock {
  width: 100%;
  padding: 10px;
  border-radius: 10px;
  font-size: 0.9rem;
  font-weight: 600;
  color: #fff;
  background: #007AFF;
  border: none;
  font-family: inherit;
  transition: opacity 0.12s;
}

.btn-unlock:disabled { opacity: 0.4; cursor: default; }
.btn-unlock:not(:disabled):hover { opacity: 0.88; }
</style>
