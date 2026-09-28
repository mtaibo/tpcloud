<script setup>
import { ref, onMounted } from 'vue'
import { Lock, Save, Play, Trash2, Plus } from 'lucide-vue-next'
import ExtensionSetupModal from '../components/ExtensionSetupModal.vue'

defineProps({ player: Object, linkedAccount: Object, isOwnAccount: Boolean, user: Object })

const loadout = ref(null)
const presets = ref([])
const loading = ref(true)
const error = ref('')
const showCredentials = ref(false)
const showNameModal = ref(false)
const newPresetName = ref('')
const applying = ref(null)

async function load() {
  loading.value = true
  error.value = ''
  try {
    const [l, p] = await Promise.all([
      fetch('/api/val/loadout/current'),
      fetch('/api/val/loadout/presets'),
    ])
    if (l.ok) loadout.value = await l.json()
    if (p.ok) presets.value = (await p.json()).presets
  } catch { error.value = 'Network error' } finally { loading.value = false }
}

async function savePreset() {
  const name = newPresetName.value.trim()
  if (!name) return
  await fetch('/api/val/loadout/presets', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name, payload: loadout.value }),
  })
  newPresetName.value = ''
  showNameModal.value = false
  await load()
}

async function applyPreset(id) {
  applying.value = id
  try {
    const r = await fetch(`/api/val/loadout/presets/${id}/apply`, { method: 'POST' })
    if (r.ok) {
      const data = await r.json()
      loadout.value = data.loadout
    } else {
      const d = await r.json().catch(() => ({}))
      error.value = d.detail || 'Failed to apply'
    }
  } finally { applying.value = null }
}

async function deletePreset(id) {
  await fetch(`/api/val/loadout/presets/${id}`, { method: 'DELETE' })
  presets.value = presets.value.filter(p => p.id !== id)
}

function onCredentialsSaved() { showCredentials.value = false; load() }

onMounted(load)
</script>

<template>
  <div class="view">
    <div class="view-header">
      <h1 class="view-title">Loadout Builder</h1>
      <button v-if="loadout && !loadout.requires_credentials" class="btn-primary" @click="showNameModal = true">
        <Save :size="14" /> Guardar como preset
      </button>
    </div>

    <div v-if="!loading && loadout?.requires_credentials" class="cta-card">
      <Lock :size="28" color="#FF4655" />
      <h3 class="cta-title">Conecta tu cuenta Riot</h3>
      <button class="btn-primary" @click="showCredentials = true">Connect</button>
    </div>

    <section v-else class="content">
      <h2 class="section-title">Presets guardados</h2>
      <div v-if="presets.length" class="presets">
        <div v-for="p in presets" :key="p.id" class="preset-row">
          <span class="preset-name">{{ p.name }}</span>
          <span class="preset-date">{{ new Date(p.updated_at).toLocaleDateString() }}</span>
          <button class="btn-mini" :disabled="applying === p.id" @click="applyPreset(p.id)">
            <Play :size="12" /> {{ applying === p.id ? 'Aplicando…' : 'Aplicar' }}
          </button>
          <button class="btn-mini danger" @click="deletePreset(p.id)"><Trash2 :size="12" /></button>
        </div>
      </div>
      <p v-else class="empty">Guarda tu loadout actual como preset para poder aplicarlo con 1 click.</p>

      <h2 class="section-title mt">Loadout actual del juego</h2>
      <p class="hint">Se muestra el payload crudo de Riot. Para editar arma por arma, entra en Valorant y ajústalo desde Collection — luego pulsa "Guardar como preset" para conservar la combinación.</p>
      <pre v-if="loadout && !loadout.requires_credentials" class="raw">{{ JSON.stringify(loadout, null, 2) }}</pre>
    </section>

    <div v-if="showNameModal" class="modal-backdrop" @click.self="showNameModal = false">
      <div class="modal">
        <p class="modal-title">Nombre del preset</p>
        <input v-model="newPresetName" placeholder="Ej. Full Reaver" class="modal-input" @keydown.enter="savePreset" autofocus />
        <div class="modal-footer">
          <button class="btn-mini" @click="showNameModal = false">Cancelar</button>
          <button class="btn-primary" :disabled="!newPresetName.trim()" @click="savePreset">Guardar</button>
        </div>
      </div>
    </div>

    <p v-if="error" class="error-banner">{{ error }}</p>
    <ExtensionSetupModal v-if="showCredentials" @close="showCredentials = false" @saved="onCredentialsSaved" />
  </div>
</template>

<style scoped>
.view { flex: 1; overflow-y: auto; }
.view-header { display: flex; align-items: center; justify-content: space-between; padding: 32px 32px 24px; }
.view-title { font-size: 1.5rem; font-weight: 700; letter-spacing: -0.02em; }

.content { padding: 0 32px 32px; }
.section-title { font-size: 0.85rem; font-weight: 600; color: #8E8E93; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px; }
.section-title.mt { margin-top: 28px; }

.presets { display: flex; flex-direction: column; gap: 6px; }
.preset-row { display: flex; align-items: center; gap: 10px; padding: 10px 14px; background: rgba(255,255,255,0.03); border: 0.5px solid rgba(255,255,255,0.06); border-radius: 9px; }
.preset-name { flex: 1; font-size: 0.85rem; font-weight: 500; color: #d1d1d6; }
.preset-date { font-size: 0.72rem; color: #636366; }

.hint { font-size: 0.78rem; color: #636366; line-height: 1.5; margin-bottom: 12px; max-width: 640px; }
.raw { padding: 16px; background: #050505; border: 0.5px solid rgba(255,255,255,0.06); border-radius: 10px; color: #8E8E93; font-family: monospace; font-size: 0.72rem; line-height: 1.4; overflow-x: auto; max-height: 400px; }

.empty { font-size: 0.85rem; color: #636366; padding: 20px 0; }

.btn-primary { display: inline-flex; align-items: center; gap: 6px; padding: 7px 14px; border-radius: 8px; background: #FF4655; border: none; color: #fff; font-size: 0.78rem; font-weight: 600; cursor: pointer; }
.btn-primary:disabled { opacity: 0.4; }
.btn-mini { display: inline-flex; align-items: center; gap: 4px; padding: 5px 10px; border-radius: 6px; background: rgba(255,255,255,0.06); border: 0.5px solid rgba(255,255,255,0.1); color: #d1d1d6; font-size: 0.72rem; cursor: pointer; transition: background 0.15s; }
.btn-mini:hover { background: rgba(255,255,255,0.12); }
.btn-mini.danger:hover { background: rgba(255,69,58,0.15); color: #ff453a; }

.cta-card { margin: 0 32px; padding: 40px; border-radius: 14px; background: rgba(255,255,255,0.03); display: flex; flex-direction: column; align-items: center; gap: 12px; text-align: center; }
.cta-title { font-size: 1.125rem; font-weight: 600; }

.modal-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.7); display: flex; align-items: center; justify-content: center; z-index: 9000; }
.modal { width: 340px; padding: 20px; background: #111113; border: 0.5px solid rgba(255,255,255,0.1); border-radius: 14px; display: flex; flex-direction: column; gap: 12px; }
.modal-title { font-size: 0.9rem; font-weight: 600; }
.modal-input { padding: 10px 12px; border-radius: 8px; background: rgba(255,255,255,0.05); border: 0.5px solid rgba(255,255,255,0.1); color: #fff; font-size: 0.85rem; outline: none; }
.modal-input:focus { border-color: rgba(255,70,85,0.4); }
.modal-footer { display: flex; justify-content: flex-end; gap: 8px; }

.error-banner { padding: 12px 32px; font-size: 0.82rem; color: #ff453a; }
</style>
