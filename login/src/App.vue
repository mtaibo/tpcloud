<script setup>
import { ref, onMounted } from 'vue'

const email = ref('')
const password = ref('')
const totpCode = ref('')
const step = ref('credentials') // 'credentials' | 'totp'
const message = ref(null)
const loading = ref(false)
const usingPasskey = ref(false)

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

function backToCredentials() {
  step.value = 'credentials'
  totpCode.value = ''
  message.value = null
}
</script>

<template>
  <div class="page">
    <div class="container">
      <p class="label">tpcloud</p>

      <!-- Step: credentials -->
      <template v-if="step === 'credentials'">
        <div class="heading">
          <h1>Accede a tu cuenta</h1>
          <p class="subtitle">Introduce tu correo y contraseña para autenticarte.</p>
        </div>

        <div class="form">
          <div class="field">
            <label for="email">Correo electrónico</label>
            <input
              id="email"
              v-model="email"
              type="email"
              placeholder="usuario@ejemplo.com"
              autocomplete="email"
              :disabled="loading"
              @keydown.enter="loginWithPassword"
            />
          </div>

          <div class="field">
            <label for="password">Contraseña</label>
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

          <button :disabled="loading || !email || !password" @click="loginWithPassword">
            <span v-if="loading" class="spinner" />
            <span v-else>Iniciar sesión</span>
          </button>

          <button class="btn-secondary" :disabled="loading || !email" @click="loginWithPasskey">
            <span v-if="loading" class="spinner" />
            <span v-else>Usar passkey en su lugar</span>
          </button>
        </div>
      </template>

      <!-- Step: TOTP -->
      <template v-else-if="step === 'totp'">
        <div class="heading">
          <h1>Verificación en dos pasos</h1>
          <p class="subtitle">Introduce el código de 6 dígitos de tu aplicación de autenticación.</p>
        </div>

        <div class="form">
          <div class="field">
            <label for="totp">Código TOTP</label>
            <input
              id="totp"
              v-model="totpCode"
              type="text"
              inputmode="numeric"
              pattern="[0-9]*"
              maxlength="6"
              placeholder="000000"
              autocomplete="one-time-code"
              :disabled="loading"
              @keydown.enter="verifyTotp"
            />
          </div>

          <button :disabled="loading || totpCode.length < 6" @click="verifyTotp">
            <span v-if="loading" class="spinner" />
            <span v-else>Verificar</span>
          </button>
        </div>

        <p class="footer">
          <button class="link" @click="backToCredentials">Volver</button>
        </p>
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
  border: 1px solid #262626;
  border-radius: 0.5rem;
  padding: 2rem;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.label {
  font-size: 0.75rem;
  font-weight: 500;
  color: #737373;
  text-transform: uppercase;
  letter-spacing: 0.1em;
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
  background: #000;
  border: 1px solid #262626;
  border-radius: 0.5rem;
  padding: 0.6rem 0.75rem;
  color: #fff;
  font-size: 0.9rem;
  outline: none;
  transition: border-color 0.2s;
}

input::placeholder { color: #404040; }
input:focus { border-color: #525252; }
input:disabled { opacity: 0.4; cursor: not-allowed; }

button {
  width: 100%;
  background: transparent;
  border: 1px solid #404040;
  border-radius: 0.5rem;
  padding: 0.65rem 1rem;
  color: #fff;
  font-size: 0.9rem;
  font-weight: 500;
  cursor: pointer;
  transition: border-color 0.3s;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 2.5rem;
}

button:hover:not(:disabled) { border-color: #737373; }
button:disabled { opacity: 0.35; cursor: not-allowed; }

.btn-secondary {
  border-color: #262626;
  color: #737373;
  font-size: 0.85rem;
}

.btn-secondary:hover:not(:disabled) {
  border-color: #404040;
  color: #a3a3a3;
}

.spinner {
  width: 14px;
  height: 14px;
  border: 1.5px solid #525252;
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
