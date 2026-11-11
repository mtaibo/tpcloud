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

const route = useRoute()
const router = useRouter()
const session = computed(() => store.profile.sessions.find(s => String(s.id) === route.params.id))

async function revoke() {
  const s = session.value
  if (s.current) {
    const ok = await confirmDestructive('¿Cerrar sesión en este dispositivo?', 'Tendrás que volver a iniciar sesión aquí.', 'Cerrar sesión')
    if (ok) signOut()
    return
  }
  const ok = await confirmDestructive(
    `¿Cerrar la sesión de ${s.device?.label || 'este dispositivo'}?`,
    'Tendrá que volver a iniciar sesión para entrar.',
    'Cerrar sesión',
  )
  if (!ok) return
  try {
    await api(`/auth/account/session/${s.id}`, { method: 'DELETE' })
    await store.reload()
    toast('Sesión cerrada')
    router.replace('/sessions')
  } catch (e) {
    toast(e.message, 'warning')
  }
}
</script>

<template>
  <Page title="Sesión">
    <template #header><span /></template>
    <SessionInfo v-if="session" :session="session">
      <ListSection>
        <ListRow
          :title="session.current ? 'Cerrar sesión en este dispositivo' : 'Cerrar esta sesión'"
          clickable
          destructive
          centered
          @click="revoke"
        />
      </ListSection>
    </SessionInfo>
    <div v-else class="empty">
      <h3>Esta sesión ya no existe</h3>
      <p class="subheadline">Se cerró o caducó.</p>
      <router-link to="/sessions" class="btn btn-bordered small" style="margin-top: 12px">Ver dispositivos</router-link>
    </div>
  </Page>
</template>
