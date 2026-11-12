<script setup>
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import { initials } from '@ui/format.js'
import { signOut } from '@ui/api.js'
import { ask } from '@ui/dialog.js'
import { lang, t } from '@ui/i18n.js'
import { store, wide } from '../store.js'

// The account's sections, like the root of Settings: the profile card first, then each screen.
const route = useRoute()
const profile = computed(() => store.profile)
const otherSessions = computed(() => (profile.value?.sessions.length ?? 1) - 1)
const security = computed(() => {
  const p = profile.value
  if (!p) return ''
  const n = p.passkeys.length
  return n ? `${n} passkey${n === 1 ? '' : 's'}` : p.totp_enabled ? t('With 2FA', 'Con 2FA') : ''
})
const active = path => wide.value && (route.path === path || route.path.startsWith(`${path}/`))

async function confirmSignOut() {
  const ok = await ask({
    title: t('Log out?', '¿Cerrar sesión?'),
    message: t('You\'ll have to sign in again on this browser.', 'Tendrás que volver a iniciar sesión en este navegador.'),
    actions: [
      { label: t('Cancel', 'Cancelar'), role: 'cancel', value: false },
      { label: t('Log out', 'Cerrar sesión'), role: 'destructive', value: true },
    ],
  })
  if (ok) signOut()
}
</script>

<template>
  <nav v-if="profile" class="menu">
    <ListSection>
      <ListRow to="/profile" :chevron="!wide" :class="{ selected: active('/profile') }" class="profile-row">
        <template #leading>
          <span class="avatar" style="width: 60px; height: 60px; font-size: 24px">{{ initials(profile.display_name) }}</span>
        </template>
        <span class="title-3 name">{{ profile.display_name }}</span>
        <span class="row-subtitle">{{ profile.email }}</span>
      </ListRow>
    </ListSection>

    <ListSection>
      <ListRow to="/security" :chevron="!wide" icon="shield" tint="blue" :title="t('Security', 'Seguridad')" :value="security" :class="{ selected: active('/security') }" />
      <ListRow to="/sessions" :chevron="!wide" icon="laptop" tint="green" :title="t('Devices', 'Dispositivos')" :value="otherSessions > 0 ? `${otherSessions + 1}` : ''" :class="{ selected: active('/sessions') }" />
      <ListRow to="/activity" :chevron="!wide" icon="history" tint="orange" :title="t('Activity', 'Actividad')" :class="{ selected: active('/activity') }" />
      <ListRow to="/language" :chevron="!wide" icon="globe" tint="blue" :title="t('Language', 'Idioma')" :value="lang === 'es' ? 'Español' : 'English'" :class="{ selected: active('/language') }" />
    </ListSection>

    <ListSection v-if="profile.is_admin" :header="t('Administration', 'Administración')">
      <ListRow to="/users" :chevron="!wide" icon="person-2" tint="indigo" :title="t('Users', 'Usuarios')" :class="{ selected: active('/users') }" />
      <ListRow to="/all-sessions" :chevron="!wide" icon="network" tint="teal" :title="t('All sessions', 'Todas las sesiones')" :class="{ selected: active('/all-sessions') }" />
    </ListSection>

    <ListSection>
      <ListRow href="/" icon="cloud" tint="gray" :title="t('Back to TPCloud', 'Volver a TPCloud')" :chevron="false" />
      <ListRow :title="t('Log out', 'Cerrar sesión')" clickable destructive @click="confirmSignOut" />
    </ListSection>
  </nav>
</template>

<style scoped>
.profile-row { min-height: 84px; --inset: 96px; }
.name { display: block; }
.selected, .selected:hover { background: var(--accent) !important; color: #fff; }
.selected :deep(.row-value), .selected :deep(.row-subtitle), .selected :deep(.row-chevron) { color: rgba(255, 255, 255, 0.75); }
.selected :deep(.row-icon) { box-shadow: 0 0 0 1.5px rgba(255, 255, 255, 0.85); }
.selected + :deep(.row)::before, .selected::before { opacity: 0; }
</style>
