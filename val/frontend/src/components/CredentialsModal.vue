<script setup>
import { ref } from 'vue'
import BaseModal from './BaseModal.vue'

const emit = defineEmits(['close', 'saved'])

const step = ref(1) // 1 = instructions, 2 = paste
const ssid = ref('')
const loading = ref(false)
const error = ref('')

function openRiot() {
  window.open('https://auth.riotgames.com', '_blank')
  step.value = 2
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

      <!-- Step 1: instructions -->
      <template v-if="step === 1">
        <p class="title">Conectar Riot</p>
        <p class="sub">Solo tienes que hacer esto una vez. Da acceso a tu tienda e inventario.</p>

        <div class="steps">
          <div class="step-row">
            <span class="num">1</span>
            <span>Pulsa el botón de abajo — se abre <strong>auth.riotgames.com</strong></span>
          </div>
          <div class="step-row">
            <span class="num">2</span>
            <span>Inicia sesión con tu cuenta de Riot si aún no lo has hecho</span>
          </div>
          <div class="step-row">
            <span class="num">3</span>
            <span>Pulsa <kbd>F12</kbd> → <strong>Application</strong> → <strong>Cookies</strong> → <code>auth.riotgames.com</code> → copia el valor de <strong>ssid</strong></span>
          </div>
        </div>

        <button class="btn-primary" @click="openRiot">Abrir Riot Auth →</button>
        <button class="btn-ghost" @click="step = 2">Ya tengo el ssid</button>
      </template>

      <!-- Step 2: paste -->
      <template v-else>
        <button class="back" @click="step = 1">← Volver</button>
        <p class="title">Pega el valor de ssid</p>
        <p class="sub">Lo encuentras en <strong>F12 → Application → Cookies → auth.riotgames.com</strong></p>

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

.title {
  font-size: 0.9rem;
  font-weight: 700;
  color: #fff;
}

.sub {
  font-size: 0.73rem;
  color: #636366;
  line-height: 1.5;
}
.sub strong { color: #8E8E93; }

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

.footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

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
