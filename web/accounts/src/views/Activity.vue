<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import Icon from '@ui/Icon.vue'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import Segmented from '@ui/Segmented.vue'
import Spinner from '@ui/Spinner.vue'
import { api } from '@ui/api.js'
import { ask, toast } from '@ui/dialog.js'
import { dayHeading, formatTime, methodNames } from '@ui/format.js'
import Page from '../components/Page.vue'
import { store } from '../store.js'
import { lang, t } from '@ui/i18n.js'

const router = useRouter()
const events = ref(null)
const filter = ref('all')
const filters = computed(() => [
  { value: 'all', label: t('All', 'Todo') },
  { value: 'signin', label: t('Sign-ins', 'Entradas') },
  { value: 'failed', label: t('Failed', 'Fallidos') },
])

onMounted(async () => { events.value = await api('/auth/account/activity?limit=300') })

const kinds = {
  signin: { icon: 'checkmark', tint: 'green' },
  failed: { icon: 'warning', tint: 'red' },
  revoked: { icon: 'xmark', tint: 'orange' },
  signout: { icon: 'sign-out', tint: 'gray' },
}

// Auth stores each event's detail in Spanish (old events too), so English is worked out here.
const details = {
  'Contraseña incorrecta': 'Incorrect password',
  'Código de verificación incorrecto': 'Incorrect verification code',
  'La passkey no se pudo verificar': 'The passkey couldn’t be verified',
}
function detail(text) {
  if (!text || lang.value === 'es') return text
  if (details[text]) return details[text]
  const others = text.match(/^Cerraste las demás sesiones \((\d+)\)$/)
  if (others) return `You logged out your other sessions (${others[1]})`
  const denied = text.match(/^Rechazaste un inicio de sesión desde (.*)$/)
  if (denied) return `You declined a sign-in from ${denied[1]}`
  return text
}

function title(e) {
  const method = methodNames[e.method]?.toLowerCase()
  if (e.kind === 'signin') {
    if (e.method === 'qr') return t('Signed in with a QR code', 'Inicio de sesión con código QR')
    return method ? t(`Signed in with ${method}`, `Inicio de sesión con ${method}`) : t('Signed in', 'Inicio de sesión')
  }
  if (e.kind === 'failed') return detail(e.detail) || t('Failed attempt', 'Intento fallido')
  if (e.kind === 'revoked') return !e.method ? detail(e.detail) || t('Sessions logged out', 'Sesiones cerradas') : e.detail ? t(`You logged out ${e.detail}`, `Cerraste la sesión de ${e.detail}`) : t('Session logged out', 'Sesión cerrada')
  return t('Logged out', 'Cierre de sesión')
}

function subtitle(e) {
  const place = e.location && e.location !== e.ip_address ? `${e.location} · ${e.ip_address}` : e.ip_address
  const extra = e.kind === 'signin' && e.method === 'qr' && e.detail ? t(`Approved from ${e.detail}`, `Aprobado desde ${e.detail}`) : ''
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
// Failed sign-ins this week that haven't been confirmed as the owner's own.
const recentFailures = computed(() => {
  const reviewed = store.profile?.failures_reviewed_at ? new Date(store.profile.failures_reviewed_at) : 0
  return (events.value || []).filter(e => e.kind === 'failed' && Date.now() - new Date(e.at) < 7 * 86400000 && new Date(e.at) > reviewed)
})
const failures = computed(() => recentFailures.value.length)

async function reviewFailures() {
  const last = recentFailures.value[0]
  const where = last ? [last.device_info, last.location && last.location !== last.ip_address ? last.location : last.ip_address].filter(Boolean).join(' · ') : ''
  const answer = await ask({
    title: t('Was it you?', '¿Fuiste tú?'),
    message: t(
      `${failures.value === 1 ? 'One failed attempt' : `${failures.value} failed attempts`} to get into your account this week${where ? `, the last one from ${where}` : ''}.`,
      `${failures.value === 1 ? 'Un intento fallido' : `${failures.value} intentos fallidos`} de entrar en tu cuenta esta semana${where ? `, el último desde ${where}` : ''}.`,
    ),
    actions: [
      { label: t('Yes, it was me', 'Sí, fui yo'), role: 'default', value: 'me' },
      { label: t('No, change the password', 'No, cambiar la contraseña'), role: 'destructive', value: 'not-me' },
      { label: t('Cancel', 'Cancelar'), role: 'cancel', value: null },
    ],
  })
  if (answer === 'me') {
    const { failures_reviewed_at } = await api('/auth/account/failures/reviewed', { method: 'POST' })
    if (store.profile) store.profile.failures_reviewed_at = failures_reviewed_at
    toast(t('Warning dismissed', 'Aviso descartado'))
  } else if (answer === 'not-me') {
    router.push('/security')
  }
}
</script>

<template>
  <Page :title="t('Activity', 'Actividad')" :subtitle="t('Every sign-in, failed attempt and logged-out session from the last 6 months.', 'Cada inicio de sesión, intento fallido y sesión cerrada de los últimos 6 meses.')">
    <div v-if="!events" class="empty"><Spinner :size="24" /></div>
    <template v-else>
      <button v-if="failures" type="button" class="banner" @click="reviewFailures">
        <Icon name="warning" :size="22" />
        <p class="subheadline">
          {{ failures === 1
            ? t('There was 1 failed attempt this week. Tap to confirm whether it was you.', 'Ha habido 1 intento fallido esta semana. Toca para confirmar si fuiste tú.')
            : t(`There were ${failures} failed attempts this week. Tap to confirm whether it was you.`, `Ha habido ${failures} intentos fallidos esta semana. Toca para confirmar si fuiste tú.`) }}
        </p>
        <Icon name="chevron-right" :size="17" :weight="2.4" class="row-chevron" />
      </button>

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
        <h3>{{ t('Nothing here', 'Nada por aquí') }}</h3>
        <p class="subheadline">{{ t('Every time someone signs in, or tries to, it will show up here.', 'Aquí aparecerá cada vez que alguien entre o lo intente.') }}</p>
      </div>
    </template>
  </Page>
</template>

<style scoped>
.filter { margin: 0 0 24px; }
.banner {
  display: flex;
  width: 100%;
  gap: 12px;
  align-items: center;
  text-align: left;
  cursor: pointer;
  padding: 14px 18px;
  margin-bottom: 18px;
  border-radius: 20px;
  color: var(--label);
  background: color-mix(in srgb, var(--orange) 16%, var(--cell));
}
.banner .icon { flex-shrink: 0; color: var(--orange); }
.banner p { flex: 1; }
.banner .row-chevron { color: var(--label-3); }
.banner:active { transform: scale(0.99); }
</style>
