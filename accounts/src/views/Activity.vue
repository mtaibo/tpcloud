<script setup>
import { computed, onMounted, ref } from 'vue'
import Icon from '@ui/Icon.vue'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import Segmented from '@ui/Segmented.vue'
import Spinner from '@ui/Spinner.vue'
import { api } from '@ui/api.js'
import { dayHeading, formatTime, methodNames } from '@ui/format.js'
import Page from '../components/Page.vue'

const events = ref(null)
const filter = ref('all')
const filters = [
  { value: 'all', label: 'Todo' },
  { value: 'signin', label: 'Entradas' },
  { value: 'failed', label: 'Fallidos' },
]

onMounted(async () => { events.value = await api('/auth/account/activity?limit=300') })

const kinds = {
  signin: { icon: 'checkmark', tint: 'green' },
  failed: { icon: 'warning', tint: 'red' },
  revoked: { icon: 'xmark', tint: 'orange' },
  signout: { icon: 'sign-out', tint: 'gray' },
}

function title(e) {
  const method = methodNames[e.method]?.toLowerCase()
  if (e.kind === 'signin') {
    if (e.method === 'qr') return 'Inicio de sesión con código QR'
    return method ? `Inicio de sesión con ${method}` : 'Inicio de sesión'
  }
  if (e.kind === 'failed') return e.detail || 'Intento fallido'
  if (e.kind === 'revoked') return !e.method ? e.detail || 'Sesiones cerradas' : e.detail ? `Cerraste la sesión de ${e.detail}` : 'Sesión cerrada'
  return 'Cierre de sesión'
}

function subtitle(e) {
  const place = e.location && e.location !== e.ip_address ? `${e.location} · ${e.ip_address}` : e.ip_address
  const extra = e.kind === 'signin' && e.method === 'qr' && e.detail ? `Aprobado desde ${e.detail}` : ''
  return [e.device_info, place, extra].filter(Boolean).join(' · ')
}

const groups = computed(() => {
  const list = (events.value || []).filter(e => filter.value === 'all' || e.kind === filter.value)
  const out = []
  for (const e of list) {
    const heading = dayHeading(e.at)
    if (out.at(-1)?.heading !== heading) out.push({ heading, items: [] })
    out.at(-1).items.push(e)
  }
  return out
})
const failures = computed(() => (events.value || []).filter(e => e.kind === 'failed' && Date.now() - new Date(e.at) < 7 * 86400000).length)
</script>

<template>
  <Page title="Actividad" subtitle="Cada inicio de sesión, intento fallido y sesión cerrada de los últimos 6 meses.">
    <div v-if="!events" class="empty"><Spinner :size="24" /></div>
    <template v-else>
      <div v-if="failures" class="banner">
        <Icon name="warning" :size="22" />
        <p class="subheadline">
          {{ failures === 1 ? 'Ha habido 1 intento fallido' : `Ha habido ${failures} intentos fallidos` }} esta semana.
          Si no fuiste tú, cambia la contraseña y revisa tus dispositivos.
        </p>
      </div>

      <Segmented v-model="filter" :options="filters" class="filter" />

      <ListSection v-for="g in groups" :key="g.heading" :header="g.heading">
        <ListRow
          v-for="e in g.items"
          :key="e.id"
          :icon="kinds[e.kind]?.icon || 'info'"
          :tint="kinds[e.kind]?.tint || 'gray'"
          :title="title(e)"
          :subtitle="subtitle(e)"
          small-subtitle
          :value="formatTime(e.at)"
        />
      </ListSection>

      <div v-if="!groups.length" class="empty">
        <span class="empty-icon"><Icon name="history" :size="44" :weight="1.4" /></span>
        <h3>Nada por aquí</h3>
        <p class="subheadline">Aquí aparecerá cada vez que alguien entre o lo intente.</p>
      </div>
    </template>
  </Page>
</template>

<style scoped>
.filter { margin: 0 0 24px; }
.banner {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  padding: 14px 18px;
  margin-bottom: 18px;
  border-radius: 20px;
  color: var(--label);
  background: color-mix(in srgb, var(--orange) 16%, var(--cell));
}
.banner .icon { flex-shrink: 0; color: var(--orange); }
</style>
