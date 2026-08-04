<script setup>
import { ref, onMounted } from 'vue'

const user = ref(null)
const loading = ref(true)
const section = ref('dashboard')

// --- Account state ---
const profile = ref(null)
const editName = ref('')
const savingName = ref(false)
const addingPasskey = ref(false)
const newPasskeyName = ref('')

// --- Admin state ---
const adminTab = ref('users')
const adminUsers = ref([])
const adminSessions = ref([])
const adminInvites = ref([])
const inviteEmail = ref('')
const inviteLoading = ref(false)

const services = [
  { name: 'TPHome', description: 'Home automation', url: 'https://tphome.migueltaibo.com' },
  { name: 'Portfolio', description: 'migueltaibo.com', url: 'https://migueltaibo.com' },
  { name: 'Login', description: 'Auth · Passkeys', url: 'https://login.migueltaibo.com' },
]

// ── Auth ──────────────────────────────────────────────────

onMounted(async () => {
  try {
    const res = await fetch('/auth/passkey/me')
    if (res.ok) {
      user.value = await res.json()
    } else {
      window.location.href = 'https://login.migueltaibo.com/?redirect=cloud.migueltaibo.com'
    }
  } catch {
    window.location.href = 'https://login.migueltaibo.com/?redirect=cloud.migueltaibo.com'
  } finally {
    loading.value = false
  }
})

async function logout() {
  await fetch('/auth/passkey/logout', { method: 'POST' })
  window.location.href = 'https://login.migueltaibo.com'
}

// ── Navigation ────────────────────────────────────────────

async function goTo(s) {
  section.value = s
  if (s === 'account') await loadProfile()
  if (s === 'admin') await loadAdminData()
}

// ── Account ───────────────────────────────────────────────

async function loadProfile() {
  const res = await fetch('/auth/account/profile')
  if (res.ok) {
    profile.value = await res.json()
    editName.value = profile.value.display_name
  }
}

async function saveName() {
  if (!editName.value.trim() || savingName.value) return
  savingName.value = true
  const res = await fetch('/auth/account/profile', {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ display_name: editName.value.trim() }),
  })
  if (res.ok) {
    const data = await res.json()
    profile.value.display_name = data.display_name
    user.value.display_name = data.display_name
  }
  savingName.value = false
}

async function deletePasskey(credentialId) {
  if (!confirm('¿Eliminar esta passkey?')) return
  const res = await fetch(`/auth/account/passkey/${encodeURIComponent(credentialId)}`, { method: 'DELETE' })
  if (res.ok) await loadProfile()
  else alert((await res.json()).detail)
}

async function addPasskey() {
  if (addingPasskey.value) return
  addingPasskey.value = true
  try {
    const optsRes = await fetch('/auth/account/passkey/add/begin', { method: 'POST' })
    if (!optsRes.ok) throw new Error((await optsRes.json()).detail)
    const cred = await navigator.credentials.create({
      publicKey: PublicKeyCredential.parseCreationOptionsFromJSON(await optsRes.json())
    })
    const body = cred.toJSON()
    body.device_name = newPasskeyName.value.trim() || navigator.userAgent.split(')')[0].split('(')[1] || 'nuevo dispositivo'
    const res = await fetch('/auth/account/passkey/add/complete', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })
    if (!res.ok) throw new Error((await res.json()).detail)
    newPasskeyName.value = ''
    await loadProfile()
  } catch (e) {
    if (e.name !== 'NotAllowedError') alert(e.message)
  } finally {
    addingPasskey.value = false
  }
}

async function revokeOwnSession(sessionId) {
  const res = await fetch(`/auth/account/session/${sessionId}`, { method: 'DELETE' })
  if (res.ok) await loadProfile()
}

// ── Admin ─────────────────────────────────────────────────

async function loadAdminData() {
  const [uRes, sRes, iRes] = await Promise.all([
    fetch('/auth/admin/users'),
    fetch('/auth/admin/sessions'),
    fetch('/auth/admin/invites'),
  ])
  if (uRes.ok) adminUsers.value = await uRes.json()
  if (sRes.ok) adminSessions.value = await sRes.json()
  if (iRes.ok) adminInvites.value = await iRes.json()
}

async function toggleAdmin(userId) {
  const res = await fetch(`/auth/admin/users/${userId}`, { method: 'PATCH' })
  if (res.ok) await loadAdminData()
}

async function deleteUser(userId, email) {
  if (!confirm(`¿Eliminar a ${email}? Se borrarán sus passkeys y sesiones.`)) return
  const res = await fetch(`/auth/admin/users/${userId}`, { method: 'DELETE' })
  if (res.ok) await loadAdminData()
}

async function revokeSession(sessionId) {
  const res = await fetch(`/auth/admin/sessions/${sessionId}`, { method: 'DELETE' })
  if (res.ok) await loadAdminData()
}

async function sendInvite() {
  if (!inviteEmail.value.trim() || inviteLoading.value) return
  inviteLoading.value = true
  const res = await fetch('/auth/admin/invites', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email: inviteEmail.value.trim() }),
  })
  if (res.ok) { inviteEmail.value = ''; await loadAdminData() }
  else alert((await res.json()).detail)
  inviteLoading.value = false
}

async function revokeInvite(email) {
  const res = await fetch(`/auth/admin/invites/${encodeURIComponent(email)}`, { method: 'DELETE' })
  if (res.ok) await loadAdminData()
}

// ── Helpers ───────────────────────────────────────────────

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
  <div class="page">
    <div v-if="!loading && user" class="container">

      <!-- Header -->
      <header class="header">
        <p class="label">TPCloud</p>
        <nav class="nav">
          <button class="nav-link" :class="{ active: section === 'dashboard' }" @click="goTo('dashboard')">Dashboard</button>
          <button class="nav-link" :class="{ active: section === 'account' }" @click="goTo('account')">Cuenta</button>
          <button v-if="user.is_admin" class="nav-link" :class="{ active: section === 'admin' }" @click="goTo('admin')">Admin</button>
        </nav>
        <button class="logout" @click="logout">Cerrar sesión</button>
      </header>

      <!-- ── DASHBOARD ── -->
      <template v-if="section === 'dashboard'">
        <div class="hero">
          <h1>Hola, {{ user.display_name }}.</h1>
          <p class="muted">{{ user.email }}</p>
        </div>
        <div class="section">
          <p class="section-label">Servicios</p>
          <div class="grid">
            <a v-for="s in services" :key="s.name" :href="s.url" class="card">
              <p class="card-name">{{ s.name }}</p>
              <p class="card-desc">{{ s.description }}</p>
            </a>
          </div>
        </div>
      </template>

      <!-- ── CUENTA ── -->
      <template v-else-if="section === 'account' && profile">

        <div class="section">
          <p class="section-label">Perfil</p>
          <div class="block">
            <div class="row">
              <span class="row-label">Email</span>
              <span class="muted">{{ profile.email }}</span>
            </div>
            <div class="row">
              <span class="row-label">Nombre</span>
              <div class="row-end">
                <input v-model="editName" class="inline-input" @keydown.enter="saveName" />
                <button v-if="editName !== profile.display_name" class="btn-sm" @click="saveName" :disabled="savingName">
                  {{ savingName ? '…' : 'Guardar' }}
                </button>
              </div>
            </div>
          </div>
        </div>

        <div class="section">
          <p class="section-label">Passkeys</p>
          <div class="block">
            <div v-for="p in profile.passkeys" :key="p.credential_id" class="row">
              <div>
                <p class="text-sm">{{ p.device_name }}</p>
                <p class="muted text-xs">{{ p.last_used_at ? 'Usado ' + relativeTime(p.last_used_at) : 'Nunca usado' }}</p>
              </div>
              <button
                class="btn-danger"
                :disabled="profile.passkeys.length <= 1"
                @click="deletePasskey(p.credential_id)"
              >Eliminar</button>
            </div>
          </div>
          <div class="add-passkey">
            <input v-model="newPasskeyName" class="inline-input" placeholder="Nombre del dispositivo (opcional)" />
            <button class="btn-sm" @click="addPasskey" :disabled="addingPasskey">
              {{ addingPasskey ? '…' : '+ Añadir passkey' }}
            </button>
          </div>
        </div>

        <div class="section">
          <p class="section-label">Sesiones activas</p>
          <div class="block">
            <div v-for="s in profile.sessions" :key="s.session_id" class="row">
              <div>
                <p class="text-sm">{{ s.ip_address || 'IP desconocida' }} <span v-if="s.current" class="badge">actual</span></p>
                <p class="muted text-xs">Expira en {{ expiresIn(s.expires_at) }} · Creada {{ relativeTime(s.created_at) }}</p>
              </div>
              <button class="btn-danger" @click="revokeOwnSession(s.session_id)">Revocar</button>
            </div>
          </div>
        </div>

      </template>

      <!-- ── ADMIN ── -->
      <template v-else-if="section === 'admin'">

        <div class="tabs">
          <button class="tab" :class="{ active: adminTab === 'users' }" @click="adminTab = 'users'">Usuarios</button>
          <button class="tab" :class="{ active: adminTab === 'invites' }" @click="adminTab = 'invites'">Invitaciones</button>
          <button class="tab" :class="{ active: adminTab === 'sessions' }" @click="adminTab = 'sessions'">Sesiones</button>
        </div>

        <!-- Usuarios -->
        <div v-if="adminTab === 'users'" class="section">
          <div class="block">
            <div v-for="u in adminUsers" :key="u.id" class="row">
              <div>
                <p class="text-sm">{{ u.display_name }} <span v-if="u.is_admin" class="badge">admin</span></p>
                <p class="muted text-xs">{{ u.email }} · {{ u.passkey_count }} passkey{{ u.passkey_count !== 1 ? 's' : '' }} · {{ u.active_session_count }} sesión{{ u.active_session_count !== 1 ? 'es' : '' }}</p>
              </div>
              <div class="row-actions" v-if="u.id !== user.id">
                <button class="btn-sm" @click="toggleAdmin(u.id)">{{ u.is_admin ? 'Quitar admin' : 'Hacer admin' }}</button>
                <button class="btn-danger" @click="deleteUser(u.id, u.email)">Eliminar</button>
              </div>
            </div>
          </div>
        </div>

        <!-- Invitaciones -->
        <div v-else-if="adminTab === 'invites'" class="section">
          <div class="add-passkey">
            <input v-model="inviteEmail" class="inline-input" placeholder="email@ejemplo.com" type="email" @keydown.enter="sendInvite" />
            <button class="btn-sm" @click="sendInvite" :disabled="inviteLoading">
              {{ inviteLoading ? '…' : 'Invitar' }}
            </button>
          </div>
          <div class="block">
            <div v-for="i in adminInvites" :key="i.email" class="row">
              <div>
                <p class="text-sm">{{ i.email }} <span class="badge" :class="i.used ? 'badge-used' : ''">{{ i.used ? 'usada' : 'pendiente' }}</span></p>
                <p class="muted text-xs">{{ relativeTime(i.created_at) }}</p>
              </div>
              <button v-if="!i.used" class="btn-danger" @click="revokeInvite(i.email)">Revocar</button>
            </div>
          </div>
        </div>

        <!-- Sesiones -->
        <div v-else-if="adminTab === 'sessions'" class="section">
          <div class="block">
            <div v-for="s in adminSessions" :key="s.session_id" class="row">
              <div>
                <p class="text-sm">{{ s.user_email }}</p>
                <p class="muted text-xs">{{ s.ip_address || 'IP desconocida' }} · Expira en {{ expiresIn(s.expires_at) }}</p>
              </div>
              <button class="btn-danger" @click="revokeSession(s.session_id)">Revocar</button>
            </div>
          </div>
        </div>

      </template>

    </div>
  </div>
</template>

<style>
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
html, body, #app { height: 100%; background: #000; color: #fff; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif; }
::-webkit-scrollbar { display: none; }
* { -ms-overflow-style: none; scrollbar-width: none; }
</style>

<style scoped>
.page { min-height: 100dvh; display: flex; justify-content: center; padding: 3rem 1.5rem; }
.container { width: 100%; max-width: 640px; display: flex; flex-direction: column; gap: 2.5rem; }

.header { display: flex; align-items: center; gap: 1rem; flex-wrap: wrap; }
.label { font-size: 0.75rem; font-weight: 500; color: #737373; text-transform: uppercase; letter-spacing: 0.1em; margin-right: auto; }
.nav { display: flex; gap: 0.25rem; }
.nav-link { background: none; border: none; color: #737373; font-size: 0.82rem; cursor: pointer; padding: 0.3rem 0.6rem; border-radius: 0.375rem; transition: color 0.2s, background 0.2s; }
.nav-link:hover { color: #fff; }
.nav-link.active { color: #fff; background: #1a1a1a; }
.logout { background: none; border: none; color: #737373; font-size: 0.8rem; cursor: pointer; transition: color 0.2s; }
.logout:hover { color: #fff; }

.hero { display: flex; flex-direction: column; gap: 0.4rem; }
h1 { font-size: 2rem; font-weight: 600; letter-spacing: -0.02em; }
.muted { color: #737373; font-size: 0.875rem; }

.section { display: flex; flex-direction: column; gap: 0.75rem; }
.section-label { font-size: 0.75rem; font-weight: 500; color: #737373; text-transform: uppercase; letter-spacing: 0.1em; }

.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 1rem; }
.card { border: 1px solid #262626; border-radius: 0.5rem; padding: 1rem; text-decoration: none; transition: border-color 0.3s; display: flex; flex-direction: column; gap: 0.25rem; }
.card:hover { border-color: #525252; }
.card-name { font-size: 0.95rem; font-weight: 500; color: #fff; }
.card-desc { font-size: 0.8rem; color: #737373; }

.block { border: 1px solid #262626; border-radius: 0.5rem; overflow: hidden; }
.row { display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding: 0.75rem 1rem; border-bottom: 1px solid #1a1a1a; }
.row:last-child { border-bottom: none; }
.row-label { font-size: 0.82rem; color: #737373; min-width: 4rem; }
.row-end { display: flex; align-items: center; gap: 0.5rem; flex: 1; justify-content: flex-end; }
.row-actions { display: flex; gap: 0.5rem; }
.text-sm { font-size: 0.875rem; color: #e4e4e7; }
.text-xs { font-size: 0.75rem; }

.inline-input { background: #000; border: 1px solid #262626; border-radius: 0.375rem; padding: 0.35rem 0.6rem; color: #fff; font-size: 0.875rem; outline: none; transition: border-color 0.2s; min-width: 0; flex: 1; }
.inline-input:focus { border-color: #525252; }

.add-passkey { display: flex; gap: 0.5rem; align-items: center; }

.btn-sm { background: none; border: 1px solid #404040; border-radius: 0.375rem; color: #fff; font-size: 0.8rem; padding: 0.3rem 0.6rem; cursor: pointer; transition: border-color 0.2s; white-space: nowrap; }
.btn-sm:hover:not(:disabled) { border-color: #737373; }
.btn-sm:disabled { opacity: 0.4; cursor: not-allowed; }

.btn-danger { background: none; border: none; color: #737373; font-size: 0.8rem; cursor: pointer; transition: color 0.2s; white-space: nowrap; }
.btn-danger:hover:not(:disabled) { color: #fca5a5; }
.btn-danger:disabled { opacity: 0.3; cursor: not-allowed; }

.badge { font-size: 0.65rem; font-weight: 500; text-transform: uppercase; letter-spacing: 0.05em; padding: 0.15rem 0.4rem; border-radius: 0.25rem; background: #1a1a1a; color: #737373; margin-left: 0.4rem; }
.badge-used { color: #86efac; background: rgba(34,197,94,0.08); }

.tabs { display: flex; gap: 0.25rem; border-bottom: 1px solid #1a1a1a; padding-bottom: 0.75rem; }
.tab { background: none; border: none; color: #737373; font-size: 0.85rem; cursor: pointer; padding: 0.3rem 0.6rem; border-radius: 0.375rem; transition: color 0.2s, background 0.2s; }
.tab:hover { color: #fff; }
.tab.active { color: #fff; background: #1a1a1a; }
</style>
