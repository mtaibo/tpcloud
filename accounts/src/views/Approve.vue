<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import Icon from '@ui/Icon.vue'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import Spinner from '@ui/Spinner.vue'
import { api } from '@ui/api.js'
import { deviceIcon } from '@ui/format.js'

// Opened by scanning the QR code on a login page: shows who's asking and lets this (already
// signed-in) device let them in.
const route = useRoute()
const request = ref(null)
const state = ref('loading') // loading | ready | approved | denied | gone
const message = ref('')
const busy = ref('')
const left = ref(0)
let timer = null

const where = computed(() => {
  const r = request.value
  return r && r.location && r.location !== r.ip_address ? r.location : 'Ubicación desconocida'
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
        message.value = 'El código ha caducado. Genera otro en la pantalla de inicio de sesión.'
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
    await api(`/auth/qr/${encodeURIComponent(route.params.id)}/${approve ? 'approve' : 'deny'}`, { method: 'POST' })
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
        <h1 class="title-1">¿Iniciar sesión en {{ request.device.label }}?</h1>
        <p class="subheadline secondary">Entrará en tu cuenta <strong>{{ request.account.email }}</strong> y seguirá dentro 30 días.</p>
      </div>

      <ListSection>
        <ListRow icon="location" tint="red" title="Desde" :value="where" wrap-value />
        <ListRow icon="network" tint="teal" title="IP" :value="request.ip_address" />
        <ListRow icon="clock" tint="orange" title="Caduca en" :value="`${left} s`" />
      </ListSection>

      <div class="warning">
        <Icon name="warning" :size="20" />
        <p class="footnote">Permítelo solo si eres tú quien está delante de esa pantalla ahora mismo. Si alguien te ha pasado este código, recházalo.</p>
      </div>

      <div class="buttons">
        <button class="btn btn-prominent block" type="button" :disabled="!!busy" @click="decide(true)">
          <Spinner v-if="busy === 'approve'" :size="18" style="color: #fff" />
          <span v-else>Permitir</span>
        </button>
        <button class="btn btn-glass destructive block" type="button" :disabled="!!busy" @click="decide(false)">No, rechazar</button>
      </div>
    </div>

    <div v-else class="center result">
      <span class="result-icon" :class="state === 'approved' ? 'green' : state === 'denied' ? 'red' : 'gray'">
        <Icon :name="state === 'approved' ? 'checkmark' : state === 'denied' ? 'xmark' : 'clock'" :size="40" :weight="2.6" />
      </span>
      <h1 class="title-2">{{ state === 'approved' ? 'Listo' : state === 'denied' ? 'Rechazado' : 'Código no válido' }}</h1>
      <p class="subheadline secondary">
        {{ state === 'approved'
          ? 'Ya puedes seguir en el otro dispositivo. Lo verás en tus dispositivos y en tu actividad.'
          : state === 'denied' ? 'No ha entrado nadie. Queda anotado en tu actividad.' : message }}
      </p>
      <a href="/accounts/sessions" class="btn btn-bordered medium">Ver mis dispositivos</a>
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
