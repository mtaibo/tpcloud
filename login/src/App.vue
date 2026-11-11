<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import Icon from '@ui/Icon.vue'
import Logo from '@ui/Logo.vue'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import Spinner from '@ui/Spinner.vue'
import Toggle from '@ui/Toggle.vue'
import { conditionalSupported, getCredential, passkeysSupported } from '@ui/webauthn.js'
import QrPanel from './QrPanel.vue'

const step = ref('start') // start | email | password | totp | qr | done
const email = ref('')
const password = ref('')
const code = ref('')
const remember = ref(true)
const busy = ref('') // which action is running
const error = ref('')
const shaking = ref(false)
const ready = ref(false)
const wide = ref(window.matchMedia('(min-width: 900px)').matches)

const emailInput = ref(null)
const passwordInput = ref(null)
const codeInput = ref(null)

const hasPasskeys = passkeysSupported()
let conditional = null

const titles = {
  start: 'Inicia sesión',
  email: 'Continuar con correo',
  password: 'Introduce tu contraseña',
  totp: 'Verificación en dos pasos',
  qr: 'Usar otro dispositivo',
  done: 'Ya has entrado',
}
const subtitles = computed(() => ({
  start: 'Tus archivos, tus fotos y tu casa, en TPCloud.',
  email: 'Escribe el correo de tu cuenta. Si guardaste una passkey, el navegador te la ofrecerá aquí mismo.',
  password: email.value,
  totp: 'Escribe el código de 6 cifras de tu app de autenticación.',
  qr: 'Escanea el código con un iPhone que ya tenga la sesión iniciada.',
  done: 'Un momento…',
}[step.value]))

function fail(e) {
  if (e?.name === 'NotAllowedError' || e?.name === 'AbortError') {
    error.value = ''
    return
  }
  error.value = e?.message || 'Algo ha fallado. Vuelve a intentarlo.'
  shaking.value = false
  requestAnimationFrame(() => { shaking.value = true })
}

async function post(path, body) {
  const res = await fetch(path, {
    method: 'POST',
    headers: body ? { 'Content-Type': 'application/json' } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  })
  const data = res.headers.get('content-type')?.includes('json') ? await res.json() : null
  if (!res.ok) throw new Error(data?.detail || 'Algo ha fallado. Vuelve a intentarlo.')
  return data
}

async function finish() {
  step.value = 'done'
  error.value = ''
  let next = '/'
  try { next = (await post('/auth/login/continue')).next || '/' } catch { /* home it is */ }
  setTimeout(() => window.location.replace(next), 650)
}

function go(to) {
  error.value = ''
  step.value = to
}

// ---- Passkeys ----

function stopConditional() {
  conditional?.abort()
  conditional = null
}

async function startConditional() {
  if (!(await conditionalSupported())) return
  stopConditional()
  const controller = new AbortController()
  conditional = controller
  try {
    const options = await post('/auth/passkey/login/begin')
    const credential = await getCredential(options, { mediation: 'conditional', signal: controller.signal })
    busy.value = 'passkey'
    await post(`/auth/passkey/login/complete?remember=${remember.value}`, credential)
    await finish()
  } catch (e) {
    if (conditional === controller) fail(e)
  } finally {
    if (conditional === controller) conditional = null
    busy.value = ''
  }
}

async function signInWithPasskey(forEmail = '') {
  if (busy.value) return
  stopConditional()
  busy.value = 'passkey'
  error.value = ''
  try {
    const query = forEmail ? `?email=${encodeURIComponent(forEmail)}` : ''
    const options = await post(`/auth/passkey/login/begin${query}`)
    const credential = await getCredential(options)
    await post(`/auth/passkey/login/complete?remember=${remember.value}`, credential)
    await finish()
  } catch (e) {
    fail(e)
  } finally {
    busy.value = ''
  }
}

// ---- Email, password, code ----

function continueWithEmail() {
  if (!email.value.includes('@')) return fail(new Error('Escribe un correo válido.'))
  stopConditional()
  go('password')
}

async function signInWithPassword() {
  if (busy.value || !password.value) return
  busy.value = 'password'
  error.value = ''
  try {
    const data = await post('/auth/password/login', { email: email.value, password: password.value, remember: remember.value })
    if (data.totp_required) go('totp')
    else await finish()
  } catch (e) {
    password.value = ''
    fail(e)
  } finally {
    busy.value = ''
  }
}

async function verifyCode() {
  if (busy.value || code.value.length !== 6) return
  busy.value = 'totp'
  error.value = ''
  try {
    await post('/auth/password/totp/verify', { code: code.value })
    await finish()
  } catch (e) {
    code.value = ''
    if (/Vuelve a iniciar sesión/.test(e.message)) {
      password.value = ''
      step.value = 'password'
    }
    fail(e)
  } finally {
    busy.value = ''
  }
}

watch(code, value => {
  const digits = value.replace(/\D/g, '').slice(0, 6)
  if (digits !== value) code.value = digits
  else if (digits.length === 6) verifyCode()
})

function back() {
  error.value = ''
  if (step.value === 'password') step.value = 'email'
  else if (step.value === 'totp') { code.value = ''; step.value = 'password' }
  else step.value = 'start'
}

watch(step, async value => {
  await nextTick()
  if (value === 'email') { emailInput.value?.focus(); startConditional() }
  if (value === 'password') passwordInput.value?.focus()
  if (value === 'totp') codeInput.value?.focus()
  if (value !== 'email') stopConditional()
})

// ---- Start ----

const media = window.matchMedia('(min-width: 900px)')
const onMedia = e => { wide.value = e.matches; if (e.matches && step.value === 'qr') step.value = 'start' }

onMounted(async () => {
  media.addEventListener('change', onMedia)
  // Old links carry ?next=…: hand it to the server (which keeps it in a cookie) and come back
  // to the plain address.
  const legacyNext = new URLSearchParams(window.location.search).get('next')
  if (legacyNext) {
    window.location.replace(`/auth/signin?next=${encodeURIComponent(legacyNext)}`)
    return
  }
  if (window.location.pathname !== '/login') window.history.replaceState(null, '', '/login')

  try {
    const res = await fetch('/auth/passkey/me')
    if (res.ok) return finish()
  } catch { /* stay here */ }
  ready.value = true
})
onBeforeUnmount(() => { media.removeEventListener('change', onMedia); stopConditional() })
</script>

<template>
  <main class="login" :class="{ wide, ready }">
    <div class="layout" :class="{ split: wide && step === 'start' }">
      <section class="column">
        <div class="top">
          <button
            v-if="step !== 'start' && step !== 'done'"
            class="icon-btn back"
            type="button"
            aria-label="Atrás"
            @click="back"
          >
            <Icon name="chevron-left" :size="22" :weight="2.4" />
          </button>
        </div>

        <header class="hero">
          <Transition name="fade" mode="out-in">
            <span v-if="step === 'done'" key="done" class="done-mark"><Icon name="checkmark" :size="40" :weight="2.6" /></span>
            <Logo v-else key="logo" :size="76" />
          </Transition>
          <h1 class="title-1">{{ titles[step] }}</h1>
          <p class="subheadline secondary">{{ subtitles }}</p>
        </header>

        <Transition name="push" mode="out-in">
          <!-- Pick how -->
          <div v-if="step === 'start'" key="start" class="stack">
            <div class="actions">
              <button v-if="hasPasskeys" class="btn btn-prominent block" type="button" :disabled="!!busy" @click="signInWithPasskey()">
                <Spinner v-if="busy === 'passkey'" :size="18" style="color: #fff" />
                <Icon v-else name="passkey" :size="21" :weight="2" />
                Continuar con passkey
              </button>
              <button class="btn btn-glass block" type="button" @click="go('email')">
                <Icon name="envelope" :size="21" />
                Continuar con correo
              </button>
              <button v-if="!wide" class="btn btn-glass block" type="button" @click="go('qr')">
                <Icon name="qrcode" :size="21" />
                Usar otro dispositivo
              </button>
            </div>

            <ListSection>
              <ListRow title="Mantener la sesión iniciada">
                <template #trailing><Toggle v-model="remember" label="Mantener la sesión iniciada" /></template>
              </ListRow>
              <template #footer>
                {{ remember
                  ? 'Seguirás dentro durante 30 días desde la última vez que entres. Desactívalo en un ordenador compartido.'
                  : 'La sesión se cerrará al cerrar el navegador o tras 12 horas sin usarla.' }}
              </template>
            </ListSection>
          </div>

          <!-- Email -->
          <form v-else-if="step === 'email'" key="email" class="stack" @submit.prevent="continueWithEmail">
            <ListSection>
              <ListRow>
                <input
                  ref="emailInput"
                  v-model.trim="email"
                  class="row-input"
                  type="email"
                  name="username"
                  placeholder="nombre@correo.com"
                  autocomplete="username webauthn"
                  autocapitalize="off"
                  spellcheck="false"
                  aria-label="Correo"
                />
              </ListRow>
            </ListSection>
            <button class="btn btn-prominent block" type="submit" :disabled="!email">Continuar</button>
            <button v-if="hasPasskeys && email.includes('@')" class="btn btn-plain block" type="button" :disabled="!!busy" @click="signInWithPasskey(email)">
              Usar una passkey para este correo
            </button>
          </form>

          <!-- Password -->
          <form v-else-if="step === 'password'" key="password" class="stack" @submit.prevent="signInWithPassword">
            <input type="email" name="username" autocomplete="username" :value="email" hidden />
            <ListSection>
              <ListRow>
                <input
                  ref="passwordInput"
                  v-model="password"
                  class="row-input"
                  type="password"
                  name="password"
                  placeholder="Contraseña"
                  autocomplete="current-password"
                  aria-label="Contraseña"
                />
              </ListRow>
            </ListSection>
            <button class="btn btn-prominent block" type="submit" :disabled="!password || !!busy">
              <Spinner v-if="busy === 'password'" :size="18" style="color: #fff" />
              <span v-else>Iniciar sesión</span>
            </button>
            <button v-if="hasPasskeys" class="btn btn-plain block" type="button" :disabled="!!busy" @click="signInWithPasskey(email)">
              Usar una passkey
            </button>
          </form>

          <!-- TOTP -->
          <div v-else-if="step === 'totp'" key="totp" class="stack">
            <label class="code" :class="{ busy: busy === 'totp' }">
              <input
                ref="codeInput"
                v-model="code"
                inputmode="numeric"
                autocomplete="one-time-code"
                maxlength="6"
                aria-label="Código de verificación"
                :disabled="busy === 'totp'"
              />
              <span v-for="i in 6" :key="i" class="digit" :class="{ filled: code.length >= i, current: code.length === i - 1 }">
                {{ code[i - 1] || '' }}
              </span>
            </label>
            <div class="center-spinner"><Spinner v-if="busy === 'totp'" :size="22" /></div>
          </div>

          <!-- QR, on narrow screens -->
          <div v-else-if="step === 'qr'" key="qr" class="stack">
            <QrPanel @approved="finish" />
          </div>

          <div v-else key="done" class="stack" />
        </Transition>

        <p v-if="error" class="error footnote" :class="{ shake: shaking }" role="alert">
          <Icon name="warning" :size="16" :weight="2" />
          {{ error }}
        </p>
      </section>

      <Transition name="fade">
        <aside v-if="wide && step === 'start'" class="qr-side">
          <h2 class="title-3">Inicia sesión con tu iPhone</h2>
          <QrPanel @approved="finish" />
        </aside>
      </Transition>
    </div>

    <footer class="legal footnote tertiary">TPCloud · {{ new Date().getFullYear() }}</footer>
  </main>
</template>

<style scoped>
.login {
  min-height: 100dvh;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: env(safe-area-inset-top) 20px env(safe-area-inset-bottom);
  opacity: 0;
  transition: opacity 0.3s;
}
.login.ready { opacity: 1; }

.layout {
  flex: 1;
  width: 100%;
  max-width: 420px;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  gap: 0;
  padding: 12px 0 32px;
}
.wide .layout { align-items: center; }
.layout.split { max-width: 920px; gap: 72px; }

.column { width: 100%; max-width: 420px; display: flex; flex-direction: column; }
.top { height: 56px; display: flex; align-items: center; }
.wide .top { height: 44px; }

.hero { display: flex; flex-direction: column; align-items: center; text-align: center; gap: 10px; margin: 8px 0 30px; }
.hero h1 { margin-top: 12px; }
.hero p { max-width: 340px; overflow-wrap: anywhere; }
.done-mark {
  width: 76px;
  height: 76px;
  display: grid;
  place-items: center;
  border-radius: 50%;
  color: #fff;
  background: var(--green);
  animation: pop 0.5s var(--spring);
}
@keyframes pop { from { transform: scale(0.6); opacity: 0; } }

.stack { display: flex; flex-direction: column; gap: 14px; }
.stack > .section { margin: 8px 0 0; }
.actions { display: flex; flex-direction: column; gap: 12px; }

.qr-side {
  width: 380px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 22px;
  padding: 36px 28px 30px;
  border-radius: 34px;
  background: var(--cell);
}

.error {
  display: flex;
  align-items: flex-start;
  justify-content: center;
  gap: 6px;
  margin-top: 16px;
  color: var(--red);
  text-align: center;
}
.error .icon { flex-shrink: 0; margin-top: 1px; }

.code { position: relative; display: grid; grid-template-columns: repeat(6, 1fr); gap: 8px; cursor: text; }
.code input { position: absolute; inset: 0; opacity: 0; font-size: 16px; caret-color: transparent; }
.digit {
  height: 60px;
  display: grid;
  place-items: center;
  border-radius: 14px;
  background: var(--cell);
  font-family: var(--font-rounded);
  font-size: 28px;
  font-weight: 600;
  box-shadow: inset 0 0 0 1.5px transparent;
  transition: box-shadow 0.2s;
}
.code:focus-within .digit.current { box-shadow: inset 0 0 0 2px var(--accent); }
.code.busy .digit { opacity: 0.5; }
.center-spinner { height: 24px; display: grid; place-items: center; }

.legal { padding: 12px 0 18px; }
</style>
