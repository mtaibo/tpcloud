<script setup>
import { computed } from 'vue'
import Icon from '@ui/Icon.vue'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import { ago, deviceIcon, formatDateTime, methodSentence } from '@ui/format.js'
import { t } from '@ui/i18n.js'

// Everything known about one session: where it is, how and when it signed in, until when it
// lasts and what it runs on.
const props = defineProps({ session: { type: Object, required: true } })
const s = computed(() => props.session)

const lastSeen = computed(() => {
  if (s.value.current) return t('Now, on this device', 'Ahora, en este dispositivo')
  const minutes = (Date.now() - new Date(s.value.last_seen_at)) / 60000
  return minutes < 5 ? t('Right now', 'Ahora mismo') : `${formatDateTime(s.value.last_seen_at)} (${ago(s.value.last_seen_at)})`
})
const lifetime = computed(() => {
  if (s.value.client_type === 'native') return t('90 days from last use', '90 días desde el último uso')
  return s.value.remember ? t('30 days from last use', '30 días desde el último uso') : t('Until the browser closes (12 h unused)', 'Hasta cerrar el navegador (12 h sin uso)')
})
const location = computed(() => (s.value.location && s.value.location !== s.value.ip_address ? s.value.location : t('Unknown', 'Desconocida')))
</script>

<template>
  <div>
    <div class="hero">
      <span class="device" :class="{ app: s.client_type === 'native' }">
        <Icon :name="deviceIcon(s.device?.kind)" :size="40" :weight="1.5" />
      </span>
      <h2 class="title-2">{{ s.device?.label || s.device_info }}</h2>
      <span v-if="s.current" class="badge green">{{ t('This device', 'Este dispositivo') }}</span>
    </div>

    <ListSection :header="t('Where', 'Dónde')" :footer="t('The location is worked out from the IP address and may not be exact, especially on mobile data or a VPN.', 'La ubicación se calcula a partir de la dirección IP y puede no ser exacta, sobre todo con datos móviles o VPN.')">
      <ListRow icon="location" tint="red" :title="t('Location', 'Ubicación')" :value="location" wrap-value />
      <ListRow icon="network" tint="teal" :title="t('Current IP', 'IP actual')" :value="s.last_ip || '—'" />
      <ListRow v-if="s.ip_address && s.ip_address !== s.last_ip" icon="globe" tint="gray" :title="t('IP at sign-in', 'IP al entrar')" :value="s.ip_address" />
    </ListSection>

    <ListSection :header="t('When', 'Cuándo')">
      <ListRow icon="clock" tint="green" :title="t('Last used', 'Último uso')" :subtitle="lastSeen" />
      <ListRow icon="calendar" tint="orange" :title="t('Signed in', 'Inicio de sesión')" :subtitle="`${formatDateTime(s.created_at)} · ${methodSentence(s)}`" />
      <ListRow icon="history" tint="indigo" :title="t('Expires', 'Caduca')" :subtitle="`${formatDateTime(s.expires_at)} · ${lifetime}`" />
    </ListSection>

    <ListSection :header="t('Device', 'Dispositivo')">
      <ListRow :title="t('Type', 'Tipo')" :value="s.client_type === 'native' ? t('TPCloud app', 'App TPCloud') : t('Browser', 'Navegador')" />
      <ListRow v-if="s.device?.browser && s.client_type !== 'native'" :title="t('Browser', 'Navegador')" :value="s.device.browser" />
      <ListRow v-if="s.device?.os" :title="t('System', 'Sistema')" :value="s.device.os" />
      <ListRow v-if="s.user_agent">
        <span class="row-title">{{ t('User agent', 'Identificación del navegador') }}</span>
        <span class="row-subtitle small mono ua">{{ s.user_agent }}</span>
      </ListRow>
    </ListSection>

    <slot />
  </div>
</template>

<style scoped>
.hero { display: flex; flex-direction: column; align-items: center; gap: 10px; text-align: center; padding: 4px 0 26px; }
.hero h2 { overflow-wrap: anywhere; }
.device {
  width: 84px;
  height: 84px;
  display: grid;
  place-items: center;
  border-radius: 24px;
  color: var(--label-2);
  background: var(--cell);
}
.device.app { color: var(--accent); }
.ua { user-select: all; }
</style>
