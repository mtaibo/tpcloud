<script setup>
import { computed, onMounted, ref } from 'vue'
import { Copy, FolderOpen, Link, Lock, MoreHorizontal, Pencil, Trash2 } from 'lucide-vue-next'
import Spinner from '@ui/Spinner.vue'
import ListSection from '@ui/ListSection.vue'
import { confirmDestructive, toast } from '@ui/dialog.js'
import { openMenu } from '../menu.js'
import { navigate } from '../store.js'
import FileIcon from './FileIcon.vue'
import ShareSheet from './ShareSheet.vue'
import { t } from '@ui/i18n.js'

// Every link you've shared, with what it allows, and a way to copy, change or delete it.
const shares = ref(null)
const editing = ref(null)

async function load() {
  const res = await fetch('/store/api/shares')
  shares.value = res.ok ? await res.json() : []
}
onMounted(load)

const nameOf = s => s.path.split('/').filter(Boolean).at(-1) || s.token
const sorted = computed(() => [...(shares.value || [])].sort((a, b) => nameOf(a).localeCompare(nameOf(b), ['es', 'en'])))

function copy(s) {
  navigator.clipboard.writeText(s.url)
  toast(t('Link copied', 'Enlace copiado'))
}

function folderOf(s) {
  return s.is_file ? s.path.split('/').slice(0, -1).join('/') : s.path
}

async function remove(s) {
  const ok = await confirmDestructive(t(`Delete the link to “${nameOf(s)}”?`, `¿Eliminar el enlace de «${nameOf(s)}»?`), t('Anyone who has it won’t be able to open it any more. The files aren’t deleted.', 'Quien lo tenga dejará de poder abrirlo. Los archivos no se borran.'), t('Delete link', 'Eliminar enlace'))
  if (!ok) return
  const res = await fetch(`/store/api/shares/${s.token}`, { method: 'DELETE' })
  if (res.ok) { toast(t('Link deleted', 'Enlace eliminado')); load() }
}

function menu(s, e) {
  e.preventDefault()
  openMenu(e.currentTarget instanceof Element && e.type === 'click' ? e.currentTarget : e, [
    { label: t('Copy link', 'Copiar enlace'), icon: Copy, action: () => copy(s) },
    { label: t('Show in Files', 'Mostrar en Archivos'), icon: FolderOpen, action: () => navigate(s.location, folderOf(s)) },
    { label: t('Options…', 'Opciones…'), icon: Pencil, action: () => { editing.value = s } },
    { separator: true },
    { label: t('Delete link', 'Eliminar enlace'), icon: Trash2, danger: true, action: () => remove(s) },
  ])
}
</script>

<template>
  <section class="shares">
    <header class="bar">
      <h1 class="bar-title">{{ t('Shared links', 'Enlaces compartidos') }}</h1>
    </header>
    <div class="content">
      <div class="page-head">
        <h1 class="large-title">{{ t('Shared links', 'Enlaces compartidos') }}</h1>
        <p class="subheadline secondary">{{ t('What you’ve shared with a link. Anyone who has it can open it without an account.', 'Lo que has compartido con un enlace. Quien lo tenga puede abrirlo sin cuenta.') }}</p>
      </div>

      <div v-if="!shares" class="state"><Spinner :size="24" /></div>
      <div v-else-if="!shares.length" class="state">
        <span class="empty-icon"><Link :size="34" /></span>
        <h3 class="headline">{{ t('You haven’t shared anything yet', 'Aún no has compartido nada') }}</h3>
        <p class="subheadline secondary">{{ t('Open a folder’s or file’s menu and choose “Share…”.', 'Abre el menú de una carpeta o un archivo y elige «Compartir…».') }}</p>
      </div>
      <ListSection v-else :footer="`${shares.length} ${shares.length === 1 ? t('link', 'enlace') : t('links', 'enlaces')}`">
        <div v-for="s in sorted" :key="s.token" class="row share-row" @click="editing = s" @contextmenu="menu(s, $event)">
          <FileIcon :entry="{ type: s.is_file ? 'file' : 'directory', name: nameOf(s) }" :size="40" />
          <span class="row-body">
            <span class="row-title">{{ nameOf(s) }}</span>
            <span class="row-subtitle small url">{{ s.url }}</span>
            <span class="tags">
              <span v-if="s.has_password" class="badge"><Lock :size="10" :stroke-width="2.6" /> {{ t('Password', 'Contraseña') }}</span>
              <span class="badge" :class="{ green: s.editable }">{{ s.editable ? t('Can edit', 'Se puede editar') : t('Read only', 'Solo lectura') }}</span>
              <span v-if="s.public" class="badge orange">{{ t('Public page', 'Página pública') }}</span>
            </span>
          </span>
          <button type="button" class="icon-action" :title="t('Copy link', 'Copiar enlace')" @click.stop="copy(s)"><Copy :size="17" /></button>
          <button type="button" class="icon-action" :title="t('More', 'Más')" @click.stop="menu(s, $event)"><MoreHorizontal :size="18" /></button>
        </div>
      </ListSection>
    </div>

    <ShareSheet v-if="editing" :share="editing" @close="editing = null" @changed="load" />
  </section>
</template>

<style scoped>
.shares { flex: 1; min-width: 0; display: flex; flex-direction: column; overflow: hidden; }
.bar { height: 64px; flex-shrink: 0; display: flex; align-items: center; padding: 0 24px; }
.bar-title { font-size: 17px; font-weight: 700; letter-spacing: -0.3px; }
.content { flex: 1; overflow-y: auto; padding: 0 24px 40px; }
.page-head { max-width: 760px; margin: 0 auto; padding: 8px 4px 20px; }
.page-head .large-title { display: none; }
.content :deep(.section) { max-width: 760px; margin-left: auto; margin-right: auto; }
.state { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 70px 24px; text-align: center; }
.empty-icon { width: 72px; height: 72px; display: grid; place-items: center; border-radius: 22px; color: #fff; background: var(--blue); margin-bottom: 6px; }
.share-row { --inset: 74px; cursor: pointer; padding-top: 12px; padding-bottom: 12px; }
.share-row:hover { background: color-mix(in srgb, var(--cell) 94%, var(--label)); }
.url { font-family: var(--font-mono); font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 4px; }
.icon-action { width: 34px; height: 34px; display: grid; place-items: center; border-radius: 50%; color: var(--accent); flex-shrink: 0; }
.icon-action:hover { background: var(--fill-3); }
@media (max-width: 767px) {
  .bar { display: none; }
  .content { padding: calc(20px + env(safe-area-inset-top)) 16px calc(110px + env(safe-area-inset-bottom)); }
  .page-head { padding: 24px 4px 18px; }
  .page-head .large-title { display: block; }
  .icon-action:first-of-type { display: none; }
}
</style>
