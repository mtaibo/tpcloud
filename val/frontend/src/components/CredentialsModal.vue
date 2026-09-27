<script setup>
import { ref } from 'vue'
import { Lock, ShieldCheck, Cookie } from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const emit = defineEmits(['close', 'saved'])

const step = ref(1) // 1 = credentials, 2 = MFA code, 3 = SSID cookie
const username = ref('')
const password = ref('')
const mfaCode = ref('')
const ssid = ref('')
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

async function submitSsid() {
  error.value = ''
  if (!ssid.value.trim()) {
    error.value = 'Paste the ssid cookie value'
    return
  }
  loading.value = true
  try {
    const r = await fetch('/api/val/account/credentials/ssid', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ssid: ssid.value.trim() }),
    })
    const data = await r.json()
    if (!r.ok) {
      error.value = data.detail || 'Invalid or expired SSID cookie'
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
  ssid.value = ''
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

    <!-- Step 3: SSID cookie -->
    <template v-else>
      <div class="modal-header">
        <Cookie :size="18" color="#FF4655" style="flex-shrink:0" />
        <div>
          <p class="modal-title">Cookie Authentication</p>
          <span class="modal-subtitle">Works reliably from any server</span>
        </div>
      </div>

      <div class="modal-body">
        <div class="info-text">
          <p style="margin-bottom:8px">Server-side logins are blocked by Riot's IP detection. Use your session cookie instead — it lasts for months.</p>
          <p class="step-label">How to get your <code>ssid</code> cookie:</p>
          <ol class="steps-list">
            <li>Open <strong>Chrome</strong> and go to <code>https://auth.riotgames.com</code></li>
            <li>Log in with your Riot account</li>
            <li>Open DevTools → <strong>Application</strong> → Cookies → <code>https://auth.riotgames.com</code></li>
            <li>Copy the value of the <code>ssid</code> cookie</li>
          </ol>
          <p style="margin-top:8px;font-size:0.7rem;color:#48484a">Or open the console and run: <code>document.cookie.match(/ssid=([^;]+)/)?.[1]</code></p>
        </div>

        <div class="form-group">
          <label class="form-label">ssid cookie value</label>
          <input
            v-model="ssid"
            class="modal-input"
            placeholder="Paste here…"
            autocomplete="off"
            spellcheck="false"
            @keydown.enter="submitSsid"
          />
        </div>

        <p v-if="error" class="form-error">{{ error }}</p>
      </div>

      <div class="modal-footer">
        <button class="btn-cancel" @click="backToCredentials">Back</button>
        <button class="btn-primary" :disabled="loading || !ssid.trim()" @click="submitSsid">
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
