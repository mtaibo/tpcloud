<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { DatabaseBackup, X, RefreshCw, CheckCircle, AlertCircle, Loader } from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const emit = defineEmits(['close'])

const status = ref(null)
const error = ref(null)
let pollTimer = null

async function fetchStatus() {
  try {
    const res = await fetch('/api/backup/status')
    if (res.ok) status.value = await res.json()
  } catch {
    // ignore
  }
}

async function runBackup() {
  error.value = null
  const res = await fetch('/api/backup/run', { method: 'POST' })
  if (res.ok) {
    await fetchStatus()
  } else {
    const data = await res.json().catch(() => ({}))
    error.value = data.detail ?? 'Error al lanzar el backup'
  }
}

function formatDate(iso) {
  if (!iso) return '—'
  return new Date(iso).toLocaleString('es-ES', { dateStyle: 'medium', timeStyle: 'short' })
}

onMounted(() => {
  fetchStatus()
  pollTimer = setInterval(fetchStatus, 3000)
})
onUnmounted(() => clearInterval(pollTimer))
</script>

<template>
  <BaseModal @close="emit('close')">
    <div class="modal-header">
      <DatabaseBackup class="header-icon" />
      <div class="header-text">
        <span class="modal-title">Backup</span>
      </div>
      <button class="close-btn" @click="emit('close')"><X class="close-icon" /></button>
    </div>

    <div class="modal-body">
      <div v-if="!status" class="backup-loading">
        <Loader class="spin" />
      </div>

      <template v-else>
        <div class="status-row">
          <Loader v-if="status.running" class="spin status-icon status-icon--running" />
          <CheckCircle v-else-if="status.last_exit_code === 0" class="status-icon status-icon--ok" />
          <AlertCircle v-else-if="status.last_exit_code !== null" class="status-icon status-icon--err" />
          <DatabaseBackup v-else class="status-icon status-icon--none" />

          <div class="status-text">
            <span v-if="status.running" class="status-label">En curso…</span>
            <span v-else-if="status.last_exit_code === 0" class="status-label status-label--ok">Completado</span>
            <span v-else-if="status.last_exit_code !== null" class="status-label status-label--err">Error ({{ status.last_exit_code }})</span>
            <span v-else class="status-label status-label--none">Sin backups todavía</span>
            <span v-if="status.last_run" class="status-date">Último: {{ formatDate(status.last_run) }}</span>
          </div>
        </div>

        <p v-if="error" class="backup-error">{{ error }}</p>
      </template>
    </div>

    <div class="modal-footer">
      <p class="backup-hint">Automático cada domingo a las 2:00 AM</p>
      <button class="run-btn" :disabled="!status || status.running" @click="runBackup">
        <RefreshCw class="btn-icon" :class="{ spin: status?.running }" />
        {{ status?.running ? 'En curso…' : 'Lanzar ahora' }}
      </button>
    </div>
  </BaseModal>
</template>

<style scoped>
.modal-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 1rem 1rem 0.75rem;
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.08);
}

.header-icon { width: 20px; height: 20px; color: #8E8E93; flex-shrink: 0; }

.header-text { flex: 1; }

.modal-title { font-size: 0.9375rem; font-weight: 600; color: #fff; }

.close-btn {
  display: flex; align-items: center; justify-content: center;
  padding: 4px; background: none; border: none; border-radius: 6px;
  cursor: default; transition: background 0.15s;
}
.close-btn:hover { background: rgba(255,255,255,0.08); }
.close-icon { width: 16px; height: 16px; color: #636366; }

.modal-body { padding: 1rem; min-width: 280px; }

.backup-loading { display: flex; justify-content: center; padding: 1.5rem 0; }

.status-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem;
  background: rgba(255,255,255,0.04);
  border-radius: 10px;
}

.status-icon { width: 20px; height: 20px; flex-shrink: 0; }
.status-icon--running { color: #FFD60A; }
.status-icon--ok { color: #30D158; }
.status-icon--err { color: #FF453A; }
.status-icon--none { color: #48484a; }

.status-text { display: flex; flex-direction: column; gap: 0.15rem; }

.status-label { font-size: 0.875rem; font-weight: 600; color: #fff; }
.status-label--ok { color: #30D158; }
.status-label--err { color: #FF453A; }
.status-label--none { color: #8E8E93; font-weight: 500; }

.status-date { font-size: 0.75rem; color: #636366; }

.backup-error {
  margin-top: 0.75rem;
  font-size: 0.8rem;
  color: #FF453A;
  padding: 0.5rem 0.75rem;
  background: rgba(255, 69, 58, 0.1);
  border-radius: 8px;
}

.modal-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.75rem 1rem 1rem;
  border-top: 0.5px solid rgba(255, 255, 255, 0.08);
}

.backup-hint { font-size: 0.7rem; color: #48484a; }

.run-btn {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.5rem 0.875rem;
  background: #2c2c2e;
  border: none;
  border-radius: 8px;
  font-size: 0.8125rem;
  font-weight: 600;
  color: #fff;
  cursor: default;
  transition: background 0.15s, opacity 0.15s;
  white-space: nowrap;
}
.run-btn:hover:not(:disabled) { background: #3a3a3c; }
.run-btn:disabled { opacity: 0.4; }

.btn-icon { width: 14px; height: 14px; }

.spin { animation: spin 1.2s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
</style>
