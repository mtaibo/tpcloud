<script setup>
import { computed, onMounted } from 'vue'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import { api } from '@ui/api.js'
import { confirmDestructive, toast } from '@ui/dialog.js'
import Page from '../components/Page.vue'
import SessionRow from '../components/SessionRow.vue'
import { store } from '../store.js'
import { t } from '@ui/i18n.js'

const current = computed(() => store.profile.sessions.find(s => s.current))
const others = computed(() => store.profile.sessions.filter(s => !s.current))

onMounted(() => store.reload())

async function signOutOthers() {
  const n = others.value.length
  const ok = await confirmDestructive(
    n === 1 ? t('Log out the other session?', '¿Cerrar la otra sesión?') : t(`Log out the other ${n} sessions?`, `¿Cerrar las otras ${n} sesiones?`),
    t('Those devices will have to sign in again. This one stays signed in.', 'Esos dispositivos tendrán que volver a iniciar sesión. Este seguirá dentro.'),
    t('Log out', 'Cerrar sesiones'),
  )
  if (!ok) return
  try {
    await api('/auth/account/sessions/others', { method: 'DELETE' })
    await store.reload()
    toast(n === 1 ? t('Session logged out', 'Sesión cerrada') : t(`${n} sessions logged out`, `${n} sesiones cerradas`))
  } catch (e) {
    toast(e.message, 'warning')
  }
}
</script>

<template>
  <Page :title="t('Devices', 'Dispositivos')" :subtitle="t('Where you’re signed in right now.', 'Dónde tienes la sesión iniciada ahora mismo.')">
    <ListSection v-if="current" :header="t('This device', 'Este dispositivo')">
      <SessionRow :session="current" :to="`/sessions/${current.id}`" />
    </ListSection>

    <ListSection
      v-if="others.length"
      :header="t(`Other devices · ${others.length}`, `Otros dispositivos · ${others.length}`)"
      :footer="t('If you don’t recognise one, open it to see where and when it signed in, and log it out. If you think someone knows your password, change it: every other session will be logged out.', 'Si no reconoces alguno, ábrelo para ver desde dónde y cuándo entró, y cierra su sesión. Si crees que alguien sabe tu contraseña, cámbiala: se cerrarán todas las demás.')"
    >
      <SessionRow v-for="s in others" :key="s.id" :session="s" :to="`/sessions/${s.id}`" />
    </ListSection>
    <ListSection v-else :footer="t('You’re not signed in anywhere else.', 'No tienes la sesión iniciada en ningún otro sitio.')">
      <ListRow :title="t('No other devices', 'Ningún otro dispositivo')" />
    </ListSection>

    <ListSection v-if="others.length">
      <ListRow :title="t('Log out other sessions', 'Cerrar las demás sesiones')" clickable destructive centered @click="signOutOthers" />
    </ListSection>

    <p class="footnote secondary note">
      {{ t('A session lasts 30 days from the last time you use it if you chose “Stay signed in”, or until you close the browser if not. In the TPCloud app, 90 days.', 'Una sesión dura 30 días desde la última vez que la usas si marcaste «Mantener la sesión iniciada», o hasta cerrar el navegador si no. En la app TPCloud, 90 días.') }}
    </p>
  </Page>
</template>

<style scoped>
.note { padding: 0 20px; }
</style>
