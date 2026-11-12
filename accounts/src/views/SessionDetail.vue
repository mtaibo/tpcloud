<script setup>
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import { api, signOut } from '@ui/api.js'
import { confirmDestructive, toast } from '@ui/dialog.js'
import Page from '../components/Page.vue'
import SessionInfo from '../components/SessionInfo.vue'
import { store } from '../store.js'
import { t } from '@ui/i18n.js'

const route = useRoute()
const router = useRouter()
const session = computed(() => store.profile.sessions.find(s => String(s.id) === route.params.id))

async function revoke() {
  const s = session.value
  if (s.current) {
    const ok = await confirmDestructive(t('Log out on this device?', '¿Cerrar sesión en este dispositivo?'), t('You’ll have to sign in again here.', 'Tendrás que volver a iniciar sesión aquí.'), t('Log out', 'Cerrar sesión'))
    if (ok) signOut()
    return
  }
  const ok = await confirmDestructive(
    t(`Log out ${s.device?.label || 'this device'}?`, `¿Cerrar la sesión de ${s.device?.label || 'este dispositivo'}?`),
    t('It will have to sign in again to get in.', 'Tendrá que volver a iniciar sesión para entrar.'),
    t('Log out', 'Cerrar sesión'),
  )
  if (!ok) return
  try {
    await api(`/auth/account/session/${s.id}`, { method: 'DELETE' })
    await store.reload()
    toast(t('Session logged out', 'Sesión cerrada'))
    router.replace('/sessions')
  } catch (e) {
    toast(e.message, 'warning')
  }
}
</script>

<template>
  <Page :title="t('Session', 'Sesión')">
    <template #header><span /></template>
    <SessionInfo v-if="session" :session="session">
      <ListSection>
        <ListRow
          :title="session.current ? t('Log out on this device', 'Cerrar sesión en este dispositivo') : t('Log out this session', 'Cerrar esta sesión')"
          clickable
          destructive
          centered
          @click="revoke"
        />
      </ListSection>
    </SessionInfo>
    <div v-else class="empty">
      <h3>{{ t('This session no longer exists', 'Esta sesión ya no existe') }}</h3>
      <p class="subheadline">{{ t('It was logged out or expired.', 'Se cerró o caducó.') }}</p>
      <router-link to="/sessions" class="btn btn-bordered small" style="margin-top: 12px">{{ t('See devices', 'Ver dispositivos') }}</router-link>
    </div>
  </Page>
</template>
