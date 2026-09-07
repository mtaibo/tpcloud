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
  { name: 'Accounts', description: 'accounts.migueltaibo.com', url: 'https://accounts.migueltaibo.com' },
  { name: 'Store', description: 'store.migueltaibo.com', url: 'https://store.migueltaibo.com' },
  { name: 'Portfolio', description: 'migueltaibo.com', url: 'https://migueltaibo.com' },
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
  if (!confirm('Delete this passkey?')) return
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
    body.device_name = newPasskeyName.value.trim() || navigator.userAgent.split(')')[0].split('(')[1] || 'new device'
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
  if (!confirm(`Delete ${email}? Their passkeys and sessions will be removed.`)) return
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
  if (m < 1) return 'just now'
  if (m < 60) return `${m}m ago`
  const h = Math.floor(m / 60)
  if (h < 24) return `${h}h ago`
  return `${Math.floor(h / 24)}d ago`
}

function expiresIn(iso) {
  const diff = new Date(iso).getTime() - Date.now()
  const m = Math.floor(diff / 60000)
  if (m < 1) return 'expired'
  if (m < 60) return `${m}m`
  return `${Math.floor(m / 60)}h`
}
</script>

<template>
  <div class="page">

    <!-- Logo: top-left corner, desktop only -->
    <div class="logo-corner">
      <svg width="28" height="28" viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
        <rect x="7" y="8.5" width="18" height="4" rx="2" fill="#fff"/>
        <circle cx="16" cy="18.5" r="3" fill="#fff"/>
        <circle cx="16" cy="25" r="2.2" fill="#fff"/>
      </svg>
    </div>

    <div v-if="!loading && user" class="container">

      <!-- Header -->
      <header class="header">
        <nav class="nav">
          <button class="nav-link" :class="{ active: section === 'dashboard' }" @click="goTo('dashboard')">Dashboard</button>
          <button class="nav-link" :class="{ active: section === 'account' }" @click="goTo('account')">Account</button>
          <button v-if="user.is_admin" class="nav-link" :class="{ active: section === 'admin' }" @click="goTo('admin')">Admin</button>
        </nav>
        <button class="logout" @click="logout">Log out</button>
      </header>

      <!-- ── DASHBOARD ── -->
      <template v-if="section === 'dashboard'">
        <div class="hero">
          <h1>Hello, {{ user.display_name }}.</h1>
          <p class="muted">{{ user.email }}</p>
        </div>
        <div class="section">
          <p class="section-label">Services</p>
          <div class="grid">
            <a v-for="s in services" :key="s.name" :href="s.url" class="card">
              <p class="card-name">{{ s.name }}</p>
              <p class="card-desc">{{ s.description }}</p>
            </a>
          </div>
        </div>
      </template>

      <!-- ── ACCOUNT ── -->
      <template v-else-if="section === 'account' && profile">

        <div class="section">
          <p class="section-label">Profile</p>
          <div class="block">
            <div class="row">
              <span class="row-label">Email</span>
              <span class="muted">{{ profile.email }}</span>
            </div>
            <div class="row">
              <span class="row-label">Name</span>
              <div class="row-end">
                <input v-model="editName" class="inline-input" @keydown.enter="saveName" />
                <button v-if="editName !== profile.display_name" class="btn-sm" @click="saveName" :disabled="savingName">
                  {{ savingName ? '…' : 'Save' }}
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
                <p class="muted text-xs">{{ p.last_used_at ? 'Used ' + relativeTime(p.last_used_at) : 'Never used' }}</p>
              </div>
              <button
                class="btn-danger"
                :disabled="profile.passkeys.length <= 1"
                @click="deletePasskey(p.credential_id)"
              >Remove</button>
            </div>
          </div>
          <div class="add-passkey">
            <input v-model="newPasskeyName" class="inline-input" placeholder="Device name (optional)" />
            <button class="btn-sm" @click="addPasskey" :disabled="addingPasskey">
              {{ addingPasskey ? '…' : '+ Add passkey' }}
            </button>
          </div>
        </div>

        <div class="section">
          <p class="section-label">Sessions</p>
          <div class="block">
            <div v-for="s in profile.sessions" :key="s.session_id" class="row">
              <div>
                <p class="text-sm">{{ s.ip_address || 'Unknown IP' }} <span v-if="s.current" class="badge">current</span></p>
                <p class="muted text-xs">Expires in {{ expiresIn(s.expires_at) }} · Created {{ relativeTime(s.created_at) }}</p>
              </div>
              <button class="btn-danger" @click="revokeOwnSession(s.session_id)">Revoke</button>
            </div>
          </div>
        </div>

      </template>

      <!-- ── ADMIN ── -->
      <template v-else-if="section === 'admin'">

        <div class="tabs">
          <button class="tab" :class="{ active: adminTab === 'users' }" @click="adminTab = 'users'">Users</button>
          <button class="tab" :class="{ active: adminTab === 'invites' }" @click="adminTab = 'invites'">Invites</button>
          <button class="tab" :class="{ active: adminTab === 'sessions' }" @click="adminTab = 'sessions'">Sessions</button>
        </div>

        <!-- Users -->
        <div v-if="adminTab === 'users'" class="section">
          <div class="block">
            <div v-for="u in adminUsers" :key="u.id" class="row">
              <div>
                <p class="text-sm">{{ u.display_name }} <span v-if="u.is_admin" class="badge">admin</span></p>
                <p class="muted text-xs">{{ u.email }} · {{ u.passkey_count }} passkey{{ u.passkey_count !== 1 ? 's' : '' }} · {{ u.active_session_count }} session{{ u.active_session_count !== 1 ? 's' : '' }}</p>
              </div>
              <div class="row-actions" v-if="u.id !== user.id">
                <button class="btn-sm" @click="toggleAdmin(u.id)">{{ u.is_admin ? 'Remove admin' : 'Make admin' }}</button>
                <button class="btn-danger" @click="deleteUser(u.id, u.email)">Delete</button>
              </div>
            </div>
          </div>
        </div>

        <!-- Invites -->
        <div v-else-if="adminTab === 'invites'" class="section">
          <div class="add-passkey">
            <input v-model="inviteEmail" class="inline-input" placeholder="email@example.com" type="email" @keydown.enter="sendInvite" />
            <button class="btn-sm" @click="sendInvite" :disabled="inviteLoading">
              {{ inviteLoading ? '…' : 'Invite' }}
            </button>
          </div>
          <div class="block">
            <div v-for="i in adminInvites" :key="i.email" class="row">
              <div>
                <p class="text-sm">{{ i.email }} <span class="badge" :class="i.used ? 'badge-used' : ''">{{ i.used ? 'used' : 'pending' }}</span></p>
                <p class="muted text-xs">{{ relativeTime(i.created_at) }}</p>
              </div>
              <button v-if="!i.used" class="btn-danger" @click="revokeInvite(i.email)">Revoke</button>
            </div>
          </div>
        </div>

        <!-- Sessions -->
        <div v-else-if="adminTab === 'sessions'" class="section">
          <div class="block">
            <div v-for="s in adminSessions" :key="s.session_id" class="row">
              <div>
                <p class="text-sm">{{ s.user_email }}</p>
                <p class="muted text-xs">{{ s.ip_address || 'Unknown IP' }} · Expires in {{ expiresIn(s.expires_at) }}</p>
              </div>
              <button class="btn-danger" @click="revokeSession(s.session_id)">Revoke</button>
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

/* Logo corner */
.logo-corner {
  display: none;
  position: fixed;
  top: 1.5rem;
  left: 1.5rem;
  opacity: 0.85;
  transition: opacity 0.2s;
}
.logo-corner:hover { opacity: 1; }
@media (min-width: 768px) {
  .logo-corner { display: block; }
}

/* Header */
.header { display: flex; align-items: center; justify-content: center; gap: 0.75rem; flex-wrap: wrap; }
.nav { display: flex; gap: 0.25rem; }
.nav-link {
  background: none;
  border: 0.5px solid transparent;
  color: #737373;
  font-size: 0.82rem;
  cursor: pointer;
  padding: 0.3rem 0.7rem;
  border-radius: 8px;
  transition: color 0.2s, background 0.2s, border-color 0.2s;
}
.nav-link:hover { color: #fff; }
.nav-link.active {
  color: #fff;
  background: rgba(255, 255, 255, 0.1);
  border-color: rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}
.logout {
  background: none;
  border: none;
  color: #525252;
  font-size: 0.8rem;
  cursor: pointer;
  transition: color 0.2s;
}
.logout:hover { color: #fff; }

/* Hero */
.hero { display: flex; flex-direction: column; gap: 0.4rem; }
h1 { font-size: 2rem; font-weight: 600; letter-spacing: -0.02em; }
.muted { color: #737373; font-size: 0.875rem; }

/* Sections */
.section { display: flex; flex-direction: column; gap: 0.75rem; }
.section-label { font-size: 0.75rem; font-weight: 500; color: #737373; text-transform: uppercase; letter-spacing: 0.1em; }

/* Service cards */
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 0.75rem; }
.card {
  border: 0.5px solid rgba(255, 255, 255, 0.14);
  border-radius: 18px;
  padding: 1rem;
  text-decoration: none;
  transition: border-color 0.2s, box-shadow 0.2s;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  background: rgba(255, 255, 255, 0.065);
  backdrop-filter: blur(24px) saturate(160%);
  -webkit-backdrop-filter: blur(24px) saturate(160%);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.35),
    inset 0 1px 0 rgba(255, 255, 255, 0.12),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15);
}
.card:hover {
  border-color: rgba(255, 255, 255, 0.25);
  box-shadow:
    0 12px 40px rgba(0, 0, 0, 0.45),
    inset 0 1px 0 rgba(255, 255, 255, 0.16),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15);
}
.card-name { font-size: 0.95rem; font-weight: 500; color: #fff; }
.card-desc { font-size: 0.8rem; color: #737373; }

/* Blocks (account / admin rows) */
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
.row-label { font-size: 0.82rem; color: #737373; min-width: 4rem; }
.row-end { display: flex; align-items: center; gap: 0.5rem; flex: 1; justify-content: flex-end; }
.row-actions { display: flex; gap: 0.5rem; }
.text-sm { font-size: 0.875rem; color: #e4e4e7; }
.text-xs { font-size: 0.75rem; }

/* Inline inputs */
.inline-input {
  background: rgba(255, 255, 255, 0.05);
  border: 0.5px solid rgba(255, 255, 255, 0.14);
  border-radius: 10px;
  padding: 0.35rem 0.6rem;
  color: #fff;
  font-size: 0.875rem;
  outline: none;
  transition: border-color 0.2s;
  min-width: 0;
  flex: 1;
}
.inline-input:focus { border-color: rgba(255, 255, 255, 0.3); }

.add-passkey { display: flex; gap: 0.5rem; align-items: center; }

/* Buttons */
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
.btn-danger:hover:not(:disabled) { color: #fca5a5; }
.btn-danger:disabled { opacity: 0.3; cursor: not-allowed; }

/* Badges */
.badge { font-size: 0.65rem; font-weight: 500; text-transform: uppercase; letter-spacing: 0.05em; padding: 0.15rem 0.4rem; border-radius: 0.25rem; background: rgba(255, 255, 255, 0.08); color: #737373; margin-left: 0.4rem; }
.badge-used { color: #86efac; background: rgba(34, 197, 94, 0.08); }

/* Admin tabs */
.tabs { display: flex; gap: 0.25rem; border-bottom: 1px solid rgba(255, 255, 255, 0.07); padding-bottom: 0.75rem; }
.tab {
  background: none;
  border: 0.5px solid transparent;
  color: #737373;
  font-size: 0.85rem;
  cursor: pointer;
  padding: 0.3rem 0.7rem;
  border-radius: 8px;
  transition: color 0.2s, background 0.2s, border-color 0.2s;
}
.tab:hover { color: #fff; }
.tab.active {
  color: #fff;
  background: rgba(255, 255, 255, 0.1);
  border-color: rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}
</style>
