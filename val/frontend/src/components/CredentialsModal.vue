<script setup>
import { ref } from 'vue'
import { Lock, ShieldCheck, Cookie, KeyRound, FlaskConical } from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const emit = defineEmits(['close', 'saved'])

const tab = ref('cookie') // 'cookie' | 'password'
const step = ref(1) // for password flow: 1 = creds, 2 = MFA

// Cookie auth
const ssid = ref('')
const loadingCookie = ref(false)
const errorCookie = ref('')

// Password auth
const username = ref('')
const password = ref('')
const mfaCode = ref('')
const loadingPw = ref(false)
const errorPw = ref('')
const showDiagnose = ref(false)
const diagnosing = ref(false)
const diagResults = ref(null)

async function saveCookie() {
  errorCookie.value = ''
  if (!ssid.value.trim()) {
    errorCookie.value = 'Paste the ssid cookie value'
    return
  }
  loadingCookie.value = true
  try {
    const r = await fetch('/api/val/account/credentials/cookie', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ssid: ssid.value.trim() }),
    })
    const data = await r.json()
    if (!r.ok) {
      errorCookie.value = data.detail || 'Invalid ssid — get a fresh one from your browser'
      return
    }
    emit('saved')
  } catch {
    errorCookie.value = 'Network error'
  } finally {
    loadingCookie.value = false
  }
}

async function savePassword() {
  errorPw.value = ''
  if (!username.value || !password.value) {
    errorPw.value = 'Enter your Riot username and password'
    return
  }
  loadingPw.value = true
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
      errorPw.value = data.detail || 'Invalid credentials'
      showDiagnose.value = true
      return
    }
    if (data.requires_mfa) {
      step.value = 2
      return
    }
    emit('saved')
  } catch {
    errorPw.value = 'Network error — check server logs'
    showDiagnose.value = true
  } finally {
    loadingPw.value = false
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
  errorPw.value = ''
  if (!mfaCode.value.trim()) { errorPw.value = 'Enter the 6-digit code'; return }
  loadingPw.value = true
  try {
    const r = await fetch('/api/val/account/credentials/mfa', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code: mfaCode.value.trim() }),
    })
    const data = await r.json()
    if (!r.ok) { errorPw.value = data.detail || 'Invalid MFA code'; return }
    emit('saved')
  } catch {
    errorPw.value = 'Network error'
  } finally {
    loadingPw.value = false
  }
}

function backToCredentials() {
  step.value = 1
  mfaCode.value = ''
  errorPw.value = ''
}
</script>

<template>
  <BaseModal width="390px" @close="emit('close')">

    <div class="modal-header">
      <Lock :size="18" color="#FF4655" style="flex-shrink:0" />
      <div>
        <p class="modal-title">Riot Credentials</p>
        <span class="modal-subtitle">Encrypted and stored on your server</span>
      </div>
    </div>

    <!-- Tabs -->
    <div class="tabs">
      <button :class="['tab', tab === 'cookie' && 'active']" @click="tab = 'cookie'">
        <Cookie :size="13" /> Browser Cookie
      </button>
      <button :class="['tab', tab === 'password' && 'active']" @click="tab = 'password'">
        <KeyRound :size="13" /> Password
      </button>
    </div>

    <!-- ── COOKIE TAB ── -->
    <template v-if="tab === 'cookie'">
      <div class="modal-body">
        <div class="steps-box">
          <p class="steps-title">How to get your ssid cookie</p>
          <ol class="steps-list">
            <li>Go to <strong>playvalorant.com</strong> in your browser and log in</li>
            <li>Press <kbd>F12</kbd> to open DevTools</li>
            <li>Go to <strong>Application</strong> → <strong>Cookies</strong> → <code>https://auth.riotgames.com</code></li>
            <li>Find the row named <strong>ssid</strong> and copy its value</li>
          </ol>
        </div>

        <div class="form-group" style="margin-top:12px">
          <label class="form-label">ssid cookie value</label>
          <textarea
            v-model="ssid"
            class="modal-input ssid-input"
            placeholder="Paste the long ssid value here…"
            rows="3"
            spellcheck="false"
            autocomplete="off"
          />
        </div>

        <p v-if="errorCookie" class="form-error">{{ errorCookie }}</p>
      </div>

      <div class="modal-footer">
        <button class="btn-cancel" @click="emit('close')">Cancel</button>
        <button class="btn-primary" :disabled="loadingCookie || !ssid.trim()" @click="saveCookie">
          {{ loadingCookie ? 'Verifying…' : 'Continue' }}
        </button>
      </div>
    </template>

    <!-- ── PASSWORD TAB ── -->
    <template v-else>

      <!-- MFA step -->
      <template v-if="step === 2">
        <div class="modal-body">
          <div class="mfa-header">
            <ShieldCheck :size="16" color="#ffd60a" />
            <span>Two-Factor Authentication</span>
          </div>
          <p class="info-text">Enter the 6-digit code from your authenticator app.</p>
          <div class="form-group">
            <label class="form-label">Authentication code</label>
            <input v-model="mfaCode" class="modal-input mfa-input" placeholder="000000"
              maxlength="6" inputmode="numeric" pattern="[0-9]*"
              autocomplete="one-time-code" @keydown.enter="submitMfa" />
          </div>
          <p v-if="errorPw" class="form-error">{{ errorPw }}</p>
        </div>
        <div class="modal-footer">
          <button class="btn-cancel" @click="backToCredentials">Back</button>
          <button class="btn-primary" :disabled="loadingPw || mfaCode.length < 6" @click="submitMfa">
            {{ loadingPw ? 'Verifying…' : 'Verify' }}
          </button>
        </div>
      </template>

      <!-- Credentials step -->
      <template v-else>
        <div class="modal-body">
          <p class="info-text">
            Username and password are encrypted server-side. Note: Riot may block automated login — use the Cookie method if this fails.
          </p>
          <div class="form-group">
            <label class="form-label">Riot username</label>
            <input v-model="username" class="modal-input" placeholder="username" autocomplete="username" />
          </div>
          <div class="form-group">
            <label class="form-label">Password</label>
            <input v-model="password" type="password" class="modal-input" placeholder="••••••••"
              autocomplete="current-password" @keydown.enter="savePassword" />
          </div>
          <p v-if="errorPw" class="form-error">{{ errorPw }}</p>

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
                {{ row.captcha ? 'bot blocked' : (row.error && !row.type) ? row.error : `init=${row.init_status} auth=${row.auth_status}` }}
              </span>
            </div>
            <p class="diag-note">If all show AUTH, Riot is blocking — use the Cookie method instead.</p>
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn-cancel" @click="emit('close')">Cancel</button>
          <button class="btn-primary" :disabled="loadingPw || !username || !password" @click="savePassword">
            {{ loadingPw ? 'Verifying…' : 'Continue' }}
          </button>
        </div>
      </template>
    </template>

  </BaseModal>
</template>

<style scoped>
.tabs {
  display: flex;
  gap: 4px;
  padding: 0 16px;
  margin-bottom: 2px;
}

.tab {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 0.72rem;
  font-weight: 600;
  padding: 5px 12px;
  border-radius: 7px;
  border: 0.5px solid transparent;
  background: transparent;
  color: #636366;
  cursor: pointer;
  transition: all 0.15s;
}
.tab:hover { color: #8E8E93; background: rgba(255,255,255,0.04); }
.tab.active {
  color: #fff;
  background: rgba(255,255,255,0.08);
  border-color: rgba(255,255,255,0.1);
}

.steps-box {
  background: rgba(255, 214, 10, 0.06);
  border: 0.5px solid rgba(255, 214, 10, 0.18);
  border-radius: 8px;
  padding: 10px 12px;
}

.steps-title {
  font-size: 0.7rem;
  font-weight: 700;
  color: #ffd60a;
  margin-bottom: 8px;
  letter-spacing: 0.02em;
}

.steps-list {
  font-size: 0.72rem;
  color: #8E8E93;
  line-height: 1.7;
  padding-left: 16px;
  margin: 0;
}
.steps-list strong { color: #d1d1d6; }
.steps-list code, .steps-list kbd {
  background: rgba(255,255,255,0.08);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 0.68rem;
  color: #d1d1d6;
}

.ssid-input {
  font-family: monospace;
  font-size: 0.68rem;
  resize: none;
  word-break: break-all;
}

.mfa-header {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.8rem;
  font-weight: 600;
  color: #ffd60a;
  margin-bottom: 8px;
}

.info-text {
  font-size: 0.72rem;
  color: #636366;
  line-height: 1.5;
  margin-bottom: 12px;
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

.form-error { font-size: 0.75rem; color: #ff453a; margin-top: 8px; }

.diag-trigger { display: flex; align-items: center; gap: 8px; margin-top: 10px; }

.btn-diag {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 0.7rem;
  padding: 4px 10px;
  border-radius: 6px;
  background: rgba(255,255,255,0.06);
  border: 0.5px solid rgba(255,255,255,0.12);
  color: #8E8E93;
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;
}
.btn-diag:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-diag:not(:disabled):hover { background: rgba(255,255,255,0.1); color: #d1d1d6; }

.diag-hint { font-size: 0.65rem; color: #525252; }

.diag-results {
  margin-top: 10px;
  padding: 10px;
  background: rgba(0,0,0,0.3);
  border-radius: 8px;
  border: 0.5px solid rgba(255,255,255,0.08);
}
.diag-title { font-size: 0.65rem; font-weight: 600; color: #636366; margin-bottom: 8px; text-transform: uppercase; letter-spacing: 0.05em; }
.diag-row { display: grid; grid-template-columns: 108px 62px 1fr; align-items: center; gap: 6px; padding: 4px 0; border-bottom: 0.5px solid rgba(255,255,255,0.04); }
.diag-row:last-of-type { border-bottom: none; }
.diag-strategy { font-size: 0.65rem; color: #8E8E93; font-family: monospace; }
.diag-badge { font-size: 0.58rem; font-weight: 700; padding: 2px 4px; border-radius: 4px; border: 0.5px solid; text-align: center; }
.diag-detail { font-size: 0.6rem; color: #525252; font-family: monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.diag-note { font-size: 0.65rem; color: #636366; margin-top: 8px; line-height: 1.4; }
</style>
