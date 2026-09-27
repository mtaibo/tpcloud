<script setup>
import { ref } from 'vue'
import { Lock, ExternalLink, CheckCircle, ChevronDown } from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const emit = defineEmits(['close', 'saved'])

const popupOpened = ref(false)
const accessToken = ref('')
const loading = ref(false)
const error = ref('')

const showCookie = ref(false)
const ssid = ref('')
const loadingCookie = ref(false)
const errorCookie = ref('')

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
  if (trimmed.includes('access_token=')) {
    const m = trimmed.match(/[#&]access_token=([^&]+)/)
    return m ? m[1] : ''
  }
  if (trimmed.startsWith('eyJ')) return trimmed
  return ''
}

async function saveToken() {
  error.value = ''
  const token = extractToken(accessToken.value)
  if (!token) {
    error.value = 'Pega la URL completa de la barra de direcciones'
    return
  }
  loading.value = true
  try {
    const r = await fetch('/api/val/account/credentials/token', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ access_token: token }),
    })
    const data = await r.json()
    if (!r.ok) {
      error.value = data.detail || 'Token inválido — abre un nuevo login de Riot'
      return
    }
    emit('saved')
  } catch {
    error.value = 'Error de red'
  } finally {
    loading.value = false
  }
}

async function saveCookie() {
  errorCookie.value = ''
  if (!ssid.value.trim()) { errorCookie.value = 'Pega el valor del ssid'; return }
  loadingCookie.value = true
  try {
    const r = await fetch('/api/val/account/credentials/cookie', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ssid: ssid.value.trim() }),
    })
    const data = await r.json()
    if (!r.ok) { errorCookie.value = data.detail || 'ssid inválido'; return }
    emit('saved')
  } catch {
    errorCookie.value = 'Error de red'
  } finally {
    loadingCookie.value = false
  }
}
</script>

<template>
  <BaseModal width="400px" @close="emit('close')">

    <div class="modal-header">
      <Lock :size="18" color="#FF4655" style="flex-shrink:0" />
      <div>
        <p class="modal-title">Riot Credentials</p>
        <span class="modal-subtitle">Solo en memoria · expira en ~1h</span>
      </div>
    </div>

    <div class="modal-body">

      <!-- Step 1: Instructions + open button -->
      <template v-if="!popupOpened">
        <p class="info-text">Para acceder a tu tienda necesitas iniciar sesión con Riot.</p>
        <div class="instructions-box">
          <p class="inst-title">Cómo hacerlo:</p>
          <ol class="inst-list">
            <li>Pulsa el botón de abajo</li>
            <li>Inicia sesión con Riot (incluye tu 2FA)</li>
            <li>Verás una página de error — <strong>es normal</strong></li>
            <li>Copia la URL de la barra de direcciones y vuelve aquí</li>
          </ol>
        </div>
        <button class="btn-riot-login" @click="openRiotLogin">
          <ExternalLink :size="14" />
          Abrir Login de Riot
        </button>
      </template>

      <!-- Step 2: Paste URL -->
      <template v-else>
        <div class="step-done">
          <CheckCircle :size="14" color="#30d158" />
          <span>Ventana de Riot abierta</span>
        </div>

        <div class="instructions-box">
          <p class="inst-title">Tras iniciar sesión, en la barra de direcciones verás:</p>
          <code class="url-example">playvalorant.com/opt_in#access_token=eyJ...</code>
          <p class="inst-subtitle">Copia esa URL completa y pégala aquí:</p>
        </div>

        <div class="form-group" style="margin-top:10px">
          <textarea
            v-model="accessToken"
            class="modal-input token-input"
            placeholder="https://playvalorant.com/opt_in#access_token=eyJ…"
            rows="3"
            spellcheck="false"
            autocomplete="off"
          />
        </div>

        <p v-if="error" class="form-error">{{ error }}</p>
      </template>

      <!-- Advanced: ssid cookie -->
      <button class="btn-advanced" @click="showCookie = !showCookie">
        <ChevronDown :size="12" :style="{ transform: showCookie ? 'rotate(180deg)' : '', transition: 'transform 0.15s' }" />
        Usar cookie ssid (avanzado)
      </button>

      <template v-if="showCookie">
        <div class="instructions-box" style="margin-top:8px">
          <ol class="inst-list">
            <li>Ve a <strong>playvalorant.com</strong> en Chrome e inicia sesión</li>
            <li>F12 → Application → Cookies → <code>auth.riotgames.com</code></li>
            <li>Copia el valor de <strong>ssid</strong></li>
          </ol>
        </div>
        <div class="form-group" style="margin-top:8px">
          <textarea
            v-model="ssid"
            class="modal-input token-input"
            placeholder="Valor de ssid…"
            rows="2"
            spellcheck="false"
            autocomplete="off"
          />
        </div>
        <p v-if="errorCookie" class="form-error">{{ errorCookie }}</p>
      </template>

    </div>

    <div class="modal-footer">
      <button class="btn-cancel" @click="emit('close')">Cancelar</button>
      <button
        v-if="showCookie && ssid.trim()"
        class="btn-primary"
        :disabled="loadingCookie"
        @click="saveCookie"
      >
        {{ loadingCookie ? 'Verificando…' : 'Guardar cookie' }}
      </button>
      <button
        v-else-if="popupOpened"
        class="btn-primary"
        :disabled="loading || !accessToken.trim()"
        @click="saveToken"
      >
        {{ loading ? 'Verificando…' : 'Confirmar' }}
      </button>
      <button v-else class="btn-primary" @click="openRiotLogin">
        <ExternalLink :size="13" />
        Abrir Login de Riot
      </button>
    </div>

  </BaseModal>
</template>

<style scoped>
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
.inst-subtitle {
  font-size: 0.7rem;
  color: #8E8E93;
  margin-top: 8px;
  margin-bottom: 0;
}
.inst-list {
  font-size: 0.72rem;
  color: #8E8E93;
  line-height: 1.75;
  padding-left: 16px;
  margin: 0;
}
.inst-list strong { color: #d1d1d6; }
.inst-list code {
  background: rgba(255,255,255,0.08);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 0.68rem;
  color: #d1d1d6;
}

.url-example {
  display: block;
  font-size: 0.68rem;
  color: #636366;
  background: rgba(0,0,0,0.3);
  padding: 4px 6px;
  border-radius: 4px;
  word-break: break-all;
  margin-top: 4px;
}

.token-input {
  font-family: monospace;
  font-size: 0.68rem;
  resize: none;
  word-break: break-all;
}

.btn-advanced {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 0.68rem;
  color: #525252;
  background: transparent;
  border: none;
  cursor: pointer;
  padding: 0;
  margin-top: 14px;
  transition: color 0.15s;
}
.btn-advanced:hover { color: #8E8E93; }

.info-text {
  font-size: 0.72rem;
  color: #636366;
  line-height: 1.5;
  margin-bottom: 12px;
}

.form-group { margin-bottom: 10px; }
.form-error { font-size: 0.75rem; color: #ff453a; margin-top: 8px; }
</style>
