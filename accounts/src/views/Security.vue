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
import { t } from '@ui/i18n.js'

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
    toast(t('Passkey added', 'Passkey añadida'))
  } catch (e) {
    if (e.name === 'InvalidStateError') passkeyError.value = t('This device already has a passkey for your account.', 'Este dispositivo ya tiene una passkey de tu cuenta.')
    else if (e.name !== 'NotAllowedError') passkeyError.value = e.message
  } finally {
    passkeyBusy.value = false
  }
}

async function removePasskey(p) {
  const ok = await confirmDestructive(
    t(`Delete “${p.device_name}”?`, `¿Eliminar «${p.device_name}»?`),
    t('You won’t be able to sign in with this passkey any more. You can create it again whenever you like.', 'Ya no podrás iniciar sesión con esta passkey. Puedes volver a crearla cuando quieras.'),
  )
  if (!ok) return
  try {
    await api(`/auth/account/passkey/${encodeURIComponent(p.credential_id)}`, { method: 'DELETE' })
    await store.reload()
    if (!profile.value.passkeys.length) editing.value = false
    toast(t('Passkey deleted', 'Passkey eliminada'))
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
  if (next.value && next.value.length < 12) return t(`${12 - next.value.length} more characters.`, `Faltan ${12 - next.value.length} caracteres.`)
  if (repeat.value && next.value !== repeat.value) return t('The passwords don’t match.', 'Las contraseñas no coinciden.')
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
    toast(t('Password updated', 'Contraseña actualizada'))
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
    toast(enabling ? t('Two-step verification turned on', 'Verificación en dos pasos activada') : t('Two-step verification turned off', 'Verificación en dos pasos desactivada'))
  } catch (e) {
    totpError.value = e.message
    totpCode.value = ''
  } finally {
    totpBusy.value = false
  }
}

function copySecret() {
  navigator.clipboard?.writeText(totpSetup.value.secret)
  toast(t('Key copied', 'Clave copiada'))
}
</script>

<template>
  <Page :title="t('Security', 'Seguridad')">
    <ListSection
      icons
      :footer="t('With a passkey you sign in with Face ID, Touch ID or your device’s passcode, without typing anything. They’re kept in your keychain and can’t be stolen with a fake email.', 'Con una passkey entras con Face ID, Touch ID o el código de tu dispositivo, sin escribir nada. Se guardan en tu llavero y no se pueden robar con un correo falso.')"
    >
      <template #header>Passkeys</template>
      <template v-if="profile.passkeys.length" #action>
        <button class="btn-plain" type="button" @click="editing = !editing">{{ editing ? t('Done', 'OK') : t('Edit', 'Editar') }}</button>
      </template>
      <ListRow
        v-for="p in profile.passkeys"
        :key="p.credential_id"
        icon="passkey"
        tint="green"
        :title="p.device_name"
        :subtitle="p.last_used_at ? t(`Used ${ago(p.last_used_at)} · created on ${formatDate(p.created_at)}`, `Usada ${ago(p.last_used_at)} · creada el ${formatDate(p.created_at)}`) : t(`Never used · created on ${formatDate(p.created_at)}`, `Sin usar · creada el ${formatDate(p.created_at)}`)"
        small-subtitle
      >
        <template v-if="editing" #trailing>
          <button class="delete" type="button" :aria-label="t(`Delete ${p.device_name}`, `Eliminar ${p.device_name}`)" @click="removePasskey(p)">
            <Icon name="minus" :size="16" :weight="3" />
          </button>
        </template>
      </ListRow>
      <ListRow
        v-if="passkeysSupported()"
        icon="plus"
        plain-icon
        :title="t('Add a passkey', 'Añadir una passkey')"
        accent
        clickable
        @click="openPasskeySheet"
      />
      <ListRow v-else :title="t('This browser doesn’t support passkeys', 'Este navegador no admite passkeys')" />
    </ListSection>

    <ListSection
      :header="t('Password', 'Contraseña')"
      :footer="t('With your email and password you can sign in from anywhere, the app included. Changing it signs out your other sessions.', 'Con correo y contraseña puedes entrar desde cualquier sitio, también desde la app. Al cambiarla se cierran tus demás sesiones.')"
    >
      <ListRow
        icon="lock"
        tint="gray"
        :title="profile.has_password ? t('Change password', 'Cambiar contraseña') : t('Create a password', 'Crear una contraseña')"
        clickable
        :chevron="true"
        @click="openPasswordSheet"
      />
    </ListSection>

    <ListSection
      :header="t('Two-step verification', 'Verificación en dos pasos')"
      :footer="t('Besides the password, you’ll be asked for a 6-digit code from an authenticator app. Passkeys and QR codes don’t need it.', 'Además de la contraseña, te pedirá un código de 6 cifras de una app de autenticación. Las passkeys y los códigos QR no lo necesitan.')"
    >
      <ListRow
        icon="shield"
        tint="blue"
        :title="t('Verification code', 'Código de verificación')"
        :value="profile.totp_enabled ? t('On', 'Activado') : t('Off', 'Desactivado')"
        clickable
        :chevron="true"
        @click="openTotpSheet"
      />
    </ListSection>

    <!-- Add a passkey -->
    <Sheet v-model:open="passkeySheet" :title="t('New passkey', 'Nueva passkey')" confirmable :busy="passkeyBusy" @confirm="addPasskey">
      <div class="sheet-hero">
        <span class="hero-icon green"><Icon name="passkey" :size="34" /></span>
        <p class="subheadline secondary">{{ t('Give it a name you’ll recognise, like “iPhone” or “MacBook”. Your device will then ask you to confirm.', 'Ponle un nombre para reconocerla, como «iPhone» o «MacBook». Después tu dispositivo te pedirá confirmarlo.') }}</p>
      </div>
      <ListSection>
        <ListRow>
          <input v-model="passkeyName" class="row-input" :placeholder="t('Name (optional)', 'Nombre (opcional)')" :aria-label="t('Passkey name', 'Nombre de la passkey')" @keydown.enter="addPasskey" />
        </ListRow>
      </ListSection>
      <p v-if="passkeyError" class="sheet-error footnote">{{ passkeyError }}</p>
    </Sheet>

    <!-- Password -->
    <Sheet
      v-model:open="passwordSheet"
      :title="profile.has_password ? t('Change password', 'Cambiar contraseña') : t('Create password', 'Crear contraseña')"
      confirmable
      :can-confirm="!!passwordReady"
      :busy="passwordBusy"
      @confirm="savePassword"
    >
      <form @submit.prevent="savePassword">
        <input type="email" autocomplete="username" :value="profile.email" hidden />
        <ListSection v-if="profile.has_password">
          <ListRow>
            <input v-model="current" class="row-input" type="password" :placeholder="t('Current password', 'Contraseña actual')" autocomplete="current-password" :aria-label="t('Current password', 'Contraseña actual')" />
          </ListRow>
        </ListSection>
        <ListSection :footer="t('At least 12 characters. A phrase of a few words is easier to remember and harder to guess.', 'Al menos 12 caracteres. Una frase de varias palabras es más fácil de recordar y más difícil de adivinar.')">
          <ListRow>
            <input v-model="next" class="row-input" type="password" :placeholder="t('New password', 'Nueva contraseña')" autocomplete="new-password" :aria-label="t('New password', 'Nueva contraseña')" />
          </ListRow>
          <ListRow>
            <input v-model="repeat" class="row-input" type="password" :placeholder="t('Repeat it', 'Repítela')" autocomplete="new-password" :aria-label="t('Repeat the password', 'Repite la contraseña')" />
          </ListRow>
        </ListSection>
        <button type="submit" hidden />
      </form>
      <p v-if="passwordError || passwordHint" class="sheet-error footnote">{{ passwordError || passwordHint }}</p>
    </Sheet>

    <!-- Two-factor -->
    <Sheet
      v-model:open="totpSheet"
      :title="t('Two-step verification', 'Verificación en dos pasos')"
      confirmable
      :can-confirm="totpCode.length === 6"
      :busy="totpBusy"
      @confirm="confirmTotp"
    >
      <template v-if="!profile.totp_enabled">
        <div class="sheet-hero">
          <div v-if="totpSetup" class="totp-qr"><img :src="totpSetup.qr" :alt="t('QR code for your authenticator app', 'Código QR para tu app de autenticación')" /></div>
          <div v-else class="totp-qr placeholder" />
          <p class="subheadline secondary">{{ t('Scan it with your authenticator app (Apple Passwords, Google Authenticator, 1Password…) and type the code it shows.', 'Escanéalo con tu app de autenticación (Contraseñas de Apple, Google Authenticator, 1Password…) y escribe el código que te muestre.') }}</p>
        </div>
        <ListSection v-if="totpSetup">
          <ListRow :title="t('Key', 'Clave')" clickable :chevron="false" @click="copySecret">
            <template #trailing><span class="row-value mono secret">{{ totpSetup.secret }}</span></template>
          </ListRow>
        </ListSection>
      </template>
      <div v-else class="sheet-hero">
        <span class="hero-icon blue"><Icon name="shield" :size="34" /></span>
        <p class="subheadline secondary">{{ t('To turn it off, type the code your authenticator app shows now.', 'Para desactivarla, escribe el código que te muestra ahora tu app de autenticación.') }}</p>
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
            :aria-label="t('Code', 'Código')"
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
