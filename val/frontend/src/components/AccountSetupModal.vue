<script setup>
import { ref } from 'vue'
import { Shield, ChevronRight } from 'lucide-vue-next'
import CredentialsModal from './CredentialsModal.vue'

const emit = defineEmits(['linked', 'close'])

const step = ref(1) // 1 = link account, 2 = optional credentials
const riotId = ref('')
const region = ref('eu')
const loading = ref(false)
const error = ref('')
const linkedData = ref(null)
const showCredentials = ref(false)

const regions = [
  { id: 'eu', label: 'EU' },
  { id: 'na', label: 'NA' },
  { id: 'ap', label: 'AP' },
  { id: 'kr', label: 'KR' },
]

async function linkAccount() {
  error.value = ''
  const parts = riotId.value.trim().split('#')
  if (parts.length !== 2 || !parts[0] || !parts[1]) {
    error.value = 'Enter your Riot ID in the format Name#TAG'
    return
  }
  const [name, tag] = parts
  loading.value = true
  try {
    const r = await fetch('/api/val/account', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ riot_name: name, riot_tag: tag, region: region.value }),
    })
    if (!r.ok) {
      const data = await r.json()
      error.value = data.detail || 'Failed to link account'
      return
    }
    linkedData.value = await r.json()
    step.value = 2
  } catch {
    error.value = 'Network error'
  } finally {
    loading.value = false
  }
}

function skipCredentials() {
  emit('linked', linkedData.value)
}

function onCredentialsSaved() {
  showCredentials.value = false
  emit('linked', linkedData.value)
}
</script>

<template>
  <div class="setup-screen" @click.self="emit('close')">
    <div class="setup-card">
      <button class="close-btn" @click="emit('close')">✕</button>

      <!-- Step 1: Link Riot account -->
      <template v-if="step === 1">
        <div class="setup-logo">
          <svg viewBox="0 0 32 32" width="40" height="40">
            <rect width="32" height="32" rx="8" fill="#FF4655"/>
            <path d="M8 10 L16 22 L24 10" stroke="white" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
          </svg>
        </div>
        <h1 class="setup-title">Welcome to TPVal</h1>
        <p class="setup-subtitle">Link your Riot account to get started</p>

        <div class="setup-form">
          <label class="form-label">Riot ID</label>
          <input
            v-model="riotId"
            class="modal-input"
            placeholder="PlayerName#TAG"
            @keydown.enter="linkAccount"
            autocomplete="off"
            spellcheck="false"
          />

          <label class="form-label" style="margin-top: 12px;">Region</label>
          <div class="region-pills">
            <button
              v-for="r in regions"
              :key="r.id"
              :class="['region-pill', { active: region === r.id }]"
              @click="region = r.id"
            >{{ r.label }}</button>
          </div>

          <p v-if="error" class="form-error">{{ error }}</p>

          <button class="btn-primary btn-full" :disabled="loading || !riotId.trim()" @click="linkAccount" style="margin-top: 16px;">
            {{ loading ? 'Verifying…' : 'Link Account' }}
          </button>
        </div>
      </template>

      <!-- Step 2: Optional credentials for shop -->
      <template v-else-if="step === 2">
        <div class="setup-logo">
          <Shield :size="36" color="#FF4655" />
        </div>
        <h1 class="setup-title">Enable Daily Shop</h1>
        <p class="setup-subtitle">
          To view your personal store and inventory, enter your Riot credentials.<br/>
          They're encrypted and stored only on your private server.
        </p>

        <div class="setup-actions">
          <button class="btn-primary btn-full" @click="showCredentials = true">
            Connect Riot Account
            <ChevronRight :size="14" style="margin-left: 4px;" />
          </button>
          <button class="btn-cancel btn-full" @click="skipCredentials" style="margin-top: 8px;">
            Skip for now
          </button>
        </div>
      </template>

    </div>

    <CredentialsModal
      v-if="showCredentials"
      @close="showCredentials = false"
      @saved="onCredentialsSaved"
    />
  </div>
</template>

<style scoped>
.setup-screen {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.85);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9000;
}

.close-btn {
  position: absolute;
  top: 12px;
  right: 12px;
  background: none;
  border: none;
  color: #636366;
  font-size: 1rem;
  cursor: default;
  padding: 4px 8px;
  border-radius: 6px;
  transition: color 0.15s, background 0.15s;
}
.close-btn:hover { color: #fff; background: rgba(255, 255, 255, 0.06); }

.setup-card {
  position: relative;
  width: 100%;
  max-width: 380px;
  padding: 40px 32px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.setup-logo { margin-bottom: 8px; }

.setup-title {
  font-size: 1.375rem;
  font-weight: 700;
  color: #fff;
  letter-spacing: -0.02em;
  text-align: center;
}

.setup-subtitle {
  font-size: 0.875rem;
  color: #636366;
  text-align: center;
  line-height: 1.5;
  margin-top: 4px;
  margin-bottom: 8px;
}

.setup-form {
  width: 100%;
  display: flex;
  flex-direction: column;
  margin-top: 8px;
}

.form-label {
  font-size: 0.75rem;
  font-weight: 600;
  color: #8E8E93;
  margin-bottom: 6px;
  letter-spacing: 0.02em;
}

.region-pills {
  display: flex;
  gap: 6px;
}

.region-pill {
  flex: 1;
  padding: 6px 0;
  border-radius: 8px;
  font-size: 0.8rem;
  font-weight: 600;
  background: rgba(255, 255, 255, 0.06);
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  color: #8E8E93;
  cursor: default;
  transition: background 0.12s, color 0.12s, border-color 0.12s;
}

.region-pill.active {
  background: rgba(255, 70, 85, 0.15);
  border-color: rgba(255, 70, 85, 0.4);
  color: #FF4655;
}

.form-error {
  font-size: 0.75rem;
  color: #ff453a;
  margin-top: 8px;
}

.btn-full { width: 100%; text-align: center; justify-content: center; display: flex; align-items: center; }

.setup-actions { width: 100%; display: flex; flex-direction: column; margin-top: 8px; }
</style>
