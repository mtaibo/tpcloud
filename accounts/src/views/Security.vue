<script setup>
import { computed, ref } from 'vue'
import QRCode from 'qrcode'
import Icon from '@ui/Icon.vue'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import Sheet from '@ui/Sheet.vue'
import { api } from '@ui/api.js'
import { confirmDestructive, toast } from '@ui/dialog.js'
import { ago, formatDate } from '@ui/format.js'
import { createCredential, passkeysSupported } from '@ui/webauthn.js'
import Page from '../components/Page.vue'
import { store } from '../store.js'

const profile = computed(() => store.profile)
const editing = ref(false)

// ---- Passkeys ----

const passkeySheet = ref(false)
const passkeyName = ref('')
const passkeyBusy = ref(false)
const passkeyError = ref('')

function openPasskeySheet() {
  passkeyName.value = ''
  passkeyError.value = ''
  passkeySheet.value = true
}

async function addPasskey() {
  passkeyBusy.value = true
  passkeyError.value = ''
  try {
    const options = await api('/auth/account/passkey/add/begin', { method: 'POST' })
    const credential = await createCredential(options)
    await api('/auth/account/passkey/add/complete', {
      method: 'POST',
      body: { ...credential, device_name: passkeyName.value.trim() },
    })
    await store.reload()
    passkeySheet.value = false
    toast('Passkey añadida')
  } catch (e) {
    if (e.name === 'InvalidStateError') passkeyError.value = 'Este dispositivo ya tiene una passkey de tu cuenta.'
    else if (e.name !== 'NotAllowedError') passkeyError.value = e.message
  } finally {
    passkeyBusy.value = false
  }
}

async function removePasskey(p) {
  const ok = await confirmDestructive(
    `¿Eliminar «${p.device_name}»?`,
    'Ya no podrás iniciar sesión con esta passkey. Puedes volver a crearla cuando quieras.',
  )
  if (!ok) return
  try {
    await api(`/auth/account/passkey/${encodeURIComponent(p.credential_id)}`, { method: 'DELETE' })
    await store.reload()
    if (!profile.value.passkeys.length) editing.value = false
    toast('Passkey eliminada')
  } catch (e) {
    toast(e.message, 'warning')
  }
}

// ---- Password ----

const passwordSheet = ref(false)
const current = ref('')
const next = ref('')
const repeat = ref('')
const passwordBusy = ref(false)
const passwordError = ref('')
const passwordReady = computed(() =>
  next.value.length >= 12 && next.value === repeat.value && (!profile.value.has_password || current.value),
)
const passwordHint = computed(() => {
  if (next.value && next.value.length < 12) return `Faltan ${12 - next.value.length} caracteres.`
  if (repeat.value && next.value !== repeat.value) return 'Las contraseñas no coinciden.'
  return ''
})

function openPasswordSheet() {
  current.value = next.value = repeat.value = ''
  passwordError.value = ''
  passwordSheet.value = true
}

async function savePassword() {
  if (!passwordReady.value) return
  passwordBusy.value = true
  passwordError.value = ''
  try {
    await api('/auth/account/password', {
      method: 'PATCH',
      body: { current_password: current.value, new_password: next.value },
    })
    await store.reload()
    passwordSheet.value = false
    toast('Contraseña actualizada')
  } catch (e) {
    passwordError.value = e.message
  } finally {
    passwordBusy.value = false
  }
}

// ---- Two-factor ----

const totpSheet = ref(false)
const totpSetup = ref(null)
const totpCode = ref('')
const totpBusy = ref(false)
const totpError = ref('')

async function openTotpSheet() {
  totpCode.value = ''
  totpError.value = ''
  totpSetup.value = null
  totpSheet.value = true
  if (profile.value.totp_enabled) return
  try {
    const data = await api('/auth/account/totp/setup/begin', { method: 'POST' })
    totpSetup.value = { ...data, qr: await QRCode.toDataURL(data.otpauth_uri, { margin: 1, width: 360 }) }
  } catch (e) {
    totpError.value = e.message
  }
}

async function confirmTotp() {
  if (totpCode.value.length !== 6) return
  totpBusy.value = true
  totpError.value = ''
  try {
    const enabling = !profile.value.totp_enabled
    await api(enabling ? '/auth/account/totp/setup/complete' : '/auth/account/totp', {
      method: enabling ? 'POST' : 'DELETE',
      body: { code: totpCode.value },
    })
    await store.reload()
    totpSheet.value = false
    toast(enabling ? 'Verificación en dos pasos activada' : 'Verificación en dos pasos desactivada')
  } catch (e) {
    totpError.value = e.message
    totpCode.value = ''
  } finally {
    totpBusy.value = false
  }
}

function copySecret() {
  navigator.clipboard?.writeText(totpSetup.value.secret)
  toast('Clave copiada')
}
</script>

<template>
  <Page title="Inicio de sesión y seguridad">
    <ListSection
      icons
      footer="Con una passkey entras con Face ID, Touch ID o el código de tu dispositivo, sin escribir nada. Se guardan en tu llavero y no se pueden robar con un correo falso."
    >
      <template #header>Passkeys</template>
      <template v-if="profile.passkeys.length" #action>
        <button class="btn-plain" type="button" @click="editing = !editing">{{ editing ? 'OK' : 'Editar' }}</button>
      </template>
      <ListRow
        v-for="p in profile.passkeys"
        :key="p.credential_id"
        icon="passkey"
        tint="green"
        :title="p.device_name"
        :subtitle="p.last_used_at ? `Usada ${ago(p.last_used_at)} · creada el ${formatDate(p.created_at)}` : `Sin usar · creada el ${formatDate(p.created_at)}`"
        small-subtitle
      >
        <template v-if="editing" #trailing>
          <button class="delete" type="button" :aria-label="`Eliminar ${p.device_name}`" @click="removePasskey(p)">
            <Icon name="minus" :size="16" :weight="3" />
          </button>
        </template>
      </ListRow>
      <ListRow
        v-if="passkeysSupported()"
        icon="plus"
        plain-icon
        title="Añadir una passkey"
        accent
        clickable
        @click="openPasskeySheet"
      />
      <ListRow v-else title="Este navegador no admite passkeys" />
    </ListSection>

    <ListSection
      header="Contraseña"
      footer="Con correo y contraseña puedes entrar desde cualquier sitio, también desde la app. Al cambiarla se cierran tus demás sesiones."
    >
      <ListRow
        icon="lock"
        tint="gray"
        :title="profile.has_password ? 'Cambiar contraseña' : 'Crear una contraseña'"
        clickable
        :chevron="true"
        @click="openPasswordSheet"
      />
    </ListSection>

    <ListSection
      header="Verificación en dos pasos"
      footer="Además de la contraseña, te pedirá un código de 6 cifras de una app de autenticación. Las passkeys y los códigos QR no lo necesitan."
    >
      <ListRow
        icon="shield"
        tint="blue"
        title="Código de verificación"
        :value="profile.totp_enabled ? 'Activado' : 'Desactivado'"
        clickable
        :chevron="true"
        @click="openTotpSheet"
      />
    </ListSection>

    <!-- Add a passkey -->
    <Sheet v-model:open="passkeySheet" title="Nueva passkey" confirmable :busy="passkeyBusy" @confirm="addPasskey">
      <div class="sheet-hero">
        <span class="hero-icon green"><Icon name="passkey" :size="34" /></span>
        <p class="subheadline secondary">Ponle un nombre para reconocerla, como «iPhone» o «MacBook». Después tu dispositivo te pedirá confirmarlo.</p>
      </div>
      <ListSection>
        <ListRow>
          <input v-model="passkeyName" class="row-input" placeholder="Nombre (opcional)" aria-label="Nombre de la passkey" @keydown.enter="addPasskey" />
        </ListRow>
      </ListSection>
      <p v-if="passkeyError" class="sheet-error footnote">{{ passkeyError }}</p>
    </Sheet>

    <!-- Password -->
    <Sheet
      v-model:open="passwordSheet"
      :title="profile.has_password ? 'Cambiar contraseña' : 'Crear contraseña'"
      confirmable
      :can-confirm="!!passwordReady"
      :busy="passwordBusy"
      @confirm="savePassword"
    >
      <form @submit.prevent="savePassword">
        <input type="email" autocomplete="username" :value="profile.email" hidden />
        <ListSection v-if="profile.has_password">
          <ListRow>
            <input v-model="current" class="row-input" type="password" placeholder="Contraseña actual" autocomplete="current-password" aria-label="Contraseña actual" />
          </ListRow>
        </ListSection>
        <ListSection footer="Al menos 12 caracteres. Una frase de varias palabras es más fácil de recordar y más difícil de adivinar.">
          <ListRow>
            <input v-model="next" class="row-input" type="password" placeholder="Nueva contraseña" autocomplete="new-password" aria-label="Nueva contraseña" />
          </ListRow>
          <ListRow>
            <input v-model="repeat" class="row-input" type="password" placeholder="Repítela" autocomplete="new-password" aria-label="Repite la contraseña" />
          </ListRow>
        </ListSection>
        <button type="submit" hidden />
      </form>
      <p v-if="passwordError || passwordHint" class="sheet-error footnote">{{ passwordError || passwordHint }}</p>
    </Sheet>

    <!-- Two-factor -->
    <Sheet
      v-model:open="totpSheet"
      title="Verificación en dos pasos"
      confirmable
      :can-confirm="totpCode.length === 6"
      :busy="totpBusy"
      @confirm="confirmTotp"
    >
      <template v-if="!profile.totp_enabled">
        <div class="sheet-hero">
          <div v-if="totpSetup" class="totp-qr"><img :src="totpSetup.qr" alt="Código QR para tu app de autenticación" /></div>
          <div v-else class="totp-qr placeholder" />
          <p class="subheadline secondary">Escanéalo con tu app de autenticación (Contraseñas de Apple, Google Authenticator, 1Password…) y escribe el código que te muestre.</p>
        </div>
        <ListSection v-if="totpSetup">
          <ListRow title="Clave" clickable :chevron="false" @click="copySecret">
            <template #trailing><span class="row-value mono secret">{{ totpSetup.secret }}</span></template>
          </ListRow>
        </ListSection>
      </template>
      <div v-else class="sheet-hero">
        <span class="hero-icon blue"><Icon name="shield" :size="34" /></span>
        <p class="subheadline secondary">Para desactivarla, escribe el código que te muestra ahora tu app de autenticación.</p>
      </div>
      <ListSection>
        <ListRow>
          <input
            v-model="totpCode"
            class="row-input code-input"
            inputmode="numeric"
            autocomplete="one-time-code"
            maxlength="6"
            placeholder="000000"
            aria-label="Código"
            @input="totpCode = totpCode.replace(/\D/g, '')"
            @keydown.enter="confirmTotp"
          />
        </ListRow>
      </ListSection>
      <p v-if="totpError" class="sheet-error footnote">{{ totpError }}</p>
    </Sheet>
  </Page>
</template>

<style scoped>
.delete {
  width: 24px;
  height: 24px;
  display: grid;
  place-items: center;
  border-radius: 50%;
  color: #fff;
  background: var(--red);
  animation: pop-in 0.3s var(--spring);
}
@keyframes pop-in { from { transform: scale(0); } }
.sheet-hero { display: flex; flex-direction: column; align-items: center; gap: 14px; text-align: center; padding: 6px 8px 22px; }
.hero-icon { width: 72px; height: 72px; display: grid; place-items: center; border-radius: 20px; color: #fff; }
.hero-icon.green { background: var(--green); }
.hero-icon.blue { background: var(--blue); }
.totp-qr { width: 196px; height: 196px; padding: 10px; border-radius: 24px; background: #fff; }
.totp-qr img { width: 100%; height: 100%; display: block; image-rendering: pixelated; }
.totp-qr.placeholder { background: var(--fill-3); }
.secret { max-width: 65%; font-size: 13px; overflow-wrap: anywhere; white-space: normal; }
.code-input { font-family: var(--font-rounded); font-size: 24px !important; letter-spacing: 6px !important; text-align: center; }
.sheet-error { color: var(--red); text-align: center; margin-top: -12px; padding: 0 20px; }
</style>
