<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import BaseModal from './BaseModal.vue'

const emit = defineEmits(['close', 'saved'])

const step = ref(1) // 1 = intro/token, 2 = install ext, 3 = login+sync (polling)
const pairToken = ref('')
const loading = ref(false)
const error = ref('')
const session = ref(null)
const copied = ref(false)

let pollTimer = null

async function fetchSession() {
  try {
    const r = await fetch('/api/val/account/session')
    if (r.ok) session.value = await r.json()
  } catch { /* ignore */ }
}

async function generateToken() {
  loading.value = true
  error.value = ''
  try {
    const r = await fetch('/api/val/account/extension/pair-token', { method: 'POST' })
    const data = await r.json()
    if (!r.ok) { error.value = data.detail || 'No se pudo generar el token'; return }
    pairToken.value = data.pair_token
    step.value = 2
  } catch {
    error.value = 'Error de red'
  } finally {
    loading.value = false
  }
}

async function copyToken() {
  try {
    await navigator.clipboard.writeText(pairToken.value)
    copied.value = true
    setTimeout(() => (copied.value = false), 1500)
  } catch { /* ignore */ }
}

function startPolling() {
  step.value = 3
  pollTimer = setInterval(async () => {
    await fetchSession()
    if (session.value?.has_extension_synced && !session.value?.needs_resync) {
      clearInterval(pollTimer)
      pollTimer = null
      emit('saved')
    }
  }, 3000)
}

onMounted(fetchSession)
onUnmounted(() => { if (pollTimer) clearInterval(pollTimer) })
</script>

<template>
  <BaseModal width="440px" @close="emit('close')">
    <div class="modal">
      <!-- Step 1: Intro + generate token -->
      <template v-if="step === 1">
        <p class="title">Conectar Riot con extensión</p>
        <p class="sub">
          La extensión <strong>TPVal Sync</strong> lee tus cookies de Riot desde el navegador y las envía a tu servidor.
          Setup una sola vez; se renueva sola cada 72h y dura ~3 semanas por sesión.
        </p>

        <div class="checklist">
          <div class="check-row"><span class="num">1</span><span>Generas un <strong>pair token</strong> único</span></div>
          <div class="check-row"><span class="num">2</span><span>Instalas la extensión TPVal Sync</span></div>
          <div class="check-row"><span class="num">3</span><span>Pegas el token en el popup + login en <code>auth.riotgames.com</code></span></div>
        </div>

        <p v-if="error" class="error">{{ error }}</p>
        <button class="btn-primary" :disabled="loading" @click="generateToken">
          {{ loading ? 'Generando…' : 'Generar pair token' }}
        </button>
      </template>

      <!-- Step 2: Show token + install extension -->
      <template v-else-if="step === 2">
        <button class="back" @click="step = 1">← Volver</button>
        <p class="title">Tu pair token</p>
        <p class="sub">Copia este token. Lo pegas en el popup de la extensión.</p>

        <div class="token-box">
          <code>{{ pairToken }}</code>
          <button class="btn-mini" @click="copyToken">{{ copied ? '✓' : 'Copiar' }}</button>
        </div>

        <div class="checklist">
          <div class="check-row">
            <span class="num">1</span>
            <span>Instala <strong>TPVal Sync</strong> (Chrome dev mode: carga la carpeta <code>val/extension</code>)</span>
          </div>
          <div class="check-row">
            <span class="num">2</span>
            <span>Abre el popup de la extensión → pega el token en <strong>Pair token</strong> → Guardar</span>
          </div>
          <div class="check-row">
            <span class="num">3</span>
            <span>Ve a <a href="https://auth.riotgames.com" target="_blank" rel="noreferrer">auth.riotgames.com</a> y haz login con tu cuenta Riot</span>
          </div>
        </div>

        <button class="btn-primary" @click="startPolling">Ya lo hice, sincronizar</button>
      </template>

      <!-- Step 3: Polling for sync -->
      <template v-else>
        <button class="back" @click="step = 2; if (pollTimer) { clearInterval(pollTimer); pollTimer = null }">← Volver</button>
        <p class="title">Esperando sincronización…</p>
        <p class="sub">
          Abre el popup de la extensión y pulsa <strong>Sync now</strong>.
          En cuanto el backend reciba las cookies, esta ventana se cerrará automáticamente.
        </p>

        <div class="polling">
          <div class="spinner"></div>
          <span>Sondeando estado cada 3 segundos…</span>
        </div>

        <div v-if="session" class="status-line">
          <span :class="['dot', session.has_extension_synced ? (session.needs_resync ? 'warn' : 'ok') : 'idle']"></span>
          <span v-if="!session.has_extension_synced">Sin sincronizar aún</span>
          <span v-else-if="session.needs_resync">Sincronizada pero necesita renovar</span>
          <span v-else>Sincronizada · {{ session.days_remaining }}d restantes</span>
        </div>
      </template>
    </div>
  </BaseModal>
</template>

<style scoped>
.modal { display: flex; flex-direction: column; gap: 14px; padding: 22px; }
.title { font-size: 0.95rem; font-weight: 700; color: #fff; }
.sub { font-size: 0.78rem; color: #8E8E93; line-height: 1.55; }
.sub strong { color: #d1d1d6; }

.checklist {
  display: flex; flex-direction: column; gap: 8px;
  background: rgba(255,255,255,0.03);
  border: 0.5px solid rgba(255,255,255,0.07);
  border-radius: 10px;
  padding: 12px 14px;
}
.check-row { display: flex; align-items: flex-start; gap: 10px; font-size: 0.76rem; color: #8E8E93; line-height: 1.5; }
.check-row strong { color: #d1d1d6; }
.check-row a { color: #FF4655; text-decoration: none; }
.check-row a:hover { text-decoration: underline; }
.num {
  flex-shrink: 0; width: 20px; height: 20px;
  background: rgba(255,70,85,0.15);
  border: 0.5px solid rgba(255,70,85,0.3);
  border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  font-size: 0.65rem; font-weight: 700; color: #FF4655; margin-top: 1px;
}

code {
  background: rgba(255,255,255,0.08);
  border-radius: 3px;
  padding: 1px 5px;
  font-size: 0.72rem;
  color: #d1d1d6;
  font-family: monospace;
}

.token-box {
  display: flex; align-items: center; gap: 10px;
  background: rgba(255,70,85,0.06);
  border: 0.5px solid rgba(255,70,85,0.2);
  border-radius: 9px;
  padding: 10px 12px;
}
.token-box code {
  flex: 1;
  background: transparent;
  padding: 0;
  color: #FF4655;
  word-break: break-all;
  font-size: 0.72rem;
}

.btn-mini {
  padding: 5px 10px; border-radius: 6px;
  background: rgba(255,255,255,0.06);
  border: 0.5px solid rgba(255,255,255,0.1);
  color: #d1d1d6; font-size: 0.72rem; cursor: pointer;
  transition: background 0.15s;
}
.btn-mini:hover { background: rgba(255,255,255,0.12); }

.btn-primary {
  width: 100%; padding: 11px; border-radius: 9px;
  background: #FF4655; border: none; color: #fff;
  font-size: 0.82rem; font-weight: 700; cursor: pointer;
  transition: opacity 0.15s;
}
.btn-primary:hover:not(:disabled) { opacity: 0.88; }
.btn-primary:disabled { opacity: 0.4; cursor: default; }

.back {
  background: none; border: none; color: #525252;
  font-size: 0.72rem; cursor: pointer; padding: 0; text-align: left;
}
.back:hover { color: #8E8E93; }

.error { font-size: 0.75rem; color: #ff453a; }

.polling {
  display: flex; align-items: center; gap: 10px;
  background: rgba(255,255,255,0.03);
  border-radius: 9px; padding: 14px;
  font-size: 0.78rem; color: #8E8E93;
}
.spinner {
  width: 16px; height: 16px;
  border: 2px solid rgba(255,255,255,0.08);
  border-top-color: #FF4655;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }

.status-line { display: flex; align-items: center; gap: 8px; font-size: 0.75rem; color: #8E8E93; }
.dot { width: 8px; height: 8px; border-radius: 50%; }
.dot.ok { background: #30d158; }
.dot.warn { background: #ffd60a; }
.dot.idle { background: #636366; }
</style>
