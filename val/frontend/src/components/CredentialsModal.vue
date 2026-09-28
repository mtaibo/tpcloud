<script setup>
import { ref } from 'vue'
import BaseModal from './BaseModal.vue'

const emit = defineEmits(['close', 'saved'])

const step = ref(1) // 1 = login, 2 = copy cookie, 3 = paste
const ssid = ref('')
const loading = ref(false)
const error = ref('')

const OAUTH_URL =
  'https://auth.riotgames.com/authorize' +
  '?client_id=play-valorant-web-prod' +
  '&nonce=1' +
  '&redirect_uri=https%3A%2F%2Fplayvalorant.com%2Fopt_in' +
  '&response_type=token%20id_token' +
  '&scope=openid' +
  '&language=en_US'

function openLogin() {
  window.open('https://account.riotgames.com', '_blank')
  step.value = 2
}

function openAuthCookies() {
  window.open('https://auth.riotgames.com', '_blank')
  step.value = 3
}

async function save() {
  error.value = ''
  if (!ssid.value.trim()) return
  loading.value = true
  try {
    const r = await fetch('/api/val/account/credentials/cookie', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ssid: ssid.value.trim() }),
    })
    const data = await r.json()
    if (!r.ok) { error.value = data.detail || 'Cookie inválida o expirada'; return }
    emit('saved')
  } catch {
    error.value = 'Error de red'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <BaseModal width="360px" @close="emit('close')">
    <div class="modal">

      <!-- Step 1: log in with Riot -->
      <template v-if="step === 1">
        <p class="title">Conectar Riot</p>
        <p class="sub">Una vez configurado da acceso a tu tienda e inventario. No vuelves a hacerlo hasta que cambies la contraseña.</p>
        <div class="steps">
          <div class="step-row"><span class="num">1</span><span>Abre <strong>account.riotgames.com</strong> con el botón de abajo</span></div>
          <div class="step-row"><span class="num">2</span><span>Inicia sesión con tu cuenta de Riot si te lo pide</span></div>
          <div class="step-row"><span class="num">3</span><span>Vuelve aquí y pulsa <strong>Siguiente</strong></span></div>
        </div>
        <button class="btn-primary" @click="openLogin">Abrir account.riotgames.com →</button>
        <button class="btn-ghost" @click="step = 2">Ya he iniciado sesión antes</button>
      </template>

      <!-- Step 2: go get cookie -->
      <template v-else-if="step === 2">
        <button class="back" @click="step = 1">← Volver</button>
        <p class="title">Ahora copia la cookie</p>
        <div class="steps">
          <div class="step-row"><span class="num">1</span><span>Pulsa el botón — abre <code>auth.riotgames.com</code></span></div>
          <div class="step-row"><span class="num">2</span><span><kbd>F12</kbd> → <strong>Application</strong> → <strong>Cookies</strong> → <code>auth.riotgames.com</code></span></div>
          <div class="step-row"><span class="num">3</span><span>Copia el valor de la cookie <strong>ssid</strong></span></div>
        </div>
        <button class="btn-primary" @click="openAuthCookies">Abrir auth.riotgames.com →</button>
        <button class="btn-ghost" @click="step = 3">Ya lo tengo copiado</button>
      </template>

      <!-- Step 3: paste -->
      <template v-else>
        <button class="back" @click="step = 2">← Volver</button>
        <p class="title">Pega el valor de ssid</p>
        <textarea
          v-model="ssid"
          class="input"
          placeholder="Pega aquí el valor de ssid…"
          rows="3"
          spellcheck="false"
          autocomplete="off"
          autofocus
        />
        <p v-if="error" class="error">{{ error }}</p>
        <div class="footer">
          <button class="btn-cancel" @click="emit('close')">Cancelar</button>
          <button class="btn-primary" :disabled="loading || !ssid.trim()" @click="save">
            {{ loading ? 'Verificando…' : 'Guardar' }}
          </button>
        </div>
      </template>

    </div>
  </BaseModal>
</template>

<style scoped>
.modal {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 20px;
}
.title { font-size: 0.9rem; font-weight: 700; color: #fff; }
.sub { font-size: 0.73rem; color: #636366; line-height: 1.5; }

.steps {
  display: flex;
  flex-direction: column;
  gap: 10px;
  background: rgba(255,255,255,0.03);
  border: 0.5px solid rgba(255,255,255,0.07);
  border-radius: 10px;
  padding: 14px;
}
.step-row {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  font-size: 0.76rem;
  color: #8E8E93;
  line-height: 1.5;
}
.step-row strong { color: #d1d1d6; }

.num {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  background: rgba(255,70,85,0.15);
  border: 0.5px solid rgba(255,70,85,0.3);
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.65rem;
  font-weight: 700;
  color: #FF4655;
  margin-top: 1px;
}

kbd, code {
  background: rgba(255,255,255,0.08);
  border-radius: 3px;
  padding: 1px 5px;
  font-size: 0.7rem;
  color: #d1d1d6;
  font-family: monospace;
}

.btn-primary {
  width: 100%;
  padding: 11px;
  border-radius: 9px;
  background: #FF4655;
  border: none;
  color: #fff;
  font-size: 0.82rem;
  font-weight: 700;
  cursor: pointer;
  transition: opacity 0.15s;
}
.btn-primary:hover:not(:disabled) { opacity: 0.88; }
.btn-primary:disabled { opacity: 0.4; cursor: default; }

.btn-ghost {
  width: 100%;
  padding: 8px;
  border-radius: 9px;
  background: transparent;
  border: 0.5px solid rgba(255,255,255,0.1);
  color: #525252;
  font-size: 0.76rem;
  cursor: pointer;
  transition: color 0.15s;
}
.btn-ghost:hover { color: #8E8E93; }

.back {
  background: none;
  border: none;
  color: #525252;
  font-size: 0.72rem;
  cursor: pointer;
  padding: 0;
  text-align: left;
  transition: color 0.15s;
}
.back:hover { color: #8E8E93; }

.input {
  width: 100%;
  background: rgba(255,255,255,0.05);
  border: 0.5px solid rgba(255,255,255,0.1);
  border-radius: 8px;
  color: #fff;
  font-family: monospace;
  font-size: 0.7rem;
  padding: 10px;
  resize: none;
  outline: none;
  transition: border-color 0.15s;
}
.input:focus { border-color: rgba(255,70,85,0.4); }

.error { font-size: 0.73rem; color: #ff453a; }

.footer { display: flex; justify-content: flex-end; gap: 8px; }

.btn-cancel {
  padding: 8px 14px;
  border-radius: 8px;
  background: transparent;
  border: 0.5px solid rgba(255,255,255,0.1);
  color: #525252;
  font-size: 0.76rem;
  cursor: pointer;
  transition: color 0.15s;
}
.btn-cancel:hover { color: #8E8E93; }
</style>
