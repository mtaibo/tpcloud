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

const step = ref('start') // start | email | password | totp | qr | done | leaving
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
const split = computed(() => wide.value && step.value === 'start')
// Steps that share a screen keep one pane, so nothing animates between them: email and password
// are one form (the password row opens under the email, as in Apple's sign-in), "done" and
// "leaving" are one screen.
const pane = computed(() => ({ leaving: 'done', email: 'credentials', password: 'credentials' }[step.value] || step.value))
const canGoBack = computed(() => !['start', 'done', 'leaving'].includes(step.value))
let conditional = null

const titles = {
  start: 'Log in',
  email: 'Sign in',
  password: 'Sign in',
  totp: 'Two-step verification',
  qr: 'Use another device',
  done: "You're in",
  leaving: "You're in",
}
const subtitles = computed(() => ({
  start: 'Your files, your photos and your home, in TPCloud.',
  email: "Enter your account's email. If you saved a passkey, your browser will offer it right here.",
  password: "Enter your account's email. If you saved a passkey, your browser will offer it right here.",
  totp: 'Enter the 6-digit code from your authenticator app.',
  qr: 'Scan the code with an iPhone that is already signed in.',
  done: 'One last thing before you go in.',
  leaving: 'One moment…',
}[step.value]))

function fail(e) {
  if (e?.name === 'NotAllowedError' || e?.name === 'AbortError') {
    error.value = ''
    return
  }
  error.value = e?.message || 'Something went wrong. Please try again.'
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
  if (!res.ok) throw new Error(data?.detail || 'Something went wrong. Please try again.')
  return data
}

// Signed in: ask whether to keep this browser signed in, then go on.
function finish() {
  stopConditional()
  error.value = ''
  step.value = 'done'
}

// Let in from a phone: the phone already decided how long this stays signed in, so go straight on
// while the QR panel shows "Approved".
async function finishFromQr() {
  stopConditional()
  error.value = ''
  let next = '/'
  try { next = (await post('/auth/login/continue')).next || '/' } catch { /* home it is */ }
  window.location.replace(next)
}

async function leave(rememberChoice) {
  if (busy.value) return
  busy.value = 'continue'
  try {
    if (rememberChoice !== null) await post('/auth/login/remember', { remember: rememberChoice })
  } catch { /* stays a browser-session sign-in */ }
  let next = '/'
  try { next = (await post('/auth/login/continue')).next || '/' } catch { /* home it is */ }
  step.value = 'leaving'
  setTimeout(() => window.location.replace(next), 350)
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
    await post(`/auth/passkey/login/complete?remember=false`, credential)
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
    await post(`/auth/passkey/login/complete?remember=false`, credential)
    await finish()
  } catch (e) {
    fail(e)
  } finally {
    busy.value = ''
  }
}

// ---- Email, password, code ----

function continueWithEmail() {
  if (!email.value.includes('@')) return fail(new Error('Enter a valid email.'))
  stopConditional()
  go('password')
}

async function signInWithPassword() {
  if (busy.value || !password.value) return
  busy.value = 'password'
  error.value = ''
  try {
    const data = await post('/auth/password/login', { email: email.value, password: password.value, remember: false })
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
    if (/sign in again/.test(e.message)) {
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

watch(step, value => {
  if (value === 'email') startConditional()
  else stopConditional()
})
// Changing the email after Continue closes the password row again, like Apple's sign-in.
watch(email, () => { if (step.value === 'password') step.value = 'email' })

// A new pane's field only exists once the old one has faded out, so focus it when it appears.
watch([emailInput, codeInput], ([emailField, codeField]) => {
  if (codeField) codeField.focus()
  else if (emailField) (step.value === 'password' ? passwordInput.value : emailField)?.focus()
})
// Inside the email/password form the fields are already there.
watch(step, (value, old) => {
  if (value === 'password' && old === 'email') nextTick(() => passwordInput.value?.focus())
  else if (value === 'email' && old === 'password') nextTick(() => emailInput.value?.focus())
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
    if (res.ok) return leave(null)
  } catch { /* stay here */ }
  ready.value = true
})
onBeforeUnmount(() => { media.removeEventListener('change', onMedia); stopConditional() })
</script>

<template>
  <main class="login" :class="{ wide, ready }">
    <Transition name="swap" mode="out-in">
      <div :key="split ? 'split' : 'single'" class="layout" :class="{ split }">
        <section class="column">
          <Transition name="step" mode="out-in">
            <div :key="pane" class="pane">
              <div class="top">
                <button
                  v-if="canGoBack"
                  class="icon-btn back"
                  type="button"
                  aria-label="Back"
                  @click="back"
                >
                  <Icon name="chevron-left" :size="22" :weight="2.4" />
                </button>
              </div>

              <header class="hero">
                <span v-if="pane === 'done'" class="done-mark"><Icon name="checkmark" :size="40" :weight="2.6" /></span>
                <Logo v-else :size="76" />
                <h1 class="title-1">{{ titles[step] }}</h1>
                <p class="subheadline secondary">{{ subtitles }}</p>
              </header>

              <!-- Pick how -->
              <div v-if="step === 'start'" class="stack">
                <div class="actions">
                  <button v-if="hasPasskeys" class="btn btn-prominent block" type="button" :disabled="!!busy" @click="signInWithPasskey()">
                    <Spinner v-if="busy === 'passkey'" :size="18" style="color: #fff" />
                    <Icon v-else name="passkey" :size="21" :weight="2" />
                    Continue with passkey
                  </button>
                  <button class="btn btn-glass block" type="button" @click="go('email')">
                    <Icon name="envelope" :size="21" />
                    Continue with email
                  </button>
                  <button v-if="!wide" class="btn btn-glass block" type="button" @click="go('qr')">
                    <Icon name="qrcode" :size="21" />
                    Use another device
                  </button>
                </div>
              </div>

              <!-- Email, then password in the same card -->
              <form
                v-else-if="pane === 'credentials'"
                class="stack"
                @submit.prevent="step === 'email' ? continueWithEmail() : signInWithPassword()"
              >
                <ListSection>
                  <ListRow>
                    <input
                      ref="emailInput"
                      v-model.trim="email"
                      class="row-input"
                      type="email"
                      name="username"
                      placeholder="name@example.com"
                      autocomplete="username webauthn"
                      autocapitalize="off"
                      spellcheck="false"
                      aria-label="Email"
                    />
                  </ListRow>
                  <!-- Always in the page so password managers fill both; opens after Continue. -->
                  <div class="reveal" :class="{ open: step === 'password' }" :inert="step !== 'password'">
                    <div class="reveal-inner">
                      <ListRow>
                        <input
                          ref="passwordInput"
                          v-model="password"
                          class="row-input"
                          type="password"
                          name="password"
                          placeholder="Password"
                          autocomplete="current-password"
                          aria-label="Password"
                        />
                      </ListRow>
                    </div>
                  </div>
                </ListSection>
                <button
                  class="btn btn-prominent block"
                  type="submit"
                  :disabled="step === 'email' ? !email : !password || !!busy"
                >
                  <Spinner v-if="busy === 'password'" :size="18" style="color: #fff" />
                  <span v-else>{{ step === 'email' ? 'Continue' : 'Sign in' }}</span>
                </button>
                <!-- Keeps its place while hidden, so nothing moves when it shows up. -->
                <button
                  v-if="hasPasskeys"
                  class="btn btn-plain block"
                  :class="{ invisible: !email.includes('@') }"
                  type="button"
                  :disabled="!!busy || !email.includes('@')"
                  @click="signInWithPasskey(email)"
                >
                  Use a passkey for this email
                </button>
                <!-- Takes the room the password row will need, so opening it doesn't shift the screen. -->
                <div class="reserve" :class="{ open: step === 'password' }" aria-hidden="true" inert>
                  <div class="reveal-inner"><ListRow><input class="row-input" tabindex="-1" /></ListRow></div>
                </div>
              </form>

              <!-- TOTP -->
              <div v-else-if="step === 'totp'" class="stack">
                <label class="code" :class="{ busy: busy === 'totp' }">
                  <input
                    ref="codeInput"
                    v-model="code"
                    inputmode="numeric"
                    autocomplete="one-time-code"
                    maxlength="6"
                    aria-label="Verification code"
                    :disabled="busy === 'totp'"
                  />
                  <span v-for="i in 6" :key="i" class="digit" :class="{ filled: code.length >= i, current: code.length === i - 1 }">
                    {{ code[i - 1] || '' }}
                  </span>
                </label>
                <div class="center-spinner"><Spinner v-if="busy === 'totp'" :size="22" /></div>
              </div>

              <!-- QR, on narrow screens -->
              <div v-else-if="step === 'qr'" class="stack">
                <QrPanel @approved="finishFromQr" />
              </div>

              <!-- Signed in: keep it? -->
              <div v-else class="stack">
                <ListSection>
                  <ListRow title="Keep me signed in">
                    <template #trailing><Toggle v-model="remember" label="Keep me signed in" :disabled="step === 'leaving'" /></template>
                  </ListRow>
                  <template #footer>
                    {{ remember
                      ? "You'll stay signed in for 30 days after your last visit. Turn this off on a shared computer."
                      : "You'll be signed out when you close the browser or after 12 hours without using it." }}
                  </template>
                </ListSection>
                <button class="btn btn-prominent block" type="button" :disabled="!!busy || step === 'leaving'" @click="leave(remember)">
                  <Spinner v-if="busy === 'continue' || step === 'leaving'" :size="18" style="color: #fff" />
                  <span v-else>Continue</span>
                </button>
              </div>

              <p v-if="error" class="error footnote" :class="{ shake: shaking }" role="alert">
                <Icon name="warning" :size="16" :weight="2" />
                {{ error }}
              </p>
            </div>
          </Transition>
        </section>

        <aside v-if="split" class="qr-side">
          <h2 class="title-3">Sign in with your iPhone</h2>
          <QrPanel @approved="finishFromQr" />
        </aside>
      </div>
    </Transition>

    <footer class="legal footnote tertiary">
      Made by <a href="https://www.migueltaibo.com" target="_blank" rel="noopener">Miguel Taibo</a>
    </footer>
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

.pane { display: flex; flex-direction: column; }
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

.pane { position: relative; }
/* Floats under the pane, so showing it never moves anything else. */
.error {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  gap: 6px;
  margin-top: 4px;
  color: var(--red);
  text-align: center;
}
.error .icon { flex-shrink: 0; margin-top: 1px; }

/* Password row: opens under the email inside the same card. */
.reveal { display: grid; grid-template-rows: 0fr; transition: grid-template-rows 0.32s var(--ease); }
.reveal.open { grid-template-rows: 1fr; }
.reveal-inner { min-height: 0; overflow: hidden; }
.reveal-inner .row {
  opacity: 0;
  transition: opacity 0.2s;
}
.reveal.open .reveal-inner .row { opacity: 1; transition-delay: 0.08s; }
.reveal-inner .row::before {
  content: "";
  position: absolute;
  top: 0;
  right: 0;
  left: 20px;
  height: 0.5px;
  background: var(--separator);
}
.reserve { display: grid; grid-template-rows: 1fr; margin-top: -14px; visibility: hidden; transition: grid-template-rows 0.32s var(--ease); }
.reserve.open { grid-template-rows: 0fr; }
.invisible { visibility: hidden; }

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
.legal a { color: var(--label-2); text-decoration: none; font-weight: 500; transition: color 0.2s; }
.legal a:hover { color: var(--accent); }

/* Moving between steps: the old one fades out quickly, the new one rises in. */
.step-enter-active { transition: opacity 0.28s var(--ease), transform 0.34s var(--ease); }
.step-leave-active { transition: opacity 0.14s ease-in; }
.step-enter-from { opacity: 0; transform: translateY(10px); }
.step-leave-to { opacity: 0; }
/* Leaving or reaching the two-column start screen: the whole layout crossfades instead of jumping. */
.swap-enter-active { transition: opacity 0.3s var(--ease), transform 0.34s var(--ease); }
.swap-leave-active { transition: opacity 0.16s ease-in; }
.swap-enter-from { opacity: 0; transform: scale(0.985); }
.swap-leave-to { opacity: 0; }
/* The inner step transition stays still while the whole layout swaps. */
.swap-enter-active .step-enter-active, .swap-leave-active .step-leave-active { transition: none; }
@media (prefers-reduced-motion: reduce) {
  .step-enter-active, .swap-enter-active { transition: opacity 0.2s; }
  .step-enter-from, .swap-enter-from { transform: none; }
}
</style>
