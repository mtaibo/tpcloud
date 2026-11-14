<script setup>
import { computed, onMounted, ref } from 'vue'
import { Copy, Share } from 'lucide-vue-next'
import Sheet from '../ui/Sheet.vue'
import Spinner from '../ui/Spinner.vue'
import Toggle from '../ui/Toggle.vue'
import Segmented from '../ui/Segmented.vue'
import ListSection from '../ui/ListSection.vue'
import ListRow from '../ui/ListRow.vue'
import { confirmDestructive, toast } from '../ui/dialog.js'
import FileIcon from './FileIcon.vue'
import { t } from '../ui/i18n.js'
import { apiError } from '../apiErrors.js'

// Sharing an item: a new link (name, password, editing, public page), or the options of the link
// it already has. Opened with `entry` from a folder, or with `share` from the list of links.
const props = defineProps({
  entry: { type: Object, default: null },
  share: { type: Object, default: null },
  location: String,
  currentPath: String,
})
const emit = defineEmits(['close', 'changed'])

const open = ref(true)
const loading = ref(!props.share)
const existing = ref(props.share)
const busy = ref(false)
const error = ref('')

// New link
const slugMode = ref('custom')
const slug = ref('')
// Both
const usePassword = ref(false)
const password = ref('')
const editable = ref(false)
const isPublic = ref(false)

const name = computed(() => props.entry?.name || existing.value?.path.split('/').filter(Boolean).at(-1) || existing.value?.label || '')
const isFolder = computed(() => (props.entry ? props.entry.type === 'directory' : !existing.value?.is_file))
const entryPath = computed(() => (props.currentPath ? `${props.currentPath}/${props.entry.name}` : props.entry?.name))
const url = computed(() => existing.value?.url || (existing.value ? `${window.location.origin}/share/${existing.value.token}` : ''))

function fill(s) {
  editable.value = s.editable
  isPublic.value = s.public
  usePassword.value = s.has_password
  password.value = ''
}

onMounted(async () => {
  if (existing.value) { fill(existing.value); return }
  try {
    const params = new URLSearchParams({ path: entryPath.value, location: props.location })
    const res = await fetch(`/store/api/shares?${params}`)
    if (res.ok) existing.value = (await res.json())[0] ?? null
  } catch { /* offer a new link */ }
  if (existing.value) fill(existing.value)
  slug.value = props.entry.name.toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/[^a-z0-9-]/g, '-').replace(/-+/g, '-').replace(/^-|-$/g, '')
  loading.value = false
})

const dirty = computed(() => {
  const s = existing.value
  if (!s) return true
  return editable.value !== s.editable || isPublic.value !== s.public || usePassword.value !== s.has_password || (usePassword.value && password.value)
})
const canConfirm = computed(() => {
  if (loading.value) return false
  if (!existing.value) return (slugMode.value === 'hidden' || slug.value) && (!usePassword.value || password.value)
  if (usePassword.value && !existing.value.has_password && !password.value) return false
  return !!dirty.value
})

async function confirm() {
  busy.value = true
  error.value = ''
  try {
    if (!existing.value) {
      const res = await fetch('/store/api/shares', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          location: props.location,
          path: entryPath.value,
          hidden: slugMode.value === 'hidden',
          slug: slugMode.value === 'custom' ? slug.value : undefined,
          password: usePassword.value && password.value ? password.value : undefined,
          editable: editable.value,
          public: isPublic.value,
        }),
      })
      const data = await res.json().catch(() => ({}))
      if (!res.ok) throw new Error(apiError(data.detail) || t('Couldn’t create the link', 'No se pudo crear el enlace'))
      existing.value = data
      fill(data)
      navigator.clipboard?.writeText(data.url || `${window.location.origin}/share/${data.token}`)
      toast(t('Link created and copied', 'Enlace creado y copiado'))
      emit('changed')
      return
    }
    const res = await fetch(`/store/api/shares/${existing.value.token}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        editable: editable.value,
        public: isPublic.value,
        clear_password: !usePassword.value && existing.value.has_password,
        password: usePassword.value && password.value ? password.value : undefined,
      }),
    })
    const data = await res.json().catch(() => ({}))
    if (!res.ok) throw new Error(apiError(data.detail) || t('Couldn’t save', 'No se pudo guardar'))
    existing.value = data
    fill(data)
    toast(t('Link updated', 'Enlace actualizado'))
    emit('changed')
    open.value = false
    emit('close')
  } catch (e) {
    error.value = e.message
  } finally {
    busy.value = false
  }
}

async function remove() {
  const ok = await confirmDestructive(t('Delete the link?', '¿Eliminar el enlace?'), t('Anyone who has it won’t be able to open it any more. The files aren’t deleted.', 'Quien lo tenga dejará de poder abrirlo. Los archivos no se borran.'), t('Delete link', 'Eliminar enlace'))
  if (!ok) return
  const res = await fetch(`/store/api/shares/${existing.value.token}`, { method: 'DELETE' })
  if (res.ok) {
    toast(t('Link deleted', 'Enlace eliminado'))
    emit('changed')
    open.value = false
    emit('close')
  }
}

function copy() {
  navigator.clipboard.writeText(url.value)
  toast(t('Link copied', 'Enlace copiado'))
}

const canShare = typeof navigator.share === 'function'

async function shareNative() {
  try { await navigator.share({ title: name.value, url: url.value }) } catch { /* cancelled */ }
}
</script>

<template>
  <Sheet v-model:open="open" :title="existing ? t('Shared link', 'Enlace compartido') : t('Share', 'Compartir')" confirmable :can-confirm="!!canConfirm" :busy="busy" @confirm="confirm" @close="emit('close')">
    <div class="hero">
      <FileIcon :entry="entry || { type: isFolder ? 'directory' : 'file', name }" :size="64" />
      <p class="headline name">{{ name }}</p>
    </div>

    <div v-if="loading" class="loading"><Spinner :size="22" /></div>

    <template v-else>
      <template v-if="existing">
        <div class="link card">
          <span class="url mono">{{ url }}</span>
          <div class="link-actions">
            <button type="button" class="btn btn-bordered small" @click="copy"><Copy :size="15" /> {{ t('Copy', 'Copiar') }}</button>
            <button v-if="canShare" type="button" class="btn btn-bordered small" @click="shareNative"><Share :size="15" /> {{ t('Share', 'Compartir') }}</button>
          </div>
        </div>
      </template>

      <template v-else>
        <ListSection :header="t('Link address', 'Dirección del enlace')" :footer="slugMode === 'custom' ? t('Easy to remember and to read out.', 'Fácil de recordar y de dictar.') : t('Impossible to guess: only whoever gets the link can open it.', 'Imposible de adivinar: solo entra quien reciba el enlace.')">
          <ListRow>
            <Segmented v-model="slugMode" :options="[{ value: 'custom', label: t('Custom', 'Personalizada') }, { value: 'hidden', label: t('Random', 'Aleatoria') }]" class="seg" />
          </ListRow>
          <ListRow v-if="slugMode === 'custom'">
            <span class="slug">
              <span class="prefix">/share/</span>
              <input v-model="slug" class="row-input" placeholder="mallorca-2026" spellcheck="false" autocomplete="off" :aria-label="t('Address', 'Dirección')" />
            </span>
          </ListRow>
        </ListSection>
      </template>

      <ListSection :header="t('Access', 'Acceso')" :footer="isFolder ? t('With editing on, anyone with the link can upload, rename and delete inside the folder.', 'Con edición, quien tenga el enlace puede subir, renombrar y borrar dentro de la carpeta.') : ''">
        <ListRow :title="t('Protect with a password', 'Proteger con contraseña')">
          <template #trailing><Toggle v-model="usePassword" :label="t('Protect with a password', 'Proteger con contraseña')" /></template>
        </ListRow>
        <ListRow v-if="usePassword">
          <input
            v-model="password"
            class="row-input"
            type="password"
            :placeholder="existing?.has_password ? t('New password (optional)', 'Nueva contraseña (opcional)') : t('Password', 'Contraseña')"
            autocomplete="new-password"
            :aria-label="t('Password', 'Contraseña')"
          />
        </ListRow>
        <ListRow v-if="isFolder" :title="t('Allow editing', 'Permitir editar')">
          <template #trailing><Toggle v-model="editable" :label="t('Allow editing', 'Permitir editar')" /></template>
        </ListRow>
        <ListRow :title="t('Show on the public page', 'Mostrar en la página pública')">
          <template #trailing><Toggle v-model="isPublic" :label="t('Show on the public page', 'Mostrar en la página pública')" /></template>
        </ListRow>
      </ListSection>

      <p v-if="error" class="footnote danger err">{{ error }}</p>

      <ListSection v-if="existing">
        <ListRow :title="t('Delete link', 'Eliminar enlace')" clickable destructive centered @click="remove" />
      </ListSection>
    </template>
  </Sheet>
</template>

<style scoped>
.hero { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 0 0 22px; text-align: center; }
.name { overflow-wrap: anywhere; }
.loading { display: grid; place-items: center; padding: 30px; }
.link { display: flex; flex-direction: column; gap: 12px; padding: 16px 18px; margin-bottom: 26px; }
.url { font-size: 14px; overflow-wrap: anywhere; user-select: all; color: var(--label); }
.link-actions { display: flex; gap: 8px; flex-wrap: wrap; }
.link-actions .btn { gap: 6px; }
.seg { flex: 1; }
.slug { display: flex; align-items: center; gap: 2px; }
.prefix { color: var(--label-3); flex-shrink: 0; }
.err { text-align: center; margin: -12px 0 18px; }
</style>
