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
    createMsg.value = { type: 'success', text: `User ${data.email} created` }
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
  if (!confirm(`Delete ${email}? Their passkeys and sessions will be removed.`)) return
  const res = await fetch(`/auth/admin/users/${userId}`, { method: 'DELETE' })
  if (res.ok) await loadUsers()
}

function relativeTime(iso) {
  const diff = Date.now() - new Date(iso).getTime()
  const m = Math.floor(diff / 60000)
  if (m < 1) return 'just now'
  if (m < 60) return `${m}m ago`
  const h = Math.floor(m / 60)
  if (h < 24) return `${h}h ago`
  return `${Math.floor(h / 24)}d ago`
}
</script>

<template>
  <div class="view">
    <div class="page-header">
      <h1>Users</h1>
      <p class="muted">{{ users.length }} account{{ users.length !== 1 ? 's' : '' }}</p>
    </div>

    <!-- Create user -->
    <section class="section">
      <p class="section-label">Create user</p>
      <div class="block">
        <div class="create-form">
          <input v-model="newEmail" type="email" class="inline-input" placeholder="email@example.com" />
          <input v-model="newName" type="text" class="inline-input" placeholder="Name (optional)" />
          <input v-model="newPassword" type="password" class="inline-input" placeholder="Initial password" />
          <button class="btn-sm" @click="createUser" :disabled="creating || !newEmail || !newPassword">
            {{ creating ? '…' : 'Create user' }}
          </button>
        </div>
        <p v-if="createMsg" :class="['msg', createMsg.type]" style="padding: 0 1rem 0.75rem;">{{ createMsg.text }}</p>
      </div>
    </section>

    <!-- User list -->
    <section class="section">
      <p class="section-label">All users</p>
      <div class="block">
        <div v-for="u in users" :key="u.id" class="row">
          <div class="user-info">
            <p class="text-sm">
              {{ u.display_name }}
              <span v-if="u.is_admin" class="badge badge-admin">admin</span>
              <span v-if="u.has_password" class="badge">password</span>
              <span v-if="u.totp_enabled" class="badge badge-ok">2FA</span>
            </p>
            <p class="muted text-xs">
              {{ u.email }}
              · {{ u.passkey_count }} passkey{{ u.passkey_count !== 1 ? 's' : '' }}
              · {{ u.active_session_count }} session{{ u.active_session_count !== 1 ? 's' : '' }}
              · {{ relativeTime(u.created_at) }}
            </p>
          </div>
          <div class="actions" v-if="u.id !== user.id">
            <button class="btn-sm" @click="toggleAdmin(u.id)">
              {{ u.is_admin ? 'Remove admin' : 'Make admin' }}
            </button>
            <button class="btn-danger" @click="deleteUser(u.id, u.email)">Delete</button>
          </div>
          <span v-else class="muted text-xs">(you)</span>
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

.block {
  border: 0.5px solid rgba(255, 255, 255, 0.12);
  border-radius: 18px;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.04);
  backdrop-filter: blur(20px) saturate(140%);
  -webkit-backdrop-filter: blur(20px) saturate(140%);
}
.row { display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding: 0.75rem 1rem; border-bottom: 1px solid rgba(255, 255, 255, 0.06); }
.row:last-child { border-bottom: none; }

.user-info { flex: 1; min-width: 0; }
.actions { display: flex; gap: 0.5rem; flex-shrink: 0; }

.inline-input {
  background: rgba(255, 255, 255, 0.05);
  border: 0.5px solid rgba(255, 255, 255, 0.14);
  border-radius: 10px;
  padding: 0.35rem 0.6rem;
  color: #fff;
  font-size: 0.875rem;
  outline: none;
  transition: border-color 0.2s;
  width: 100%;
}
.inline-input:focus { border-color: rgba(255, 255, 255, 0.3); }

.create-form { display: flex; flex-direction: column; gap: 0.5rem; padding: 1rem; }

.btn-sm {
  background: linear-gradient(145deg, rgba(255, 255, 255, 0.1) 0%, rgba(255, 255, 255, 0.05) 100%);
  border: 0.5px solid rgba(255, 255, 255, 0.18);
  border-radius: 10px;
  color: #fff;
  font-size: 0.8rem;
  padding: 0.3rem 0.7rem;
  cursor: pointer;
  transition: background 0.18s ease, box-shadow 0.18s ease;
  white-space: nowrap;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.2), 0 2px 8px rgba(0, 0, 0, 0.2);
}
.btn-sm:hover:not(:disabled) {
  background: linear-gradient(145deg, rgba(255, 255, 255, 0.16) 0%, rgba(255, 255, 255, 0.09) 100%);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.26), 0 4px 12px rgba(0, 0, 0, 0.28);
}
.btn-sm:disabled { opacity: 0.4; cursor: not-allowed; }

.btn-danger { background: none; border: none; color: #737373; font-size: 0.8rem; cursor: pointer; transition: color 0.2s; white-space: nowrap; }
.btn-danger:hover { color: #fca5a5; }

.badge { font-size: 0.65rem; font-weight: 500; text-transform: uppercase; letter-spacing: 0.05em; padding: 0.15rem 0.4rem; border-radius: 0.25rem; background: rgba(255, 255, 255, 0.08); color: #737373; margin-left: 0.4rem; }
.badge-admin { color: #c4b5fd; background: rgba(139, 92, 246, 0.1); }
.badge-ok { color: #86efac; background: rgba(34, 197, 94, 0.08); }

.msg { font-size: 0.82rem; }
.msg.error { color: #fca5a5; }
.msg.success { color: #86efac; }
</style>
