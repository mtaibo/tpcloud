<script setup>
import { ref } from 'vue'
import { Lock, ShieldCheck, Cookie } from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const emit = defineEmits(['close', 'saved'])

const step = ref(1) // 1 = credentials, 2 = MFA code, 3 = SSID cookie
const username = ref('')
const password = ref('')
const mfaCode = ref('')
const tokenValue = ref('')
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
    const data = await r.json()
    if (!r.ok) {
      if (r.status === 401) {
        // Server IP blocked by Riot — suggest SSID method
        step.value = 3
        error.value = ''
        return
      }
      error.value = data.detail || 'Invalid credentials'
      return
    }
    if (data.requires_mfa) {
      step.value = 2
      return
    }
    emit('saved')
  } catch {
    error.value = 'Network error — check server logs'
  } finally {
    loading.value = false
  }
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

async function submitToken() {
  error.value = ''
  if (!tokenValue.value.trim()) {
    error.value = 'Paste the token value'
    return
  }
  loading.value = true
  try {
    const r = await fetch('/api/val/account/credentials/token', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token: tokenValue.value.trim() }),
    })
    const data = await r.json()
    if (!r.ok) {
      error.value = data.detail || 'Invalid or expired token'
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
  tokenValue.value = ''
  error.value = ''
}

function goToSsid() {
  step.value = 3
  error.value = ''
}
</script>

<template>
  <BaseModal width="400px" @close="emit('close')">

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

        <button class="btn-ssid-link" @click="goToSsid">Use cookie auth instead →</button>
      </div>

      <div class="modal-footer">
        <button class="btn-cancel" @click="emit('close')">Cancel</button>
        <button class="btn-primary" :disabled="loading || !username || !password" @click="save">
          {{ loading ? 'Verifying…' : 'Continue' }}
        </button>
      </div>
    </template>

    <!-- Step 2: MFA code -->
    <template v-else-if="step === 2">
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

    <!-- Step 3: Access token -->
    <template v-else>
      <div class="modal-header">
        <Cookie :size="18" color="#FF4655" style="flex-shrink:0" />
        <div>
          <p class="modal-title">Token Authentication</p>
          <span class="modal-subtitle">Copy your access token from the browser</span>
        </div>
      </div>

      <div class="modal-body">
        <div class="info-text">
          <p style="margin-bottom:10px">Server-side logins are blocked by Riot. Use your browser's access token instead.</p>
          <p class="step-label">Steps:</p>
          <ol class="steps-list">
            <li>
              Go to
              <a href="https://account.riotgames.com" target="_blank" rel="noopener" class="ext-link">account.riotgames.com</a>
              and log in if needed
            </li>
            <li>Press <strong>F12</strong> → <strong>Application</strong> tab</li>
            <li>In the sidebar: <strong>Cookies</strong> → click <code>account.riotgames.com</code></li>
            <li>Find <code>__Secure-access_token</code> and copy its <strong>Value</strong></li>
          </ol>
          <p class="note">The token starts with <code>eyJ…</code> and is valid for ~1 hour. You'll need to repeat this when it expires.</p>
        </div>

        <div class="form-group">
          <label class="form-label">__Secure-access_token value</label>
          <input
            v-model="tokenValue"
            class="modal-input"
            placeholder="eyJ…"
            autocomplete="off"
            spellcheck="false"
            @keydown.enter="submitToken"
          />
        </div>

        <p v-if="error" class="form-error">{{ error }}</p>
      </div>

      <div class="modal-footer">
        <button class="btn-cancel" @click="backToCredentials">Back</button>
        <button class="btn-primary" :disabled="loading || !tokenValue.trim()" @click="submitToken">
          {{ loading ? 'Verifying…' : 'Save' }}
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

.step-label {
  font-weight: 600;
  color: #8e8e93;
  margin-bottom: 6px;
}

.steps-list {
  margin: 0;
  padding-left: 18px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.steps-list li { color: #8e8e93; }

.steps-list strong { color: #aeaeb2; }

.ext-link { color: #FF4655; text-decoration: underline; text-underline-offset: 2px; }

.note {
  margin-top: 10px;
  font-size: 0.68rem;
  color: #48484a;
  font-style: italic;
  line-height: 1.4;
}

code {
  font-family: ui-monospace, monospace;
  background: rgba(255,255,255,0.07);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 0.7rem;
  color: #FF4655;
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

.btn-ssid-link {
  background: none;
  border: none;
  color: #636366;
  font-size: 0.72rem;
  cursor: pointer;
  padding: 0;
  margin-top: 6px;
  text-decoration: underline;
  text-underline-offset: 2px;
}

.btn-ssid-link:hover { color: #8e8e93; }
</style>
