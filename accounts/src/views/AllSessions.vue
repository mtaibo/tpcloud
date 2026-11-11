<script setup>
import { computed, onMounted, ref } from 'vue'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import Sheet from '@ui/Sheet.vue'
import Spinner from '@ui/Spinner.vue'
import { api } from '@ui/api.js'
import { confirmDestructive, toast } from '@ui/dialog.js'
import Page from '../components/Page.vue'
import SessionInfo from '../components/SessionInfo.vue'
import SessionRow from '../components/SessionRow.vue'
import { store } from '../store.js'

const sessions = ref(null)
const selected = ref(null)
const open = ref(false)

async function load() { sessions.value = await api('/auth/admin/sessions') }
onMounted(load)

const byUser = computed(() => {
  const groups = new Map()
  for (const s of sessions.value || []) {
    if (!groups.has(s.user_id)) groups.set(s.user_id, { name: s.user_display_name, email: s.user_email, items: [] })
    groups.get(s.user_id).items.push(s)
  }
  return [...groups.values()]
})

function show(s) {
  selected.value = { ...s, current: false }
  open.value = true
}

async function revoke() {
  const s = selected.value
  const own = s.user_email === store.me.email
  const ok = await confirmDestructive(
    `¿Cerrar la sesión de ${s.user_display_name}?`,
    own ? 'Es una de tus sesiones. Si es esta misma, tendrás que volver a entrar.' : `${s.device?.label || 'Ese dispositivo'} tendrá que volver a iniciar sesión.`,
    'Cerrar sesión',
  )
  if (!ok) return
  try {
    await api(`/auth/admin/sessions/${s.id}`, { method: 'DELETE' })
    open.value = false
    await load()
    toast('Sesión cerrada')
  } catch (e) {
    toast(e.message, 'warning')
  }
}
</script>

<template>
  <Page title="Todas las sesiones" :subtitle="sessions ? `${sessions.length} abiertas en el servidor` : ''">
    <div v-if="!sessions" class="empty"><Spinner :size="24" /></div>
    <ListSection v-for="g in byUser" :key="g.email" :header="`${g.name} · ${g.email}`">
      <SessionRow v-for="s in g.items" :key="s.id" :session="s" clickable @click="show(s)" />
    </ListSection>

    <Sheet v-model:open="open" :title="selected?.user_display_name || ''">
      <SessionInfo v-if="selected" :session="selected">
        <ListSection>
          <ListRow title="Cerrar esta sesión" clickable destructive centered @click="revoke" />
        </ListSection>
      </SessionInfo>
    </Sheet>
  </Page>
</template>
