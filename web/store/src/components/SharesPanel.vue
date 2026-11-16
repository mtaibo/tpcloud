<script setup>
import { computed, onMounted, ref, watch } from 'vue'
import { ChevronRight, Copy, FolderOpen, Globe, Link, Lock, MoreHorizontal, Pencil, Trash2 } from 'lucide-vue-next'
import Spinner from '@ui/Spinner.vue'
import { confirmDestructive, toast } from '@ui/dialog.js'
import { openMenu } from '../menu.js'
import { navigate } from '../store.js'
import FileIcon from './FileIcon.vue'
import ShareSheet from './ShareSheet.vue'
import { t } from '@ui/i18n.js'

// The lower half of Shared: every link you've made, with what it allows and a way to copy,
// change or delete it, then the public links other people have listed. `version` changes when
// the folder above reloads, so a link made or changed there shows up here too.
const props = defineProps({ version: { type: Number, default: 0 } })
const emit = defineEmits(['open'])

const shares = ref(null)
const others = ref([])
const editing = ref(null)

async function load() {
  const [mine, listed] = await Promise.all([
    fetch('/store/api/shares').then(r => (r.ok ? r.json() : [])).catch(() => []),
    fetch('/store/api/share').then(r => (r.ok ? r.json() : [])).catch(() => []),
  ])
  const own = new Set(mine.map(s => s.token))
  shares.value = mine
  others.value = listed.filter(s => !own.has(s.token))
}
onMounted(load)
watch(() => props.version, load)

const nameOf = s => s.path?.split('/').filter(Boolean).at(-1) || s.label || s.token
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

function otherMenu(s, e) {
  e.preventDefault()
  openMenu(e.currentTarget instanceof Element && e.type === 'click' ? e.currentTarget : e, [
    { label: t('Open', 'Abrir'), icon: FolderOpen, action: () => openOther(s) },
    { label: t('Copy link', 'Copiar enlace'), icon: Copy, action: () => copy(s) },
  ])
}

function openOther(s) {
  emit('open', { type: 'share-link', name: s.label, token: s.token, url: s.url, share: { ...s, isOwn: false } })
}
</script>

<template>
  <section class="links">
    <header class="links-head">
      <Link :size="15" :stroke-width="2.2" class="links-icon" />
      <h2>{{ t('Shared links', 'Enlaces compartidos') }}</h2>
      <span v-if="shares?.length" class="links-count">{{ shares.length }}</span>
    </header>

    <div class="links-body">
      <div v-if="!shares" class="state"><Spinner :size="20" /></div>
      <template v-else>
        <p v-if="!shares.length" class="hint">{{ t('Nothing shared with a link yet. Open a folder’s or file’s menu and choose “Share…”.', 'Aún no has compartido nada con un enlace. Abre el menú de una carpeta o un archivo y elige «Compartir…».') }}</p>
        <div v-else class="group">
          <div v-for="s in sorted" :key="s.token" class="link-row" @click="editing = s" @contextmenu="menu(s, $event)">
            <FileIcon :entry="{ type: s.is_file ? 'file' : 'directory', name: nameOf(s) }" :size="32" />
            <span class="link-body">
              <span class="link-title">
                {{ nameOf(s) }}
                <Lock v-if="s.has_password" :size="11" :stroke-width="2.6" class="lock" :aria-label="t('Password', 'Contraseña')" />
              </span>
              <span class="link-url">{{ s.url }}</span>
            </span>
            <span class="tags">
              <span class="badge" :class="{ green: s.editable }">{{ s.editable ? t('Can edit', 'Se puede editar') : t('Read only', 'Solo lectura') }}</span>
              <span v-if="s.public" class="badge orange">{{ t('Public', 'Pública') }}</span>
            </span>
            <button type="button" class="icon-action copy" :title="t('Copy link', 'Copiar enlace')" @click.stop="copy(s)"><Copy :size="16" /></button>
            <button type="button" class="icon-action" :title="t('More', 'Más')" @click.stop="menu(s, $event)"><MoreHorizontal :size="17" /></button>
          </div>
        </div>

        <template v-if="others.length">
          <p class="sub-head">{{ t('Public links from others', 'Enlaces públicos de otros') }}</p>
          <div class="group">
            <div v-for="s in others" :key="s.token" class="link-row" @click="openOther(s)" @contextmenu="otherMenu(s, $event)">
              <span class="globe"><Globe :size="17" /></span>
              <span class="link-body">
                <span class="link-title">{{ s.label }}</span>
                <span class="link-url">{{ s.url }}</span>
              </span>
              <ChevronRight :size="16" class="chevron" />
            </div>
          </div>
        </template>
      </template>
    </div>

    <ShareSheet v-if="editing" :share="editing" @close="editing = null" @changed="load" />
  </section>
</template>

<style scoped>
.links { display: flex; flex-direction: column; min-height: 0; }
.links-head { flex-shrink: 0; display: flex; align-items: center; gap: 7px; padding: 12px 20px 8px; }
.links-head h2 { font-size: 13px; font-weight: 700; letter-spacing: -0.1px; }
.links-icon { color: var(--accent); }
.links-count { min-width: 20px; height: 18px; padding: 0 6px; display: grid; place-items: center; border-radius: 9px; font-size: 11px; font-weight: 700; color: var(--label-2); background: var(--fill-3); }
.links-body { flex: 1; min-height: 0; overflow-y: auto; padding: 0 12px 16px; }
.state { display: grid; place-items: center; padding: 28px; }
.hint { padding: 4px 8px 8px; font-size: 13px; color: var(--label-2); }
.sub-head { padding: 14px 8px 6px; font-size: 11px; font-weight: 700; color: var(--label-3); }
.group { border-radius: 12px; overflow: hidden; background: var(--cell); box-shadow: 0 0 0 0.5px var(--separator); }
.link-row { position: relative; display: flex; align-items: center; gap: 12px; min-height: 52px; padding: 6px 8px 6px 12px; cursor: default; }
.link-row + .link-row::before { content: ""; position: absolute; top: 0; left: 56px; right: 0; height: 0.5px; background: var(--separator); }
.link-row:hover { background: color-mix(in srgb, var(--cell) 94%, var(--label)); }
.link-body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
.link-title { display: flex; align-items: center; gap: 5px; font-size: 13.5px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.lock { flex-shrink: 0; color: var(--label-2); }
.link-url { font-family: var(--font-mono); font-size: 11.5px; color: var(--label-2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.tags { display: flex; gap: 4px; flex-shrink: 0; }
.icon-action { width: 30px; height: 30px; display: grid; place-items: center; border-radius: 50%; color: var(--accent); flex-shrink: 0; }
.icon-action:hover { background: var(--fill-3); }
.globe { width: 32px; height: 32px; display: grid; place-items: center; flex-shrink: 0; border-radius: 9px; color: #fff; background: var(--orange); }
.chevron { flex-shrink: 0; color: var(--label-3); }
@media (max-width: 767px) {
  .links-head { padding: 22px 4px 8px; }
  .links-head h2 { font-size: 20px; font-weight: 700; letter-spacing: -0.4px; }
  .links-icon { display: none; }
  .links-body { padding: 0; overflow: visible; }
  .group { border-radius: var(--radius-card); box-shadow: none; }
  .link-row { min-height: 60px; padding: 8px 8px 8px 16px; }
  .link-row + .link-row::before { left: 60px; }
  .link-title { font-size: 16px; }
  .tags, .copy { display: none; }
}
</style>
