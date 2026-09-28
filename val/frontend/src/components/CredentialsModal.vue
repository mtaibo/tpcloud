<script setup>
import { ref } from 'vue'
import { Lock, ExternalLink, CheckCircle, ChevronDown } from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const emit = defineEmits(['close', 'saved'])

const tab = ref('ssid') // 'ssid' | 'token'

// ssid flow
const ssid = ref('')
const loadingCookie = ref(false)
const errorCookie = ref('')

// token flow
const popupOpened = ref(false)
const accessToken = ref('')
const loading = ref(false)
const error = ref('')

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
    if (!r.ok) { errorCookie.value = data.detail || 'ssid inválido o expirado'; return }
    emit('saved')
  } catch {
    errorCookie.value = 'Error de red'
  } finally {
    loadingCookie.value = false
  }
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
</script>

<template>
  <BaseModal width="420px" @close="emit('close')">

    <div class="modal-header">
      <Lock :size="18" color="#FF4655" style="flex-shrink:0" />
      <div>
        <p class="modal-title">Conectar Riot</p>
        <span class="modal-subtitle">Necesario para tienda e inventario</span>
      </div>
    </div>

    <!-- Tab switcher -->
    <div class="tab-bar">
      <button :class="['tab', { active: tab === 'ssid' }]" @click="tab = 'ssid'">
        Acceso completo
      </button>
      <button :class="['tab', { active: tab === 'token' }]" @click="tab = 'token'">
        Solo tienda
      </button>
    </div>

    <div class="modal-body">

      <!-- SSID tab (full access) -->
      <template v-if="tab === 'ssid'">
        <p class="info-text">El cookie <strong>ssid</strong> da acceso a tienda e inventario. Se guarda de forma segura y renueva automáticamente.</p>
        <div class="instructions-box">
          <p class="inst-title">Cómo obtener el ssid:</p>
          <ol class="inst-list">
            <li>Ve a <strong>auth.riotgames.com</strong> en Chrome e inicia sesión con tu cuenta Riot</li>
            <li>Pulsa <kbd>F12</kbd> → Application → Cookies → <code>auth.riotgames.com</code></li>
            <li>Copia el valor de la cookie <strong>ssid</strong></li>
          </ol>
        </div>
        <div class="form-group">
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

      <!-- Token tab (shop only) -->
      <template v-else>
        <p class="info-text">Login rápido via OAuth. <strong>Solo da acceso a la tienda</strong>, no al inventario. Expira en ~1h.</p>

        <template v-if="!popupOpened">
          <div class="instructions-box">
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
      </template>

    </div>

    <div class="modal-footer">
      <button class="btn-cancel" @click="emit('close')">Cancelar</button>

      <!-- ssid tab footer -->
      <button
        v-if="tab === 'ssid'"
        class="btn-primary"
        :disabled="loadingCookie || !ssid.trim()"
        @click="saveCookie"
      >
        {{ loadingCookie ? 'Verificando…' : 'Guardar' }}
      </button>

      <!-- token tab footer -->
      <template v-else>
        <button v-if="!popupOpened" class="btn-primary" @click="openRiotLogin">
          <ExternalLink :size="13" />
          Abrir Login de Riot
        </button>
        <button
          v-else
          class="btn-primary"
          :disabled="loading || !accessToken.trim()"
          @click="saveToken"
        >
          {{ loading ? 'Verificando…' : 'Confirmar' }}
        </button>
      </template>
    </div>

  </BaseModal>
</template>

<style scoped>
.tab-bar {
  display: flex;
  border-bottom: 0.5px solid rgba(255,255,255,0.08);
  padding: 0 16px;
}
.tab {
  flex: 1;
  padding: 8px 0;
  font-size: 0.75rem;
  font-weight: 600;
  color: #525252;
  background: transparent;
  border: none;
  border-bottom: 2px solid transparent;
  cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
  margin-bottom: -0.5px;
}
.tab.active { color: #FF4655; border-bottom-color: #FF4655; }
.tab:hover:not(.active) { color: #8E8E93; }

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
.inst-list code, kbd {
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

.info-text {
  font-size: 0.72rem;
  color: #636366;
  line-height: 1.5;
  margin-bottom: 12px;
}
.info-text strong { color: #8E8E93; }

.form-group { margin-bottom: 10px; margin-top: 10px; }
.form-error { font-size: 0.75rem; color: #ff453a; margin-top: 8px; }
</style>
