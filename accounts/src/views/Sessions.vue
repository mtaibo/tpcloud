<script setup>
import { computed, onMounted } from 'vue'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import { api } from '@ui/api.js'
import { confirmDestructive, toast } from '@ui/dialog.js'
import Page from '../components/Page.vue'
import SessionRow from '../components/SessionRow.vue'
import { store } from '../store.js'

const current = computed(() => store.profile.sessions.find(s => s.current))
const others = computed(() => store.profile.sessions.filter(s => !s.current))

onMounted(() => store.reload())

async function signOutOthers() {
  const n = others.value.length
  const ok = await confirmDestructive(
    n === 1 ? '¿Cerrar la otra sesión?' : `¿Cerrar las otras ${n} sesiones?`,
    'Esos dispositivos tendrán que volver a iniciar sesión. Este seguirá dentro.',
    'Cerrar sesiones',
  )
  if (!ok) return
  try {
    await api('/auth/account/sessions/others', { method: 'DELETE' })
    await store.reload()
    toast(n === 1 ? 'Sesión cerrada' : `${n} sesiones cerradas`)
  } catch (e) {
    toast(e.message, 'warning')
  }
}
</script>

<template>
  <Page title="Dispositivos" subtitle="Dónde tienes la sesión iniciada ahora mismo.">
    <ListSection v-if="current" header="Este dispositivo">
      <SessionRow :session="current" :to="`/sessions/${current.id}`" />
    </ListSection>

    <ListSection
      v-if="others.length"
      :header="`Otros dispositivos · ${others.length}`"
      footer="Si no reconoces alguno, ábrelo para ver desde dónde y cuándo entró, y cierra su sesión. Si crees que alguien sabe tu contraseña, cámbiala: se cerrarán todas las demás."
    >
      <SessionRow v-for="s in others" :key="s.id" :session="s" :to="`/sessions/${s.id}`" />
    </ListSection>
    <ListSection v-else footer="No tienes la sesión iniciada en ningún otro sitio.">
      <ListRow title="Ningún otro dispositivo" />
    </ListSection>

    <ListSection v-if="others.length">
      <ListRow title="Cerrar las demás sesiones" clickable destructive centered @click="signOutOthers" />
    </ListSection>

    <p class="footnote secondary note">
      Una sesión dura 30 días desde la última vez que la usas si marcaste «Mantener la sesión iniciada»,
      o hasta cerrar el navegador si no. En la app TPCloud, 90 días.
    </p>
  </Page>
</template>

<style scoped>
.note { padding: 0 20px; }
</style>
