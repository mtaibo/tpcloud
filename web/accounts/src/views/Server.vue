<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import Spinner from '@ui/Spinner.vue'
import { api } from '@ui/api.js'
import { ask } from '@ui/dialog.js'
import Page from '../components/Page.vue'
import { t } from '@ui/i18n.js'

// The host's own version and updating it from here: the server downloads the new TPCloud, checks
// it against GitHub's checksum, installs it and restarts by itself (as `sudo tpcloud update`).

const host = ref(null)
const busy = ref(false)
const error = ref('')
let timer = null

const STEPS = {
  checking: t('Looking for the new version…', 'Buscando la versión nueva…'),
  downloading: t('Downloading…', 'Descargando…'),
  installing: t('Installing and restarting…', 'Instalando y reiniciando…'),
}
const updating = computed(() => !!host.value?.update && host.value.update.state in STEPS)

async function load() {
  try {
    host.value = await api('/auth/admin/host')
    error.value = ''
  } catch (e) {
    // While it restarts the server doesn't answer: keep waiting.
    if (!updating.value) error.value = e.message
  }
  clearTimeout(timer)
  if (updating.value) timer = setTimeout(load, 1500)
}

async function check() {
  busy.value = true
  try {
    await api('/auth/admin/host/check', { method: 'POST' })
    await new Promise(r => setTimeout(r, 2500))
    await load()
  } catch (e) {
    error.value = e.message
  } finally {
    busy.value = false
  }
}

async function update() {
  const ok = await ask({
    title: t(`Update to TPCloud ${host.value.latest}?`, `¿Actualizar a TPCloud ${host.value.latest}?`),
    message: t('Everything stops for a few seconds while it restarts. Nothing is lost.', 'Todo se para unos segundos mientras se reinicia. No se pierde nada.'),
    actions: [
      { label: t('Cancel', 'Cancelar'), role: 'cancel', value: false },
      { label: t('Update', 'Actualizar'), value: true },
    ],
  })
  if (!ok) return
  busy.value = true
  try {
    await api('/auth/admin/host/update', { method: 'POST' })
    host.value = { ...host.value, update: { state: 'checking' } }
    load()
  } catch (e) {
    error.value = e.message
  } finally {
    busy.value = false
  }
}

const status = computed(() => {
  const h = host.value
  if (!h) return ''
  const u = h.update
  if (u?.state === 'done') return t(`Updated to ${u.detail}.`, `Actualizado a ${u.detail}.`)
  if (u?.state === 'failed') return t(`The update failed: ${u.detail}`, `La actualización falló: ${u.detail}`)
  if (h.available) return t(`TPCloud ${h.latest} is out.`, `Ya está TPCloud ${h.latest}.`)
  return t('This is the latest version.', 'Es la versión más reciente.')
})

onMounted(load)
onBeforeUnmount(() => clearTimeout(timer))
</script>

<template>
  <Page :title="t('Server', 'Servidor')" :subtitle="host ? `TPCloud ${host.version}` : ''">
    <div v-if="!host && !error" class="center"><Spinner :size="26" /></div>
    <template v-else-if="host">
      <ListSection :footer="status">
        <ListRow :title="t('Version', 'Versión')" :value="host.version" />
        <ListRow v-if="host.latest" :title="t('Newest', 'Más reciente')" :value="host.latest" />
        <ListRow v-if="!updating" :title="t('Look for updates', 'Buscar actualizaciones')" clickable accent :disabled="busy" @click="check">
          <template #trailing><Spinner v-if="busy" :size="16" /></template>
        </ListRow>
      </ListSection>

      <ListSection v-if="updating" :footer="t('The server restarts by itself at the end; this page carries on once it\'s back.', 'El servidor se reinicia solo al final; esta página sigue cuando vuelva.')">
        <ListRow :title="STEPS[host.update.state]"><template #trailing><Spinner :size="16" /></template></ListRow>
      </ListSection>
      <ListSection v-else-if="host.available && host.can_update" :footer="t('It downloads the new version, checks it against GitHub\'s checksum and installs it. Accounts, files and settings stay as they are.', 'Descarga la versión nueva, la comprueba con la suma de GitHub y la instala. Cuentas, archivos y ajustes se quedan como están.')">
        <ListRow :title="t(`Update to ${host.latest}`, `Actualizar a ${host.latest}`)" clickable accent :disabled="busy" @click="update" />
      </ListSection>
      <p v-if="!host.can_update" class="footnote secondary pad">{{ t('This server isn\'t run by the tpcloud command, so it\'s updated where it runs.', 'Este servidor no lo ejecuta el comando tpcloud, así que se actualiza donde corre.') }}</p>
    </template>
    <p v-if="error" class="footnote danger pad">{{ error }}</p>
  </Page>
</template>

<style scoped>
.center { display: flex; justify-content: center; padding: 40px 0; }
.pad { padding: 0 16px; }
</style>
