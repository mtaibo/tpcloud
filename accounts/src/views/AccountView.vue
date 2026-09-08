<script setup>
import { ref, onMounted } from 'vue'
import QRCode from 'qrcode'

const props = defineProps(['user'])
const emit = defineEmits(['user-updated'])

const profile = ref(null)
const editName = ref('')
const savingName = ref(false)

// Password
const currentPassword = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const passwordMsg = ref(null)
const savingPassword = ref(false)

// TOTP
const totpSetup = ref(null) // { secret, otpauth_uri, qrDataUrl }
const totpConfirmCode = ref('')
const totpDisableCode = ref('')
const totpMsg = ref(null)
const totpLoading = ref(false)

// Passkeys
const newPasskeyName = ref('')
const addingPasskey = ref(false)
const passkeyMsg = ref(null)

onMounted(() => loadProfile())

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
    emit('user-updated', { display_name: data.display_name })
  }
  savingName.value = false
}

async function savePassword() {
  passwordMsg.value = null
  if (newPassword.value !== confirmPassword.value) {
    passwordMsg.value = { type: 'error', text: 'Passwords do not match' }
    return
  }
  if (newPassword.value.length < 8) {
    passwordMsg.value = { type: 'error', text: 'Password must be at least 8 characters' }
    return
  }
  savingPassword.value = true
  try {
    const res = await fetch('/auth/account/password', {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        current_password: currentPassword.value,
        new_password: newPassword.value,
      }),
    })
    const data = await res.json()
    if (!res.ok) throw new Error(data.detail)
    passwordMsg.value = { type: 'success', text: 'Password updated' }
    currentPassword.value = ''
    newPassword.value = ''
    confirmPassword.value = ''
    await loadProfile()
  } catch (e) {
    passwordMsg.value = { type: 'error', text: e.message }
  } finally {
    savingPassword.value = false
  }
}

async function totpSetupBegin() {
  totpMsg.value = null
  totpLoading.value = true
  try {
    const res = await fetch('/auth/account/totp/setup/begin', { method: 'POST' })
    const data = await res.json()
    if (!res.ok) throw new Error(data.detail)
    const qrDataUrl = await QRCode.toDataURL(data.otpauth_uri)
    totpSetup.value = { ...data, qrDataUrl }
  } catch (e) {
    totpMsg.value = { type: 'error', text: e.message }
  } finally {
    totpLoading.value = false
  }
}

async function totpSetupComplete() {
  totpMsg.value = null
  totpLoading.value = true
  try {
    const res = await fetch('/auth/account/totp/setup/complete', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code: totpConfirmCode.value }),
    })
    const data = await res.json()
    if (!res.ok) throw new Error(data.detail)
    totpSetup.value = null
    totpConfirmCode.value = ''
    totpMsg.value = { type: 'success', text: '2FA enabled successfully' }
    await loadProfile()
  } catch (e) {
    totpMsg.value = { type: 'error', text: e.message }
  } finally {
    totpLoading.value = false
  }
}

async function totpDisable() {
  totpMsg.value = null
  totpLoading.value = true
  try {
    const res = await fetch('/auth/account/totp', {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code: totpDisableCode.value }),
    })
    const data = await res.json()
    if (!res.ok) throw new Error(data.detail)
    totpDisableCode.value = ''
    totpMsg.value = { type: 'success', text: '2FA disabled' }
    await loadProfile()
  } catch (e) {
    totpMsg.value = { type: 'error', text: e.message }
  } finally {
    totpLoading.value = false
  }
}

async function addPasskey() {
  if (addingPasskey.value) return
  passkeyMsg.value = null
  addingPasskey.value = true
  try {
    const optsRes = await fetch('/auth/account/passkey/add/begin', { method: 'POST' })
    if (!optsRes.ok) throw new Error((await optsRes.json()).detail)
    const cred = await navigator.credentials.create({
      publicKey: PublicKeyCredential.parseCreationOptionsFromJSON(await optsRes.json()),
    })
    const body = cred.toJSON()
    body.device_name = newPasskeyName.value.trim() || 'new device'
    const res = await fetch('/auth/account/passkey/add/complete', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })
    if (!res.ok) throw new Error((await res.json()).detail)
    newPasskeyName.value = ''
    passkeyMsg.value = { type: 'success', text: 'Passkey added successfully' }
    await loadProfile()
  } catch (e) {
    if (e.name !== 'NotAllowedError') passkeyMsg.value = { type: 'error', text: e.message }
  } finally {
    addingPasskey.value = false
  }
}

async function deletePasskey(credentialId) {
  if (!confirm('Delete this passkey?')) return
  const res = await fetch(`/auth/account/passkey/${encodeURIComponent(credentialId)}`, { method: 'DELETE' })
  if (res.ok) {
    passkeyMsg.value = { type: 'success', text: 'Passkey removed' }
    await loadProfile()
  } else {
    passkeyMsg.value = { type: 'error', text: (await res.json()).detail }
  }
}

async function revokeSession(sessionId) {
  const res = await fetch(`/auth/account/session/${sessionId}`, { method: 'DELETE' })
  if (res.ok) await loadProfile()
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

function expiresIn(iso) {
  const diff = new Date(iso).getTime() - Date.now()
  const m = Math.floor(diff / 60000)
  if (m < 1) return 'expired'
  if (m < 60) return `${m}m`
  return `${Math.floor(m / 60)}h`
}
</script>

<template>
  <div class="view" v-if="profile">

    <!-- Profile -->
    <section class="section">
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
    </section>

    <!-- Password -->
    <section class="section">
      <p class="section-label">Password</p>
      <div class="block">
        <div class="row">
          <span class="row-label">Status</span>
          <span class="muted">{{ profile.has_password ? 'Password set' : 'No password (passkey only)' }}</span>
        </div>
        <div class="row column">
          <div class="password-form">
            <input
              v-if="profile.has_password"
              v-model="currentPassword"
              type="password"
              class="inline-input"
              placeholder="Current password"
            />
            <input v-model="newPassword" type="password" class="inline-input" placeholder="New password" />
            <input v-model="confirmPassword" type="password" class="inline-input" placeholder="Confirm password" />
            <button class="btn-sm" @click="savePassword" :disabled="savingPassword || !newPassword">
              {{ savingPassword ? '…' : profile.has_password ? 'Change password' : 'Set password' }}
            </button>
          </div>
          <p v-if="passwordMsg" :class="['msg', passwordMsg.type]">{{ passwordMsg.text }}</p>
        </div>
      </div>
    </section>

    <!-- 2FA -->
    <section class="section">
      <p class="section-label">Two-factor authentication (TOTP)</p>
      <div class="block">
        <div class="row">
          <span class="row-label">Status</span>
          <span class="badge" :class="profile.totp_enabled ? 'badge-ok' : ''">
            {{ profile.totp_enabled ? 'Enabled' : 'Disabled' }}
          </span>
        </div>

        <!-- Setup flow -->
        <div v-if="!profile.totp_enabled" class="row column">
          <template v-if="!totpSetup">
            <button class="btn-sm" @click="totpSetupBegin" :disabled="totpLoading">
              {{ totpLoading ? '…' : 'Set up 2FA' }}
            </button>
          </template>
          <template v-else>
            <div class="totp-setup">
              <img :src="totpSetup.qrDataUrl" class="qr" alt="QR Code" />
              <p class="muted text-xs">Or enter the code manually: <code>{{ totpSetup.secret }}</code></p>
              <div class="row-end" style="margin-top: 0.5rem;">
                <input
                  v-model="totpConfirmCode"
                  type="text"
                  inputmode="numeric"
                  maxlength="6"
                  class="inline-input"
                  placeholder="Confirmation code"
                />
                <button class="btn-sm" @click="totpSetupComplete" :disabled="totpLoading || totpConfirmCode.length < 6">
                  {{ totpLoading ? '…' : 'Confirm' }}
                </button>
              </div>
            </div>
          </template>
        </div>

        <!-- Disable flow -->
        <div v-else class="row column">
          <div class="row-end">
            <input
              v-model="totpDisableCode"
              type="text"
              inputmode="numeric"
              maxlength="6"
              class="inline-input"
              placeholder="Current code"
            />
            <button class="btn-danger-red" @click="totpDisable" :disabled="totpLoading || totpDisableCode.length < 6">
              {{ totpLoading ? '…' : 'Disable 2FA' }}
            </button>
          </div>
        </div>

        <p v-if="totpMsg" :class="['msg', totpMsg.type]" style="padding: 0 1rem 0.75rem;">{{ totpMsg.text }}</p>
      </div>
    </section>

    <!-- Passkeys -->
    <section class="section">
      <p class="section-label">Passkeys</p>
      <div class="block">
        <div v-for="p in profile.passkeys" :key="p.credential_id" class="row">
          <div>
            <p class="text-sm">{{ p.device_name }}</p>
            <p class="muted text-xs">{{ p.last_used_at ? 'Used ' + relativeTime(p.last_used_at) : 'Never used' }}</p>
          </div>
          <button class="btn-danger" @click="deletePasskey(p.credential_id)">Remove</button>
        </div>
        <div v-if="profile.passkeys.length === 0" class="row">
          <span class="muted text-sm">No passkeys registered</span>
        </div>
      </div>
      <div class="add-row">
        <input v-model="newPasskeyName" class="inline-input" placeholder="Device name (optional)" />
        <button class="btn-sm" @click="addPasskey" :disabled="addingPasskey">
          {{ addingPasskey ? '…' : '+ Add passkey' }}
        </button>
      </div>
      <p v-if="passkeyMsg" :class="['msg', passkeyMsg.type]">{{ passkeyMsg.text }}</p>
    </section>

    <!-- Active sessions -->
    <section class="section">
      <p class="section-label">Active sessions</p>
      <div class="block">
        <div v-for="s in profile.sessions" :key="s.session_id" class="row">
          <div>
            <p class="text-sm">
              {{ s.device_info || 'Unknown device' }}
              <span v-if="s.current" class="badge badge-ok">current</span>
              <span class="badge badge-method">{{ s.auth_method }}</span>
            </p>
            <p class="muted text-xs">
              {{ s.location || s.ip_address || 'Unknown location' }}
              · Expires in {{ expiresIn(s.expires_at) }}
              · Started {{ relativeTime(s.created_at) }}
            </p>
          </div>
          <button class="btn-danger" @click="revokeSession(s.session_id)">Revoke</button>
        </div>
      </div>
    </section>

  </div>
</template>

<style scoped>
.view { display: flex; flex-direction: column; gap: 2.5rem; }
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
.row.column { flex-direction: column; align-items: flex-start; gap: 0.75rem; }
.row-label { font-size: 0.82rem; color: #737373; min-width: 4.5rem; flex-shrink: 0; }
.row-end { display: flex; align-items: center; gap: 0.5rem; flex: 1; justify-content: flex-end; width: 100%; }

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

.add-row { display: flex; gap: 0.5rem; align-items: center; }

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

.btn-danger-red {
  background: none;
  border: 0.5px solid rgba(239, 68, 68, 0.3);
  border-radius: 10px;
  color: #fca5a5;
  font-size: 0.8rem;
  padding: 0.3rem 0.7rem;
  cursor: pointer;
  transition: border-color 0.2s;
  white-space: nowrap;
}
.btn-danger-red:hover:not(:disabled) { border-color: #fca5a5; }
.btn-danger-red:disabled { opacity: 0.4; cursor: not-allowed; }

.badge { font-size: 0.65rem; font-weight: 500; text-transform: uppercase; letter-spacing: 0.05em; padding: 0.15rem 0.4rem; border-radius: 0.25rem; background: rgba(255, 255, 255, 0.08); color: #737373; margin-left: 0.4rem; }
.badge-ok { color: #86efac; background: rgba(34, 197, 94, 0.08); }
.badge-method { color: #93c5fd; background: rgba(59, 130, 246, 0.08); }

.password-form { display: flex; flex-direction: column; gap: 0.5rem; width: 100%; }

.totp-setup { display: flex; flex-direction: column; gap: 0.75rem; width: 100%; }
.qr { width: 160px; height: 160px; border-radius: 0.5rem; }
code { font-size: 0.75rem; color: #a3a3a3; word-break: break-all; }

.msg { font-size: 0.82rem; }
.msg.error { color: #fca5a5; }
.msg.success { color: #86efac; }
</style>
