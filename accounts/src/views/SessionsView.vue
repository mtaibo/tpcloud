<script setup>
import { ref, onMounted } from 'vue'

const sessions = ref([])
const loading = ref(false)

onMounted(() => loadSessions())

async function loadSessions() {
  loading.value = true
  const res = await fetch('/auth/admin/sessions')
  if (res.ok) sessions.value = await res.json()
  loading.value = false
}

async function revokeSession(sessionId) {
  const res = await fetch(`/auth/admin/sessions/${sessionId}`, { method: 'DELETE' })
  if (res.ok) await loadSessions()
}

function relativeTime(iso) {
  const diff = Date.now() - new Date(iso).getTime()
  const m = Math.floor(diff / 60000)
  if (m < 1) return 'ahora mismo'
  if (m < 60) return `hace ${m}m`
  const h = Math.floor(m / 60)
  if (h < 24) return `hace ${h}h`
  return `hace ${Math.floor(h / 24)}d`
}

function expiresIn(iso) {
  const diff = new Date(iso).getTime() - Date.now()
  const m = Math.floor(diff / 60000)
  if (m < 1) return 'expirada'
  if (m < 60) return `${m}m`
  return `${Math.floor(m / 60)}h`
}
</script>

<template>
  <div class="view">
    <div class="page-header">
      <h1>Sesiones activas</h1>
      <p class="muted">{{ sessions.length }} sesión{{ sessions.length !== 1 ? 'es' : '' }}</p>
    </div>

    <section class="section">
      <div class="block">
        <div v-if="loading" class="row">
          <span class="muted">Cargando…</span>
        </div>
        <div v-else-if="sessions.length === 0" class="row">
          <span class="muted">No hay sesiones activas</span>
        </div>
        <div v-for="s in sessions" :key="s.session_id" class="row">
          <div class="session-info">
            <p class="text-sm">
              {{ s.user_display_name }}
              <span class="muted-inline">{{ s.user_email }}</span>
            </p>
            <p class="device-line">
              <span class="badge" :class="s.auth_method === 'passkey' ? 'badge-passkey' : 'badge-password'">
                {{ s.auth_method }}
              </span>
              <span class="muted text-xs">{{ s.device_info || 'Dispositivo desconocido' }}</span>
            </p>
            <p class="muted text-xs">
              {{ s.location || s.ip_address || 'Ubicación desconocida' }}
              · Iniciada {{ relativeTime(s.created_at) }}
              · Expira en {{ expiresIn(s.expires_at) }}
            </p>
          </div>
          <button class="btn-danger" @click="revokeSession(s.session_id)">Revocar</button>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.view { display: flex; flex-direction: column; gap: 2.5rem; }
.page-header { display: flex; flex-direction: column; gap: 0.25rem; }
h1 { font-size: 1.5rem; font-weight: 600; letter-spacing: -0.02em; }
.muted { color: #737373; font-size: 0.875rem; }
.muted-inline { color: #525252; font-size: 0.8rem; margin-left: 0.35rem; }
.text-sm { font-size: 0.875rem; color: #e4e4e7; }
.text-xs { font-size: 0.75rem; }

.section { display: flex; flex-direction: column; gap: 0.75rem; }

.block { border: 1px solid #262626; border-radius: 0.5rem; overflow: hidden; }
.row { display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding: 0.75rem 1rem; border-bottom: 1px solid #1a1a1a; }
.row:last-child { border-bottom: none; }

.session-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 0.2rem; }
.device-line { display: flex; align-items: center; gap: 0.5rem; }

.btn-danger { background: none; border: none; color: #737373; font-size: 0.8rem; cursor: pointer; transition: color 0.2s; white-space: nowrap; flex-shrink: 0; }
.btn-danger:hover { color: #fca5a5; }

.badge { font-size: 0.65rem; font-weight: 500; text-transform: uppercase; letter-spacing: 0.05em; padding: 0.15rem 0.4rem; border-radius: 0.25rem; }
.badge-passkey { color: #93c5fd; background: rgba(59,130,246,0.08); }
.badge-password { color: #fcd34d; background: rgba(251,191,36,0.08); }
</style>
