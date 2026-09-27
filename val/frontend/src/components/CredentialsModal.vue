<script setup>
import { ref } from 'vue'
import { Lock } from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const emit = defineEmits(['close', 'saved'])

const username = ref('')
const password = ref('')
const loading = ref(false)
const error = ref('')

async function save() {
  error.value = ''
  if (!username.value || !password.value) {
    error.value = 'Enter your Riot username and password'
    return
  }
  loading.value = true
  try {
    const r = await fetch('/api/val/account/credentials', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: username.value, password: password.value }),
    })
    if (!r.ok) {
      const data = await r.json()
      error.value = data.detail || 'Invalid credentials'
      return
    }
    emit('saved')
  } catch {
    error.value = 'Network error'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <BaseModal width="360px" @close="emit('close')">
    <div class="modal-header">
      <Lock :size="18" color="#FF4655" style="flex-shrink:0" />
      <div>
        <p class="modal-title">Riot Credentials</p>
        <span class="modal-subtitle">Encrypted and stored on your server</span>
      </div>
    </div>

    <div class="modal-body">
      <p class="info-text">
        Your username and password will be encrypted with a server-side key (Fernet AES-128). They are never sent to third parties — only used to authenticate with Riot's servers to access your store and inventory.
      </p>

      <div class="form-group">
        <label class="form-label">Riot username</label>
        <input v-model="username" class="modal-input" placeholder="username" autocomplete="username" />
      </div>
      <div class="form-group">
        <label class="form-label">Password</label>
        <input v-model="password" type="password" class="modal-input" placeholder="••••••••" autocomplete="current-password" @keydown.enter="save" />
      </div>

      <p v-if="error" class="form-error">{{ error }}</p>
    </div>

    <div class="modal-footer">
      <button class="btn-cancel" @click="emit('close')">Cancel</button>
      <button class="btn-primary" :disabled="loading || !username || !password" @click="save">
        {{ loading ? 'Verifying…' : 'Save' }}
      </button>
    </div>
  </BaseModal>
</template>

<style scoped>
.info-text {
  font-size: 0.75rem;
  color: #636366;
  line-height: 1.5;
  margin-bottom: 14px;
  padding: 10px 12px;
  background: rgba(255, 255, 255, 0.04);
  border-radius: 8px;
  border: 0.5px solid rgba(255, 255, 255, 0.06);
}

.form-group { margin-bottom: 10px; }

.form-label {
  display: block;
  font-size: 0.75rem;
  font-weight: 600;
  color: #8E8E93;
  margin-bottom: 5px;
  letter-spacing: 0.02em;
}

.form-error {
  font-size: 0.75rem;
  color: #ff453a;
  margin-top: 8px;
}
</style>
