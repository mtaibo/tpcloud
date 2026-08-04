<script setup>
import { ref } from 'vue'

const email = ref('')
const mode = ref('login')
const message = ref(null) // { type: 'error' | 'success', text: string }
const loading = ref(false)

const params = new URLSearchParams(window.location.search)
const redirectHost = params.get('redirect') || 'cloud.migueltaibo.com'
const state = params.get('state') || ''

function setError(e) {
  if (e.name === 'NotAllowedError') {
    message.value = { type: 'error', text: 'Operación cancelada.' }
  } else {
    message.value = { type: 'error', text: e.message || 'Error inesperado' }
  }
}

async function register() {
  if (!email.value || loading.value) return
  loading.value = true
  message.value = null
  try {
    const optsRes = await fetch(
      `/auth/passkey/register/begin?email=${encodeURIComponent(email.value)}`,
      { method: 'POST' }
    )
    const optsBody = await optsRes.json()
    if (!optsRes.ok) throw new Error(optsBody.detail)

    const cred = await navigator.credentials.create({
      publicKey: PublicKeyCredential.parseCreationOptionsFromJSON(optsBody)
    })

    const res = await fetch(
      `/auth/passkey/register/complete?email=${encodeURIComponent(email.value)}`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(cred.toJSON()),
      }
    )
    if (!res.ok) throw new Error((await res.json()).detail)

    message.value = { type: 'success', text: 'Passkey registrada. Ya puedes iniciar sesión.' }
    mode.value = 'login'
  } catch (e) {
    setError(e)
  } finally {
    loading.value = false
  }
}

async function login() {
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
    completeUrl.searchParams.set('redirect', redirectHost)
    completeUrl.searchParams.set('state', state)

    const res = await fetch(completeUrl.toString(), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(cred.toJSON()),
      redirect: 'manual',
    })

    if (res.type === 'opaqueredirect' || res.ok) {
      if (redirectHost) {
        window.location.href = `https://${redirectHost}`
      } else {
        message.value = { type: 'success', text: 'Sesión iniciada correctamente.' }
      }
    } else {
      throw new Error((await res.json()).detail)
    }
  } catch (e) {
    setError(e)
  } finally {
    loading.value = false
  }
}

function switchMode(next) {
  mode.value = next
  message.value = null
}
</script>

<template>
  <div class="page">
    <div class="container">
      <p class="label">tpcloud</p>

      <div class="heading">
        <h1>{{ mode === 'login' ? 'Accede a tu cuenta' : 'Registrar passkey' }}</h1>
        <p class="subtitle">
          {{ mode === 'login'
            ? 'Introduce tu correo y usa tu passkey para autenticarte.'
            : 'Introduce tu correo para vincular una nueva passkey a tu cuenta.' }}
        </p>
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
            @keydown.enter="mode === 'login' ? login() : register()"
          />
        </div>

        <button
          :disabled="loading || !email"
          @click="mode === 'login' ? login() : register()"
        >
          <span v-if="loading" class="spinner" />
          <span v-else>{{ mode === 'login' ? 'Continuar con passkey' : 'Registrar passkey' }}</span>
        </button>
      </div>

      <p v-if="message" :class="['message', message.type]">{{ message.text }}</p>

      <p class="footer">
        <template v-if="mode === 'login'">
          ¿Aún no tienes passkey?
          <button class="link" @click="switchMode('register')">Registrar</button>
        </template>
        <template v-else>
          ¿Ya tienes passkey?
          <button class="link" @click="switchMode('login')">Iniciar sesión</button>
        </template>
      </p>
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

input[type="email"] {
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

input[type="email"]::placeholder { color: #404040; }
input[type="email"]:focus { border-color: #525252; }
input[type="email"]:disabled { opacity: 0.4; cursor: not-allowed; }

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
