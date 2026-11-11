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
  parts.push(u.active_session_count === 1 ? '1 sesión' : `${u.active_session_count} sesiones`)
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
  const ok = await confirmDestructive(`¿Eliminar a ${u.display_name}?`, 'Se borrarán su cuenta, sus passkeys y sus sesiones. No se puede deshacer.')
  if (!ok) return
  try {
    await api(`/auth/admin/users/${u.id}`, { method: 'DELETE' })
    detailOpen.value = false
    await load()
    toast('Usuario eliminado')
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
    toast(`${u.email} ya puede entrar`)
  } catch (e) {
    createError.value = e.message
  } finally {
    creating.value = false
  }
}
</script>

<template>
  <Page title="Usuarios" :subtitle="users ? `${users.length} ${users.length === 1 ? 'cuenta' : 'cuentas'}` : ''">
    <template #toolbar>
      <button class="icon-btn" type="button" aria-label="Nuevo usuario" @click="openCreate">
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
          <span v-if="u.email === store.me.email" class="badge">Tú</span>
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
          <ListRow title="Cuenta creada" :value="formatDate(selected.created_at)" />
          <ListRow title="Passkeys" :value="String(selected.passkey_count)" />
          <ListRow title="Contraseña" :value="selected.has_password ? 'Sí' : 'No'" />
          <ListRow title="Verificación en dos pasos" :value="selected.totp_enabled ? 'Activada' : 'Desactivada'" />
          <ListRow title="Sesiones abiertas" :value="String(selected.active_session_count)" />
        </ListSection>
        <template v-if="selected.email !== store.me.email">
          <ListSection footer="Un administrador puede gestionar usuarios, ver todas las sesiones y usar las funciones de administración de cada módulo.">
            <ListRow title="Administrador">
              <template #trailing><Toggle :model-value="selected.is_admin" label="Administrador" @update:model-value="toggleAdmin" /></template>
            </ListRow>
          </ListSection>
          <ListSection>
            <ListRow title="Eliminar usuario" clickable destructive centered @click="remove" />
          </ListSection>
        </template>
      </template>
    </Sheet>

    <Sheet v-model:open="createOpen" title="Nuevo usuario" confirmable :can-confirm="canCreate" :busy="creating" @confirm="create">
      <form @submit.prevent="create">
        <ListSection footer="Con esta contraseña podrá entrar y, desde su cuenta, añadir passkeys o cambiarla. Al menos 12 caracteres.">
          <ListRow><input v-model.trim="form.email" class="row-input" type="email" placeholder="Correo" autocomplete="off" aria-label="Correo" /></ListRow>
          <ListRow><input v-model="form.display_name" class="row-input" placeholder="Nombre (opcional)" autocomplete="off" aria-label="Nombre" /></ListRow>
          <ListRow><input v-model="form.password" class="row-input" type="password" placeholder="Contraseña inicial" autocomplete="new-password" aria-label="Contraseña inicial" /></ListRow>
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
