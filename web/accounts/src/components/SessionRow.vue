<script setup>
import { computed } from 'vue'
import Icon from '@ui/Icon.vue'
import ListRow from '@ui/ListRow.vue'
import { ago, deviceIcon, methodNames, where } from '@ui/format.js'
import { t } from '@ui/i18n.js'

const props = defineProps({
  session: { type: Object, required: true },
  to: { type: [String, Object], default: '' },
  clickable: { type: Boolean, default: false },
})
defineEmits(['click'])

const active = computed(() => props.session.current || Date.now() - new Date(props.session.last_seen_at) < 5 * 60000)
const lastSeen = computed(() => (props.session.current ? t('This device', 'Este dispositivo') : active.value ? t('Active now', 'Activa ahora') : t(`Active ${ago(props.session.last_seen_at)}`, `Activa ${ago(props.session.last_seen_at)}`)))
</script>

<template>
  <ListRow :to="to" :clickable="clickable" :chevron="!!to || clickable" @click="$emit('click')">
    <template #leading>
      <span class="device" :class="{ app: session.client_type === 'native' }">
        <Icon :name="deviceIcon(session.device?.kind)" :size="24" :weight="1.7" />
        <i v-if="active" class="dot" />
      </span>
    </template>
    <span class="row-title">{{ session.device?.label || session.device_info || t('Unknown device', 'Dispositivo desconocido') }}</span>
    <span class="row-subtitle">{{ where(session) }}</span>
    <span class="row-subtitle small" :class="{ live: session.current }">{{ lastSeen }} · {{ methodNames[session.auth_method] || session.auth_method }}</span>
  </ListRow>
</template>

<style scoped>
.device {
  position: relative;
  width: 44px;
  height: 44px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
  border-radius: 12px;
  color: var(--label-2);
  background: var(--fill-3);
}
.device.app { color: var(--accent); background: color-mix(in srgb, var(--accent) 12%, transparent); }
.dot {
  position: absolute;
  right: -2px;
  bottom: -2px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--green);
  box-shadow: 0 0 0 2.5px var(--cell);
}
.row { --inset: 78px; }
.live { color: var(--green); }
</style>
