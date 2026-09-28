<script setup>
import { ref, onUnmounted } from 'vue'
import { Lock, ExternalLink } from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const emit = defineEmits(['close', 'saved'])

const OAUTH_URL =
  'https://auth.riotgames.com/authorize' +
  '?client_id=play-valorant-web-prod' +
  '&nonce=1' +
  '&redirect_uri=https%3A%2F%2Fval.migueltaibo.com%2Fauth%2Fcallback' +
  '&response_type=token%20id_token' +
  '&scope=openid' +
  '&language=en_US'

const state = ref('idle') // idle | waiting | error
const errorMsg = ref('')

let popup = null
let pollInterval = null
let messageHandler = null

function cleanup() {
  if (pollInterval) { clearInterval(pollInterval); pollInterval = null }
  if (messageHandler) { window.removeEventListener('message', messageHandler); messageHandler = null }
  if (popup && !popup.closed) popup.close()
  popup = null
}

onUnmounted(cleanup)

function startLogin() {
  cleanup()
  errorMsg.value = ''
  state.value = 'waiting'

  popup = window.open(OAUTH_URL, 'riot-auth', 'width=500,height=700')

  messageHandler = (event) => {
    if (!event.data || event.data.type !== 'riot-auth') return
    cleanup()
    if (event.data.success) {
      emit('saved')
    } else {
      state.value = 'error'
      errorMsg.value = event.data.error === 'no_token'
        ? 'No se recibió token — asegúrate de completar el login'
        : (event.data.error || 'Error de autenticación')
    }
  }
  window.addEventListener('message', messageHandler)

  pollInterval = setInterval(() => {
    if (popup && popup.closed) {
      cleanup()
      if (state.value === 'waiting') {
        state.value = 'error'
        errorMsg.value = 'Ventana cerrada antes de completar el login'
      }
    }
  }, 500)
}
</script>

<template>
  <BaseModal width="380px" @close="emit('close')">

    <div class="modal-header">
      <Lock :size="18" color="#FF4655" style="flex-shrink:0" />
      <div>
        <p class="modal-title">Conectar Riot</p>
        <span class="modal-subtitle">Necesario para ver tu tienda e inventario</span>
      </div>
    </div>

    <div class="modal-body">

      <template v-if="state === 'idle'">
        <p class="info-text">Inicia sesión con tu cuenta de Riot para acceder a tu tienda diaria y tu inventario de skins.</p>
        <button class="btn-riot" @click="startLogin">
          <ExternalLink :size="14" />
          Iniciar sesión con Riot
        </button>
      </template>

      <template v-else-if="state === 'waiting'">
        <div class="waiting-state">
          <div class="spinner" />
          <p class="waiting-title">Esperando autenticación de Riot…</p>
          <p class="waiting-sub">Completa el login en la ventana que se ha abierto</p>
        </div>
        <button class="btn-secondary" @click="cleanup(); state = 'idle'">Cancelar</button>
      </template>

      <template v-else-if="state === 'error'">
        <div class="error-box">
          <p class="error-msg">{{ errorMsg }}</p>
        </div>
        <button class="btn-riot" @click="startLogin">
          <ExternalLink :size="14" />
          Intentar de nuevo
        </button>
      </template>

    </div>

    <div class="modal-footer">
      <button class="btn-cancel" @click="emit('close')">Cerrar</button>
    </div>

  </BaseModal>
</template>

<style scoped>
.modal-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 16px 16px 0;
}
.modal-title { font-size: 0.875rem; font-weight: 600; }
.modal-subtitle { font-size: 0.7rem; color: #636366; }

.modal-body {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.modal-footer {
  padding: 0 16px 16px;
  display: flex;
  justify-content: flex-end;
}

.info-text {
  font-size: 0.78rem;
  color: #636366;
  line-height: 1.55;
}

.btn-riot {
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
  transition: opacity 0.15s;
}
.btn-riot:hover { opacity: 0.88; }

.waiting-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 16px 0 8px;
  text-align: center;
}
.spinner {
  width: 28px;
  height: 28px;
  border: 2.5px solid rgba(255,255,255,0.1);
  border-top-color: #FF4655;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
.waiting-title { font-size: 0.85rem; font-weight: 600; }
.waiting-sub { font-size: 0.75rem; color: #636366; }

.btn-secondary {
  width: 100%;
  padding: 8px;
  border-radius: 8px;
  background: transparent;
  border: 0.5px solid rgba(255,255,255,0.1);
  color: #525252;
  font-size: 0.78rem;
  cursor: pointer;
  transition: color 0.15s;
}
.btn-secondary:hover { color: #8E8E93; }

.error-box {
  padding: 10px 12px;
  background: rgba(255, 69, 58, 0.08);
  border: 0.5px solid rgba(255, 69, 58, 0.2);
  border-radius: 8px;
}
.error-msg { font-size: 0.78rem; color: #ff453a; line-height: 1.4; }

.btn-cancel {
  padding: 7px 16px;
  border-radius: 8px;
  background: transparent;
  border: 0.5px solid rgba(255,255,255,0.1);
  color: #525252;
  font-size: 0.78rem;
  cursor: pointer;
  transition: color 0.15s;
}
.btn-cancel:hover { color: #8E8E93; }
</style>
