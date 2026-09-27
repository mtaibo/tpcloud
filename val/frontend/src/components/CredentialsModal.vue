<script setup>
import { ref, computed } from 'vue'
import { Lock, ShieldCheck, ExternalLink, Cookie, KeyRound, FlaskConical, CheckCircle } from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const emit = defineEmits(['close', 'saved'])

const tab = ref('riot') // 'riot' | 'cookie' | 'password'
const step = ref(1) // for password flow: 1 = creds, 2 = MFA

// Riot OAuth popup flow
const popupOpened = ref(false)
const accessToken = ref('')
const loadingToken = ref(false)
const errorToken = ref('')

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

const RIOT_OAUTH_URL =
  'https://auth.riotgames.com/authorize' +
  '?client_id=play-valorant-web-prod' +
  '&nonce=1' +
  '&redirect_uri=https%3A%2F%2Fplayvalorant.com%2Fopt_in' +
  '&response_type=token%20id_token' +
  '&scope=openid' +
  '&language=en_US'

function openRiotLogin() {
  window.open(RIOT_OAUTH_URL, '_blank', 'width=500,height=700,noopener')
  popupOpened.value = true
}

function extractToken(raw) {
  const trimmed = raw.trim()
  // If it's a full URL with fragment
  if (trimmed.includes('access_token=')) {
    const m = trimmed.match(/[#&]access_token=([^&]+)/)
    return m ? m[1] : ''
  }
  // If it's a raw JWT
  if (trimmed.startsWith('eyJ')) return trimmed
  return ''
}

async function saveToken() {
  errorToken.value = ''
  const token = extractToken(accessToken.value)
  if (!token) {
    errorToken.value = 'Paste the full URL from the address bar OR the raw access_token value'
    return
  }
  loadingToken.value = true
  try {
    const r = await fetch('/api/val/account/credentials/token', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ access_token: token }),
    })
    const data = await r.json()
    if (!r.ok) {
      errorToken.value = data.detail || 'Token invalid or expired — get a fresh one'
      return
    }
    emit('saved')
  } catch {
    errorToken.value = 'Network error'
  } finally {
    loadingToken.value = false
  }
}

async function saveCookie() {
  errorCookie.value = ''
  if (!ssid.value.trim()) { errorCookie.value = 'Paste the ssid value'; return }
  loadingCookie.value = true
  try {
    const r = await fetch('/api/val/account/credentials/cookie', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ssid: ssid.value.trim() }),
    })
    const data = await r.json()
    if (!r.ok) { errorCookie.value = data.detail || 'Invalid ssid'; return }
    emit('saved')
  } catch {
    errorCookie.value = 'Network error'
  } finally {
    loadingCookie.value = false
  }
}

async function savePassword() {
  errorPw.value = ''
  if (!username.value || !password.value) { errorPw.value = 'Enter username and password'; return }
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
    if (!r.ok) { errorPw.value = data.detail || 'Invalid credentials'; showDiagnose.value = true; return }
    if (data.requires_mfa) { step.value = 2; return }
    emit('saved')
  } catch {
    errorPw.value = 'Network error'
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
    diagResults.value = [{ strategy: 'network', error: 'Failed', success: false }]
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
  <BaseModal width="400px" @close="emit('close')">

    <div class="modal-header">
      <Lock :size="18" color="#FF4655" style="flex-shrink:0" />
      <div>
        <p class="modal-title">Riot Credentials</p>
        <span class="modal-subtitle">Encrypted and stored on your server</span>
      </div>
    </div>

    <!-- Tabs -->
    <div class="tabs">
      <button :class="['tab', tab === 'riot' && 'active']" @click="tab = 'riot'">
        Riot Login
      </button>
      <button :class="['tab', tab === 'cookie' && 'active']" @click="tab = 'cookie'">
        Cookie
      </button>
      <button :class="['tab', tab === 'password' && 'active']" @click="tab = 'password'">
        Password
      </button>
    </div>

    <!-- ── RIOT OAUTH TAB (default) ── -->
    <template v-if="tab === 'riot'">
      <div class="modal-body">

        <!-- Step 1: open popup -->
        <template v-if="!popupOpened">
          <p class="info-text">
            Log in with your Riot account normally — including 2FA. A Riot login window will open in your browser.
          </p>
          <button class="btn-riot-login" @click="openRiotLogin">
            <ExternalLink :size="14" />
            Open Riot Login
          </button>
        </template>

        <!-- Step 2: instructions + paste -->
        <template v-else>
          <div class="step-done">
            <CheckCircle :size="14" color="#30d158" />
            <span>Riot login window opened</span>
          </div>

          <div class="instructions-box">
            <p class="inst-title">After logging in:</p>
            <ol class="inst-list">
              <li>You'll land on <strong>playvalorant.com</strong></li>
              <li>Press <kbd>F12</kbd> → <strong>Application</strong> → <strong>Cookies</strong> → <strong>playvalorant.com</strong></li>
              <li>Find <code>__Secure-access_token</code> and copy its value</li>
              <li>Paste it below</li>
            </ol>
          </div>

          <div class="form-group" style="margin-top:10px">
            <label class="form-label">Paste the access token or redirect URL</label>
            <textarea
              v-model="accessToken"
              class="modal-input token-input"
              placeholder="eyJraWQiOiJyc28… or https://playvalorant.com/opt_in#access_token=…"
              rows="3"
              spellcheck="false"
              autocomplete="off"
            />
          </div>

          <p v-if="errorToken" class="form-error">{{ errorToken }}</p>
        </template>
      </div>

      <div class="modal-footer">
        <button class="btn-cancel" @click="emit('close')">Cancel</button>
        <button v-if="!popupOpened" class="btn-primary" @click="openRiotLogin">
          <ExternalLink :size="13" /> Open Riot Login
        </button>
        <button v-else class="btn-primary" :disabled="loadingToken || !accessToken.trim()" @click="saveToken">
          {{ loadingToken ? 'Verifying…' : 'Continue' }}
        </button>
      </div>
    </template>

    <!-- ── COOKIE TAB ── -->
    <template v-else-if="tab === 'cookie'">
      <div class="modal-body">
        <div class="instructions-box">
          <p class="inst-title">How to get your ssid cookie</p>
          <ol class="inst-list">
            <li>Go to <strong>playvalorant.com</strong> and log in</li>
            <li>Press <kbd>F12</kbd> → <strong>Application</strong> → <strong>Cookies</strong> → <code>auth.riotgames.com</code></li>
            <li>Find <strong>ssid</strong> and copy its value</li>
          </ol>
        </div>
        <div class="form-group" style="margin-top:12px">
          <label class="form-label">ssid value</label>
          <textarea v-model="ssid" class="modal-input token-input" placeholder="Paste ssid here…" rows="3" spellcheck="false" autocomplete="off" />
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
            <input v-model="mfaCode" class="modal-input mfa-input" placeholder="000000" maxlength="6" inputmode="numeric" pattern="[0-9]*" autocomplete="one-time-code" @keydown.enter="submitMfa" />
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
          <p class="info-text">Note: Riot may block automated login. Use the <strong>Riot Login</strong> tab if this fails.</p>
          <div class="form-group">
            <label class="form-label">Riot username</label>
            <input v-model="username" class="modal-input" placeholder="username" autocomplete="username" />
          </div>
          <div class="form-group">
            <label class="form-label">Password</label>
            <input v-model="password" type="password" class="modal-input" placeholder="••••••••" autocomplete="current-password" @keydown.enter="savePassword" />
          </div>
          <p v-if="errorPw" class="form-error">{{ errorPw }}</p>

          <div v-if="showDiagnose" class="diag-trigger">
            <button class="btn-diag" :disabled="diagnosing" @click="runDiagnosis">
              <FlaskConical :size="12" /> {{ diagnosing ? 'Running…' : 'Run diagnostics' }}
            </button>
            <span class="diag-hint">Tests 4 auth strategies</span>
          </div>
          <div v-if="diagResults" class="diag-results">
            <div v-for="row in diagResults" :key="row.strategy" class="diag-row">
              <span class="diag-strategy">{{ row.strategy }}</span>
              <span class="diag-badge" :style="{ color: diagColor(row), borderColor: diagColor(row) }">{{ diagLabel(row) }}</span>
            </div>
            <p class="diag-note">If all show AUTH → use the Riot Login tab instead.</p>
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
.tab.active { color: #fff; background: rgba(255,255,255,0.08); border-color: rgba(255,255,255,0.1); }

.btn-riot-login {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  width: 100%;
  padding: 12px;
  border-radius: 10px;
  background: #FF4655;
  border: none;
  color: #fff;
  font-size: 0.85rem;
  font-weight: 700;
  cursor: pointer;
  margin-top: 12px;
  transition: opacity 0.15s;
}
.btn-riot-login:hover { opacity: 0.9; }

.step-done {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.75rem;
  color: #30d158;
  margin-bottom: 12px;
}

.instructions-box {
  background: rgba(255, 255, 255, 0.04);
  border: 0.5px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 10px 12px;
}
.inst-title {
  font-size: 0.7rem;
  font-weight: 700;
  color: #d1d1d6;
  margin-bottom: 6px;
}
.inst-list {
  font-size: 0.72rem;
  color: #8E8E93;
  line-height: 1.75;
  padding-left: 16px;
  margin: 0;
}
.inst-list strong { color: #d1d1d6; }
.inst-list code, .inst-list kbd {
  background: rgba(255,255,255,0.08);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 0.68rem;
  color: #d1d1d6;
}

.token-input {
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
.mfa-input { letter-spacing: 0.3em; font-size: 1.25rem; text-align: center; font-variant-numeric: tabular-nums; }
.form-error { font-size: 0.75rem; color: #ff453a; margin-top: 8px; }

.diag-trigger { display: flex; align-items: center; gap: 8px; margin-top: 10px; }
.btn-diag { display: flex; align-items: center; gap: 5px; font-size: 0.7rem; padding: 4px 10px; border-radius: 6px; background: rgba(255,255,255,0.06); border: 0.5px solid rgba(255,255,255,0.12); color: #8E8E93; cursor: pointer; }
.btn-diag:disabled { opacity: 0.5; cursor: not-allowed; }
.diag-hint { font-size: 0.65rem; color: #525252; }
.diag-results { margin-top: 10px; padding: 8px; background: rgba(0,0,0,0.3); border-radius: 8px; border: 0.5px solid rgba(255,255,255,0.08); }
.diag-row { display: grid; grid-template-columns: 110px 70px; align-items: center; gap: 6px; padding: 3px 0; }
.diag-strategy { font-size: 0.65rem; color: #8E8E93; font-family: monospace; }
.diag-badge { font-size: 0.58rem; font-weight: 700; padding: 2px 4px; border-radius: 4px; border: 0.5px solid; text-align: center; }
.diag-note { font-size: 0.65rem; color: #636366; margin-top: 6px; }
</style>
