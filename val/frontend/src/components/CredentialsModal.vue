<script setup>
import { ref } from 'vue'
import { Lock, ShieldCheck, FlaskConical } from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const emit = defineEmits(['close', 'saved'])

const step = ref(1)
const username = ref('')
const password = ref('')
const mfaCode = ref('')
const loading = ref(false)
const error = ref('')
const showDiagnose = ref(false)
const diagnosing = ref(false)
const diagResults = ref(null)

async function save() {
  error.value = ''
  if (!username.value || !password.value) {
    error.value = 'Enter your Riot username and password'
    return
  }
  loading.value = true
  showDiagnose.value = false
  diagResults.value = null
  try {
    const r = await fetch('/api/val/account/credentials', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: username.value, password: password.value }),
    })
    const data = await r.json()
    if (!r.ok) {
      error.value = data.detail || 'Invalid credentials'
      showDiagnose.value = true
      return
    }
    if (data.requires_mfa) {
      step.value = 2
      return
    }
    emit('saved')
  } catch {
    error.value = 'Network error — check server logs'
    showDiagnose.value = true
  } finally {
    loading.value = false
  }
}

async function runDiagnosis() {
  if (!username.value || !password.value) return
  diagnosing.value = true
  diagResults.value = null
  try {
    const r = await fetch('/api/val/account/auth/diagnose', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: username.value, password: password.value }),
    })
    diagResults.value = await r.json()
  } catch {
    diagResults.value = [{ strategy: 'network', error: 'Failed to reach server', success: false }]
  } finally {
    diagnosing.value = false
  }
}

function diagLabel(row) {
  if (row.success) return 'SUCCESS'
  if (row.mfa) return 'MFA'
  if (row.captcha) return 'CAPTCHA'
  if (row.error && !row.type) return 'ERR'
  return (row.type || '?').toUpperCase()
}

function diagColor(row) {
  if (row.success) return '#30d158'
  if (row.mfa) return '#ffd60a'
  if (row.captcha) return '#ff9f0a'
  return '#ff453a'
}

async function submitMfa() {
  error.value = ''
  if (!mfaCode.value.trim()) {
    error.value = 'Enter the 6-digit code'
    return
  }
  loading.value = true
  try {
    const r = await fetch('/api/val/account/credentials/mfa', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code: mfaCode.value.trim() }),
    })
    const data = await r.json()
    if (!r.ok) {
      error.value = data.detail || 'Invalid MFA code'
      return
    }
    emit('saved')
  } catch {
    error.value = 'Network error'
  } finally {
    loading.value = false
  }
}

function backToCredentials() {
  step.value = 1
  mfaCode.value = ''
  error.value = ''
}
</script>

<template>
  <BaseModal width="380px" @close="emit('close')">

    <!-- Step 1: Credentials -->
    <template v-if="step === 1">
      <div class="modal-header">
        <Lock :size="18" color="#FF4655" style="flex-shrink:0" />
        <div>
          <p class="modal-title">Riot Credentials</p>
          <span class="modal-subtitle">Encrypted and stored on your server</span>
        </div>
      </div>

      <div class="modal-body">
        <p class="info-text">
          Your username and password are encrypted with a server-side key (Fernet AES-128) and never sent to third parties. They are only used to authenticate with Riot's servers to access your store and inventory.
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

        <div v-if="showDiagnose" class="diag-trigger">
          <button class="btn-diag" :disabled="diagnosing" @click="runDiagnosis">
            <FlaskConical :size="12" />
            {{ diagnosing ? 'Running…' : 'Run diagnostics' }}
          </button>
          <span class="diag-hint">Tests 4 auth strategies — check server logs too</span>
        </div>

        <div v-if="diagResults" class="diag-results">
          <p class="diag-title">Strategy results</p>
          <div v-for="row in diagResults" :key="row.strategy" class="diag-row">
            <span class="diag-strategy">{{ row.strategy }}</span>
            <span class="diag-badge" :style="{ color: diagColor(row), borderColor: diagColor(row) }">{{ diagLabel(row) }}</span>
            <span class="diag-detail">
              {{ row.captcha ? 'bot blocked (captcha)' : (row.error && !row.type) ? row.error : `init=${row.init_status} auth=${row.auth_status} cookies=[${(row.init_cookies||[]).join(',')}]` }}
            </span>
          </div>
          <p class="diag-note">
            If all show AUTH or CAPTCHA, Riot is blocking from this IP — wait 1–2h and retry.
          </p>
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn-cancel" @click="emit('close')">Cancel</button>
        <button class="btn-primary" :disabled="loading || !username || !password" @click="save">
          {{ loading ? 'Verifying…' : 'Continue' }}
        </button>
      </div>
    </template>

    <!-- Step 2: MFA code -->
    <template v-else>
      <div class="modal-header">
        <ShieldCheck :size="18" color="#FF4655" style="flex-shrink:0" />
        <div>
          <p class="modal-title">Two-Factor Authentication</p>
          <span class="modal-subtitle">Enter the code from your authenticator app</span>
        </div>
      </div>

      <div class="modal-body">
        <p class="info-text">
          Your Riot account has 2FA enabled. Enter the current 6-digit code from your authenticator app to continue.
        </p>

        <div class="form-group">
          <label class="form-label">Authentication code</label>
          <input
            v-model="mfaCode"
            class="modal-input mfa-input"
            placeholder="000000"
            maxlength="6"
            inputmode="numeric"
            pattern="[0-9]*"
            autocomplete="one-time-code"
            @keydown.enter="submitMfa"
          />
        </div>

        <p v-if="error" class="form-error">{{ error }}</p>
      </div>

      <div class="modal-footer">
        <button class="btn-cancel" @click="backToCredentials">Back</button>
        <button class="btn-primary" :disabled="loading || mfaCode.length < 6" @click="submitMfa">
          {{ loading ? 'Verifying…' : 'Verify' }}
        </button>
      </div>
    </template>

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

.mfa-input {
  letter-spacing: 0.3em;
  font-size: 1.25rem;
  text-align: center;
  font-variant-numeric: tabular-nums;
}

.form-error {
  font-size: 0.75rem;
  color: #ff453a;
  margin-top: 8px;
}

.diag-trigger {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
}

.btn-diag {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 0.7rem;
  padding: 4px 10px;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.06);
  border: 0.5px solid rgba(255, 255, 255, 0.12);
  color: #8E8E93;
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;
}
.btn-diag:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-diag:not(:disabled):hover { background: rgba(255, 255, 255, 0.1); color: #d1d1d6; }

.diag-hint {
  font-size: 0.65rem;
  color: #525252;
}

.diag-results {
  margin-top: 10px;
  padding: 10px;
  background: rgba(0, 0, 0, 0.3);
  border-radius: 8px;
  border: 0.5px solid rgba(255, 255, 255, 0.08);
}

.diag-title {
  font-size: 0.65rem;
  font-weight: 600;
  color: #636366;
  margin-bottom: 8px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.diag-row {
  display: grid;
  grid-template-columns: 108px 62px 1fr;
  align-items: center;
  gap: 6px;
  padding: 4px 0;
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.04);
}
.diag-row:last-of-type { border-bottom: none; }

.diag-strategy { font-size: 0.65rem; color: #8E8E93; font-family: monospace; }

.diag-badge {
  font-size: 0.58rem;
  font-weight: 700;
  padding: 2px 4px;
  border-radius: 4px;
  border: 0.5px solid;
  text-align: center;
  letter-spacing: 0.03em;
}

.diag-detail {
  font-size: 0.6rem;
  color: #525252;
  font-family: monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.diag-note {
  font-size: 0.65rem;
  color: #636366;
  margin-top: 8px;
  line-height: 1.4;
}
</style>
