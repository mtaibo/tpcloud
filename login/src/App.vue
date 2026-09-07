<script setup>
import { ref, onMounted } from 'vue'

const email = ref('')
const password = ref('')
const totpCode = ref('')
const step = ref('email') // 'email' | 'password' | 'totp'
const message = ref(null)
const loading = ref(false)

const params = new URLSearchParams(window.location.search)
const redirectHost = params.get('redirect') || 'cloud.migueltaibo.com'

onMounted(async () => {
  const res = await fetch('/auth/passkey/me')
  if (res.ok) window.location.href = `https://${redirectHost}`
})

function setError(e) {
  if (e.name === 'NotAllowedError') {
    message.value = { type: 'error', text: 'Operación cancelada.' }
  } else {
    message.value = { type: 'error', text: e.message || 'Error inesperado' }
  }
}

function continueWithPassword() {
  if (!email.value) return
  step.value = 'password'
  message.value = null
}

async function loginWithPassword() {
  if (!email.value || !password.value || loading.value) return
  loading.value = true
  message.value = null
  try {
    const res = await fetch('/auth/password/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email: email.value, password: password.value }),
    })
    const data = await res.json()
    if (!res.ok) throw new Error(data.detail)
    if (data.totp_required) {
      step.value = 'totp'
    } else {
      window.location.href = `https://${redirectHost}`
    }
  } catch (e) {
    setError(e)
  } finally {
    loading.value = false
  }
}

async function verifyTotp() {
  if (!totpCode.value || loading.value) return
  loading.value = true
  message.value = null
  try {
    const res = await fetch('/auth/password/totp/verify', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code: totpCode.value }),
    })
    const data = await res.json()
    if (!res.ok) throw new Error(data.detail)
    window.location.href = `https://${redirectHost}`
  } catch (e) {
    setError(e)
  } finally {
    loading.value = false
  }
}

async function loginWithPasskey() {
  if (!email.value || loading.value) return
  loading.value = true
  message.value = null
  try {
    const optsRes = await fetch(
      `/auth/passkey/login/begin?email=${encodeURIComponent(email.value)}`,
      { method: 'POST' }
    )
    const optsBody = await optsRes.json()
    if (!optsRes.ok) throw new Error(optsBody.detail)

    const cred = await navigator.credentials.get({
      publicKey: PublicKeyCredential.parseRequestOptionsFromJSON(optsBody)
    })

    const completeUrl = new URL('/auth/passkey/login/complete', window.location.origin)
    completeUrl.searchParams.set('email', email.value)

    const res = await fetch(completeUrl.toString(), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(cred.toJSON()),
    })

    if (res.ok) {
      window.location.href = `https://${redirectHost}`
    } else {
      throw new Error((await res.json()).detail)
    }
  } catch (e) {
    setError(e)
  } finally {
    loading.value = false
  }
}

function backToPassword() {
  step.value = 'password'
  totpCode.value = ''
  message.value = null
}
</script>

<template>
  <div class="page">
    <div class="container">

      <!-- Step: email -->
      <template v-if="step === 'email'">
        <h1>Login</h1>

        <div class="form">
          <div class="field">
            <label for="email">MAIL</label>
            <input
              id="email"
              v-model="email"
              type="email"
              placeholder=""
              autocomplete="email"
              :disabled="loading"
              @keydown.enter="continueWithPassword"
            />
          </div>

          <div class="btn-row">
            <button :disabled="!email" @click="continueWithPassword">
              Continue
            </button>

            <button class="btn-secondary" :disabled="loading || !email" @click="loginWithPasskey">
              <span v-if="loading" class="spinner" />
              <template v-else>
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="7.5" cy="15.5" r="5.5"/><path d="M21 2l-9.6 9.6"/><path d="M15.5 7.5l3 3L22 7l-3-3"/></svg>
                Passkey
              </template>
            </button>
          </div>
        </div>
      </template>

      <!-- Step: password -->
      <template v-else-if="step === 'password'">
        <h1>Login</h1>

        <div class="form">
          <div class="field">
            <label for="email">MAIL</label>
            <input
              id="email"
              v-model="email"
              type="email"
              placeholder=""
              autocomplete="email"
              :disabled="loading"
            />
          </div>

          <div class="field">
            <label for="password">PASSWORD</label>
            <input
              id="password"
              v-model="password"
              type="password"
              placeholder="••••••••"
              autocomplete="current-password"
              :disabled="loading"
              @keydown.enter="loginWithPassword"
            />
          </div>

          <div class="btn-row">
            <button :disabled="loading || !email || !password" @click="loginWithPassword">
              <span v-if="loading" class="spinner" />
              <span v-else>Log in</span>
            </button>

            <button class="btn-secondary" :disabled="loading || !email" @click="loginWithPasskey">
              <span v-if="loading" class="spinner" />
              <template v-else>
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="7.5" cy="15.5" r="5.5"/><path d="M21 2l-9.6 9.6"/><path d="M15.5 7.5l3 3L22 7l-3-3"/></svg>
                Passkey
              </template>
            </button>
          </div>
        </div>
      </template>

      <!-- Step: TOTP -->
      <template v-else-if="step === 'totp'">
        <h1>Two-factor authentication</h1>

        <div class="form">
          <div class="field">
            <label for="totp">CODE</label>
            <input
              id="totp"
              v-model="totpCode"
              type="text"
              inputmode="numeric"
              pattern="[0-9]*"
              maxlength="6"
              placeholder="······"
              autocomplete="one-time-code"
              :disabled="loading"
              @keydown.enter="verifyTotp"
            />
          </div>

          <div class="btn-row">
            <button :disabled="loading || totpCode.length < 6" @click="verifyTotp">
              <span v-if="loading" class="spinner" />
              <span v-else>Continue</span>
            </button>

            <button class="btn-secondary" @click="backToPassword">Back</button>
          </div>
        </div>
      </template>

      <p v-if="message" :class="['message', message.type]">{{ message.text }}</p>
    </div>
  </div>
</template>

<style>
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

html, body, #app {
  height: 100%;
  background: #000;
  color: #fff;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
}

::-webkit-scrollbar { display: none; }
* { -ms-overflow-style: none; scrollbar-width: none; }
</style>

<style scoped>
.page {
  min-height: 100dvh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
}

.container {
  width: 100%;
  max-width: 380px;
  border: 0.5px solid rgba(255, 255, 255, 0.14);
  border-radius: 22px;
  padding: 2rem;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  background: rgba(255, 255, 255, 0.065);
  backdrop-filter: blur(24px) saturate(160%);
  -webkit-backdrop-filter: blur(24px) saturate(160%);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.35),
    inset 0 1px 0 rgba(255, 255, 255, 0.12),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15);
}

.heading {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

h1 {
  font-size: 1.25rem;
  font-weight: 600;
  color: #fff;
  line-height: 1.3;
}

.subtitle {
  font-size: 0.875rem;
  color: #a3a3a3;
  line-height: 1.6;
}

.form {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

label {
  font-size: 0.75rem;
  font-weight: 500;
  color: #737373;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

input {
  width: 100%;
  background: rgba(255, 255, 255, 0.05);
  border: 0.5px solid rgba(255, 255, 255, 0.14);
  border-radius: 10px;
  padding: 0.6rem 0.75rem;
  color: #fff;
  font-size: 0.9rem;
  outline: none;
  transition: border-color 0.2s;
}

input::placeholder { color: #404040; }
input:focus { border-color: rgba(255, 255, 255, 0.3); }
input:disabled { opacity: 0.4; cursor: not-allowed; }

.btn-row {
  display: flex;
  gap: 0.5rem;
  margin-top: 2rem;
}

.btn-row button {
  flex: 1;
  font-size: 0.85rem;
}

button {
  width: 100%;
  background: linear-gradient(
    145deg,
    rgba(255, 255, 255, 0.14) 0%,
    rgba(255, 255, 255, 0.07) 50%,
    rgba(255, 255, 255, 0.10) 100%
  );
  backdrop-filter: blur(24px) saturate(200%);
  -webkit-backdrop-filter: blur(24px) saturate(200%);
  border: 0.5px solid rgba(255, 255, 255, 0.22);
  border-radius: 14px;
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.36),
    inset 0 -0.5px 0 rgba(0, 0, 0, 0.18),
    0 4px 12px rgba(0, 0, 0, 0.24),
    0 1px 3px rgba(0, 0, 0, 0.14);
  padding: 0.65rem 1rem;
  color: #fff;
  font-size: 0.9rem;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.18s ease, box-shadow 0.18s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
  min-height: 2.5rem;
}

button:hover:not(:disabled) {
  background: linear-gradient(
    145deg,
    rgba(255, 255, 255, 0.20) 0%,
    rgba(255, 255, 255, 0.11) 50%,
    rgba(255, 255, 255, 0.16) 100%
  );
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.42),
    inset 0 -0.5px 0 rgba(0, 0, 0, 0.18),
    0 6px 18px rgba(0, 0, 0, 0.30),
    0 1px 3px rgba(0, 0, 0, 0.14);
}

button:disabled { opacity: 0.35; cursor: not-allowed; }

.btn-secondary {
  background: linear-gradient(
    145deg,
    rgba(255, 255, 255, 0.07) 0%,
    rgba(255, 255, 255, 0.03) 100%
  );
  border-color: rgba(255, 255, 255, 0.11);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.18),
    inset 0 -0.5px 0 rgba(0, 0, 0, 0.12),
    0 2px 8px rgba(0, 0, 0, 0.16);
  color: #a3a3a3;
}

.btn-secondary:hover:not(:disabled) {
  background: linear-gradient(
    145deg,
    rgba(255, 255, 255, 0.11) 0%,
    rgba(255, 255, 255, 0.06) 100%
  );
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.24),
    inset 0 -0.5px 0 rgba(0, 0, 0, 0.12),
    0 4px 12px rgba(0, 0, 0, 0.22);
  color: #e4e4e7;
}

.spinner {
  width: 14px;
  height: 14px;
  border: 1.5px solid rgba(255, 255, 255, 0.3);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }

.message {
  font-size: 0.85rem;
  line-height: 1.5;
}

.message.error { color: #fca5a5; }
.message.success { color: #86efac; }

.footer {
  font-size: 0.82rem;
  color: #737373;
  text-align: center;
}

.link {
  all: unset;
  color: #a3a3a3;
  cursor: pointer;
  transition: color 0.2s;
  padding: 0 0.15rem;
}

.link:hover { color: #fff; }
</style>
