<script setup>
import { computed, onMounted, ref } from 'vue'
import Icon from '@ui/Icon.vue'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import Sheet from '@ui/Sheet.vue'
import Spinner from '@ui/Spinner.vue'
import Toggle from '@ui/Toggle.vue'
import { api } from '@ui/api.js'
import { confirmDestructive, toast } from '@ui/dialog.js'
import { formatDate, initials } from '@ui/format.js'
import Page from '../components/Page.vue'
import { store } from '../store.js'
import { t } from '@ui/i18n.js'

const users = ref(null)
const selectedId = ref(null)
const selected = computed(() => users.value?.find(u => u.id === selectedId.value))
const detailOpen = ref(false)

async function load() { users.value = await api('/auth/admin/users') }
onMounted(load)

function open(u) {
  selectedId.value = u.id
  detailOpen.value = true
}

function summary(u) {
  const parts = [u.email]
  parts.push(u.active_session_count === 1 ? t('1 session', '1 sesión') : t(`${u.active_session_count} sessions`, `${u.active_session_count} sesiones`))
  return parts.join(' · ')
}

async function toggleAdmin() {
  try {
    await api(`/auth/admin/users/${selected.value.id}`, { method: 'PATCH' })
    await load()
  } catch (e) {
    toast(e.message, 'warning')
  }
}

async function remove() {
  const u = selected.value
  const ok = await confirmDestructive(t(`Delete ${u.display_name}?`, `¿Eliminar a ${u.display_name}?`), t('Their account, passkeys and sessions will be deleted. This can’t be undone.', 'Se borrarán su cuenta, sus passkeys y sus sesiones. No se puede deshacer.'))
  if (!ok) return
  try {
    await api(`/auth/admin/users/${u.id}`, { method: 'DELETE' })
    detailOpen.value = false
    await load()
    toast(t('User deleted', 'Usuario eliminado'))
  } catch (e) {
    toast(e.message, 'warning')
  }
}

// ---- New user ----

const createOpen = ref(false)
const form = ref({ email: '', display_name: '', password: '' })
const creating = ref(false)
const createError = ref('')
const canCreate = computed(() => form.value.email.includes('@') && form.value.password.length >= 12)

function openCreate() {
  form.value = { email: '', display_name: '', password: '' }
  createError.value = ''
  createOpen.value = true
}

async function create() {
  if (!canCreate.value) return
  creating.value = true
  createError.value = ''
  try {
    const u = await api('/auth/admin/users', { method: 'POST', body: form.value })
    createOpen.value = false
    await load()
    toast(t(`${u.email} can sign in now`, `${u.email} ya puede entrar`))
  } catch (e) {
    createError.value = e.message
  } finally {
    creating.value = false
  }
}
</script>

<template>
  <Page :title="t('Users', 'Usuarios')" :subtitle="users ? `${users.length} ${users.length === 1 ? t('account', 'cuenta') : t('accounts', 'cuentas')}` : ''">
    <template #toolbar>
      <button class="icon-btn" type="button" :aria-label="t('New user', 'Nuevo usuario')" @click="openCreate">
        <Icon name="plus" :size="22" :weight="2.2" />
      </button>
    </template>

    <div v-if="!users" class="empty"><Spinner :size="24" /></div>
    <ListSection v-else>
      <ListRow v-for="u in users" :key="u.id" clickable :chevron="true" class="user-row" @click="open(u)">
        <template #leading>
          <span class="avatar" style="width: 42px; height: 42px; font-size: 17px">{{ initials(u.display_name) }}</span>
        </template>
        <span class="row-title">
          {{ u.display_name }}
          <span v-if="u.email === store.me.email" class="badge">{{ t('You', 'Tú') }}</span>
          <span v-if="u.is_admin" class="badge purple">Admin</span>
        </span>
        <span class="row-subtitle small">{{ summary(u) }}</span>
      </ListRow>
    </ListSection>

    <Sheet v-model:open="detailOpen" :title="selected?.display_name || ''">
      <template v-if="selected">
        <div class="sheet-hero">
          <span class="avatar" style="width: 80px; height: 80px; font-size: 32px">{{ initials(selected.display_name) }}</span>
          <p class="subheadline secondary">{{ selected.email }}</p>
        </div>
        <ListSection>
          <ListRow :title="t('Account created', 'Cuenta creada')" :value="formatDate(selected.created_at)" />
          <ListRow title="Passkeys" :value="String(selected.passkey_count)" />
          <ListRow :title="t('Password', 'Contraseña')" :value="selected.has_password ? t('Yes', 'Sí') : t('No', 'No')" />
          <ListRow :title="t('Two-step verification', 'Verificación en dos pasos')" :value="selected.totp_enabled ? t('On', 'Activada') : t('Off', 'Desactivada')" />
          <ListRow :title="t('Open sessions', 'Sesiones abiertas')" :value="String(selected.active_session_count)" />
        </ListSection>
        <template v-if="selected.email !== store.me.email">
          <ListSection :footer="t('An administrator can manage users, see every session and use each module’s admin features.', 'Un administrador puede gestionar usuarios, ver todas las sesiones y usar las funciones de administración de cada módulo.')">
            <ListRow :title="t('Administrator', 'Administrador')">
              <template #trailing><Toggle :model-value="selected.is_admin" :label="t('Administrator', 'Administrador')" @update:model-value="toggleAdmin" /></template>
            </ListRow>
          </ListSection>
          <ListSection>
            <ListRow :title="t('Delete user', 'Eliminar usuario')" clickable destructive centered @click="remove" />
          </ListSection>
        </template>
      </template>
    </Sheet>

    <Sheet v-model:open="createOpen" :title="t('New user', 'Nuevo usuario')" confirmable :can-confirm="canCreate" :busy="creating" @confirm="create">
      <form @submit.prevent="create">
        <ListSection :footer="t('With this password they can sign in and, from their account, add passkeys or change it. At least 12 characters.', 'Con esta contraseña podrá entrar y, desde su cuenta, añadir passkeys o cambiarla. Al menos 12 caracteres.')">
          <ListRow><input v-model.trim="form.email" class="row-input" type="email" :placeholder="t('Email', 'Correo')" autocomplete="off" :aria-label="t('Email', 'Correo')" /></ListRow>
          <ListRow><input v-model="form.display_name" class="row-input" :placeholder="t('Name (optional)', 'Nombre (opcional)')" autocomplete="off" :aria-label="t('Name', 'Nombre')" /></ListRow>
          <ListRow><input v-model="form.password" class="row-input" type="password" :placeholder="t('Initial password', 'Contraseña inicial')" autocomplete="new-password" :aria-label="t('Initial password', 'Contraseña inicial')" /></ListRow>
        </ListSection>
        <button type="submit" hidden />
      </form>
      <p v-if="createError" class="footnote danger" style="text-align: center">{{ createError }}</p>
    </Sheet>
  </Page>
</template>

<style scoped>
.user-row { --inset: 76px; }
.row-title .badge { margin-left: 6px; }
.sheet-hero { display: flex; flex-direction: column; align-items: center; gap: 10px; padding: 4px 0 22px; }
</style>
