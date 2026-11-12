<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import Icon from '@ui/Icon.vue'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import Spinner from '@ui/Spinner.vue'
import Toggle from '@ui/Toggle.vue'
import { api } from '@ui/api.js'
import { deviceIcon } from '@ui/format.js'
import { t } from '@ui/i18n.js'

// Opened by scanning the QR code on a login page: shows who's asking and lets this (already
// signed-in) device let them in.
const route = useRoute()
const request = ref(null)
const state = ref('loading') // loading | ready | approved | denied | gone
const message = ref('')
const busy = ref('')
const left = ref(0)
const remember = ref(true) // keep the other browser signed in
let timer = null

const where = computed(() => {
  const r = request.value
  return r && r.location && r.location !== r.ip_address ? r.location : t('Unknown location', 'Ubicación desconocida')
})

onMounted(async () => {
  window.history.replaceState(window.history.state, '', window.location.pathname)
  try {
    request.value = await api(`/auth/qr/${encodeURIComponent(route.params.id)}`)
    left.value = request.value.expires_in
    state.value = 'ready'
    timer = setInterval(() => {
      left.value -= 1
      if (left.value <= 0 && state.value === 'ready') {
        state.value = 'gone'
        message.value = t('The code has expired. Make a new one on the sign-in screen.', 'El código ha caducado. Genera otro en la pantalla de inicio de sesión.')
      }
    }, 1000)
  } catch (e) {
    state.value = 'gone'
    message.value = e.message
  }
})
onBeforeUnmount(() => clearInterval(timer))

async function decide(approve) {
  busy.value = approve ? 'approve' : 'deny'
  try {
    await api(`/auth/qr/${encodeURIComponent(route.params.id)}/${approve ? 'approve' : 'deny'}`, {
      method: 'POST',
      body: approve ? { remember: remember.value } : undefined,
    })
    state.value = approve ? 'approved' : 'denied'
  } catch (e) {
    state.value = 'gone'
    message.value = e.message
  } finally {
    busy.value = ''
  }
}
</script>

<template>
  <main class="approve">
    <div v-if="state === 'loading'" class="center"><Spinner :size="26" /></div>

    <div v-else-if="state === 'ready'" class="content">
      <div class="hero">
        <span class="device"><Icon :name="deviceIcon(request.device.kind)" :size="46" :weight="1.5" /></span>
        <h1 class="title-1">{{ t(`Sign in on ${request.device.label}?`, `¿Iniciar sesión en ${request.device.label}?`) }}</h1>
        <p class="subheadline secondary">{{ t('It will get into your account', 'Entrará en tu cuenta') }} <strong>{{ request.account.email }}</strong>.</p>
      </div>

      <ListSection>
        <ListRow icon="location" tint="red" :title="t('From', 'Desde')" :value="where" wrap-value />
        <ListRow icon="network" tint="teal" title="IP" :value="request.ip_address" />
        <ListRow icon="clock" tint="orange" :title="t('Expires in', 'Caduca en')" :value="`${left} s`" />
      </ListSection>

      <ListSection>
        <ListRow :title="t('Stay signed in', 'Mantener la sesión iniciada')">
          <template #trailing><Toggle v-model="remember" :label="t('Stay signed in', 'Mantener la sesión iniciada')" :disabled="!!busy" /></template>
        </ListRow>
        <template #footer>
          {{ remember
            ? t('It stays signed in for 30 days from the last time it’s used. Turn it off on a shared computer.', 'Seguirá dentro 30 días desde la última vez que lo use. Desactívalo si es un ordenador compartido.')
            : t('It signs out when the browser closes or after 12 hours unused.', 'Se cerrará al cerrar el navegador o tras 12 horas sin usarlo.') }}
        </template>
      </ListSection>

      <div class="warning">
        <Icon name="warning" :size="20" />
        <p class="footnote">{{ t('Only allow it if you’re the one in front of that screen right now. If someone sent you this code, decline it.', 'Permítelo solo si eres tú quien está delante de esa pantalla ahora mismo. Si alguien te ha pasado este código, recházalo.') }}</p>
      </div>

      <div class="buttons">
        <button class="btn btn-prominent block" type="button" :disabled="!!busy" @click="decide(true)">
          <Spinner v-if="busy === 'approve'" :size="18" style="color: #fff" />
          <span v-else>{{ t('Allow', 'Permitir') }}</span>
        </button>
        <button class="btn btn-glass destructive block" type="button" :disabled="!!busy" @click="decide(false)">{{ t('No, decline', 'No, rechazar') }}</button>
      </div>
    </div>

    <div v-else class="center result">
      <span class="result-icon" :class="state === 'approved' ? 'green' : state === 'denied' ? 'red' : 'gray'">
        <Icon :name="state === 'approved' ? 'checkmark' : state === 'denied' ? 'xmark' : 'clock'" :size="40" :weight="2.6" />
      </span>
      <h1 class="title-2">{{ state === 'approved' ? t('Done', 'Listo') : state === 'denied' ? t('Declined', 'Rechazado') : t('Code not valid', 'Código no válido') }}</h1>
      <p class="subheadline secondary">
        {{ state === 'approved'
          ? t('You can carry on on the other device. You’ll see it in your devices and your activity.', 'Ya puedes seguir en el otro dispositivo. Lo verás en tus dispositivos y en tu actividad.')
          : state === 'denied' ? t('Nobody got in. It’s noted in your activity.', 'No ha entrado nadie. Queda anotado en tu actividad.') : message }}
      </p>
      <a href="/accounts/sessions" class="btn btn-bordered medium">{{ t('See my devices', 'Ver mis dispositivos') }}</a>
    </div>
  </main>
</template>

<style scoped>
.approve { min-height: 100dvh; display: flex; justify-content: center; padding: calc(28px + env(safe-area-inset-top)) 20px calc(28px + env(safe-area-inset-bottom)); }
.content { width: 100%; max-width: 440px; display: flex; flex-direction: column; }
.center { flex: 1; max-width: 400px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; text-align: center; }
.hero { display: flex; flex-direction: column; align-items: center; gap: 10px; text-align: center; padding: 20px 0 28px; }
.hero h1 { margin-top: 10px; overflow-wrap: anywhere; }
.device { width: 96px; height: 96px; display: grid; place-items: center; border-radius: 28px; color: var(--accent); background: var(--cell); }
.warning { display: flex; gap: 10px; align-items: flex-start; padding: 0 20px 26px; color: var(--label-2); }
.warning .icon { flex-shrink: 0; color: var(--orange); }
.buttons { display: grid; gap: 12px; }
.result-icon { width: 84px; height: 84px; display: grid; place-items: center; border-radius: 50%; color: #fff; margin-bottom: 8px; animation: pop 0.5s var(--spring); }
.result-icon.green { background: var(--green); }
.result-icon.red { background: var(--red); }
.result-icon.gray { background: var(--gray); }
.result .btn { margin-top: 14px; }
@keyframes pop { from { transform: scale(0.6); opacity: 0; } }
</style>
