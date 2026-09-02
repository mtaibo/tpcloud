<script setup>
import { ref, onMounted } from 'vue'

defineProps(['user'])

const users = ref([])
const newEmail = ref('')
const newName = ref('')
const newPassword = ref('')
const createMsg = ref(null)
const creating = ref(false)

onMounted(() => loadUsers())

async function loadUsers() {
  const res = await fetch('/auth/admin/users')
  if (res.ok) users.value = await res.json()
}

async function createUser() {
  createMsg.value = null
  if (!newEmail.value || !newPassword.value) return
  creating.value = true
  try {
    const res = await fetch('/auth/admin/users', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        email: newEmail.value.trim(),
        display_name: newName.value.trim(),
        password: newPassword.value,
      }),
    })
    const data = await res.json()
    if (!res.ok) throw new Error(data.detail)
    newEmail.value = ''
    newName.value = ''
    newPassword.value = ''
    createMsg.value = { type: 'success', text: `Usuario ${data.email} creado` }
    await loadUsers()
  } catch (e) {
    createMsg.value = { type: 'error', text: e.message }
  } finally {
    creating.value = false
  }
}

async function toggleAdmin(userId) {
  const res = await fetch(`/auth/admin/users/${userId}`, { method: 'PATCH' })
  if (res.ok) await loadUsers()
}

async function deleteUser(userId, email) {
  if (!confirm(`¿Eliminar a ${email}? Se borrarán sus passkeys y sesiones.`)) return
  const res = await fetch(`/auth/admin/users/${userId}`, { method: 'DELETE' })
  if (res.ok) await loadUsers()
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
</script>

<template>
  <div class="view">
    <div class="page-header">
      <h1>Usuarios</h1>
      <p class="muted">{{ users.length }} cuenta{{ users.length !== 1 ? 's' : '' }}</p>
    </div>

    <!-- Crear usuario -->
    <section class="section">
      <p class="section-label">Crear usuario</p>
      <div class="block">
        <div class="create-form">
          <input v-model="newEmail" type="email" class="inline-input" placeholder="email@ejemplo.com" />
          <input v-model="newName" type="text" class="inline-input" placeholder="Nombre (opcional)" />
          <input v-model="newPassword" type="password" class="inline-input" placeholder="Contraseña inicial" />
          <button class="btn-sm" @click="createUser" :disabled="creating || !newEmail || !newPassword">
            {{ creating ? '…' : 'Crear usuario' }}
          </button>
        </div>
        <p v-if="createMsg" :class="['msg', createMsg.type]" style="padding: 0 1rem 0.75rem;">{{ createMsg.text }}</p>
      </div>
    </section>

    <!-- Lista de usuarios -->
    <section class="section">
      <p class="section-label">Todos los usuarios</p>
      <div class="block">
        <div v-for="u in users" :key="u.id" class="row">
          <div class="user-info">
            <p class="text-sm">
              {{ u.display_name }}
              <span v-if="u.is_admin" class="badge badge-admin">admin</span>
              <span v-if="u.has_password" class="badge">contraseña</span>
              <span v-if="u.totp_enabled" class="badge badge-ok">2FA</span>
            </p>
            <p class="muted text-xs">
              {{ u.email }}
              · {{ u.passkey_count }} passkey{{ u.passkey_count !== 1 ? 's' : '' }}
              · {{ u.active_session_count }} sesión{{ u.active_session_count !== 1 ? 'es' : '' }}
              · {{ relativeTime(u.created_at) }}
            </p>
          </div>
          <div class="actions" v-if="u.id !== user.id">
            <button class="btn-sm" @click="toggleAdmin(u.id)">
              {{ u.is_admin ? 'Quitar admin' : 'Hacer admin' }}
            </button>
            <button class="btn-danger" @click="deleteUser(u.id, u.email)">Eliminar</button>
          </div>
          <span v-else class="muted text-xs">(tú)</span>
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
.text-sm { font-size: 0.875rem; color: #e4e4e7; }
.text-xs { font-size: 0.75rem; }

.section { display: flex; flex-direction: column; gap: 0.75rem; }
.section-label { font-size: 0.75rem; font-weight: 500; color: #737373; text-transform: uppercase; letter-spacing: 0.1em; }

.block { border: 1px solid #262626; border-radius: 0.5rem; overflow: hidden; }
.row { display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding: 0.75rem 1rem; border-bottom: 1px solid #1a1a1a; }
.row:last-child { border-bottom: none; }

.user-info { flex: 1; min-width: 0; }
.actions { display: flex; gap: 0.5rem; flex-shrink: 0; }

.inline-input { background: #000; border: 1px solid #262626; border-radius: 0.375rem; padding: 0.35rem 0.6rem; color: #fff; font-size: 0.875rem; outline: none; transition: border-color 0.2s; width: 100%; }
.inline-input:focus { border-color: #525252; }

.create-form { display: flex; flex-direction: column; gap: 0.5rem; padding: 1rem; }

.btn-sm { background: none; border: 1px solid #404040; border-radius: 0.375rem; color: #fff; font-size: 0.8rem; padding: 0.3rem 0.6rem; cursor: pointer; transition: border-color 0.2s; white-space: nowrap; }
.btn-sm:hover:not(:disabled) { border-color: #737373; }
.btn-sm:disabled { opacity: 0.4; cursor: not-allowed; }

.btn-danger { background: none; border: none; color: #737373; font-size: 0.8rem; cursor: pointer; transition: color 0.2s; white-space: nowrap; }
.btn-danger:hover { color: #fca5a5; }

.badge { font-size: 0.65rem; font-weight: 500; text-transform: uppercase; letter-spacing: 0.05em; padding: 0.15rem 0.4rem; border-radius: 0.25rem; background: #1a1a1a; color: #737373; margin-left: 0.4rem; }
.badge-admin { color: #c4b5fd; background: rgba(139,92,246,0.1); }
.badge-ok { color: #86efac; background: rgba(34,197,94,0.08); }

.msg { font-size: 0.82rem; }
.msg.error { color: #fca5a5; }
.msg.success { color: #86efac; }
</style>
