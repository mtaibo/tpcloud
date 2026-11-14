<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { ChevronRight, Cloud, DatabaseBackup, Folder, House, Images, Link, LogOut, Plus, User, Users, X } from 'lucide-vue-next'
import { useFavourites } from '../useFavourites.js'
import { useTransfers } from '../useTransfers.js'
import { useCompression } from '../useCompression.js'
import { FOLDER_ICONS_MAP } from '../folderIcons.js'
import { openMenu } from '../menu.js'
import { folderTitle, homePath, navigate, store } from '../store.js'
import { formatSize } from '../utils/formatSize.js'
import { toast } from '../ui/dialog.js'
import { transferSummary } from '../transferSummary.js'
import TransferPanel from './TransferPanel.vue'
import BackupSheet from './BackupSheet.vue'
import { t } from '../ui/i18n.js'

const favouritesOpen = ref(true)
const transfersOpen = ref(false)
const transfersRef = ref(null)
const showBackup = ref(false)

const { favourites, load, add, remove } = useFavourites()
const { fetchPendingUploads } = useTransfers()
const { init: initCompression } = useCompression()
const summary = transferSummary()

const initials = computed(() => (store.user?.display_name || '?').split(/\s+/).slice(0, 2).map(w => w[0]?.toUpperCase()).join(''))
const usedPct = computed(() => (store.disk ? Math.round((store.disk.used / (store.disk.used + store.disk.free)) * 100) : 0))

function isActive(loc, path) {
  return store.panel === 'files' && store.location === loc && store.path === path
}

function addCurrent() {
  const added = add(store.location, store.path, store.user.email)
  toast(added ? t('Added to Favourites', 'Añadida a Favoritos') : t('Already in Favourites', 'Ya está en Favoritos'), added ? 'check-circle' : 'info')
}

function userMenu(e) {
  openMenu(e.currentTarget, [
    { label: t('Your account', 'Tu cuenta'), icon: User, action: () => { window.location.href = '/accounts' } },
    { label: t('Back to TPCloud', 'Volver a TPCloud'), icon: Cloud, action: () => { window.location.href = '/' } },
    store.user.is_admin && { separator: true },
    store.user.is_admin && { label: t('Backup…', 'Copia de seguridad…'), icon: DatabaseBackup, action: () => { showBackup.value = true } },
    { separator: true },
    { label: t('Log out', 'Cerrar sesión'), icon: LogOut, danger: true, action: logout },
  ])
}

async function logout() {
  await fetch('/auth/passkey/logout', { method: 'POST' })
  window.location.href = '/login'
}

function onDocDown(e) {
  if (transfersOpen.value && transfersRef.value && !transfersRef.value.contains(e.target) && !e.target.closest('.tp')) transfersOpen.value = false
}

onMounted(() => {
  document.addEventListener('pointerdown', onDocDown)
  load()
  fetchPendingUploads()
  initCompression()
})
onUnmounted(() => document.removeEventListener('pointerdown', onDocDown))
</script>

<template>
  <aside class="sidebar">
    <a href="/" class="brand" :title="t('Back to TPCloud', 'Volver a TPCloud')">
      <span class="brand-icon"><Cloud :size="15" :stroke-width="2.4" /></span>
      <span>{{ t('Files', 'Archivos') }}</span>
    </a>

    <nav class="nav">
      <div class="group-head" @click="favouritesOpen = !favouritesOpen">
        <span>{{ t('Favourites', 'Favoritos') }}</span>
        <button type="button" class="head-btn" :title="t('Add this folder', 'Añadir esta carpeta')" @click.stop="addCurrent"><Plus :size="14" :stroke-width="2.4" /></button>
        <ChevronRight :size="13" :stroke-width="2.6" class="head-chevron" :class="{ open: favouritesOpen }" />
      </div>
      <template v-if="favouritesOpen">
        <p v-if="!favourites.length" class="hint">{{ t('Use + or “Add to Favourites” on a folder.', 'Usa + o «Añadir a Favoritos» en una carpeta.') }}</p>
        <div v-for="fav in favourites" :key="fav.id" class="item-wrap">
          <button type="button" class="item" :class="{ active: isActive(fav.location, fav.path) }" @click="navigate(fav.location, fav.path)">
            <component :is="FOLDER_ICONS_MAP[fav.icon_name] || (fav.path === homePath ? House : Folder)" :size="18" :stroke-width="1.9" class="item-icon" />
            <span class="item-label">{{ folderTitle(fav.path) }}</span>
          </button>
          <button type="button" class="remove" :title="t('Remove from Favourites', 'Quitar de Favoritos')" @click="remove(fav.id)"><X :size="12" :stroke-width="2.6" /></button>
        </div>
      </template>

      <div class="group-head static"><span>{{ t('Locations', 'Ubicaciones') }}</span></div>
      <button type="button" class="item" :class="{ active: isActive('external', homePath) }" @click="navigate('external', homePath)">
        <House :size="18" :stroke-width="1.9" class="item-icon" />
        <span class="item-label">{{ t('Home', 'Inicio') }}</span>
      </button>
      <button type="button" class="item" :class="{ active: isActive('external', 'shared') }" @click="navigate('external', 'shared')">
        <Users :size="18" :stroke-width="1.9" class="item-icon" />
        <span class="item-label">Compartido</span>
      </button>
      <a href="/photos/" class="item" :title="t('Open Photos', 'Abrir la Galería')">
        <Images :size="18" :stroke-width="1.9" class="item-icon" />
        <span class="item-label">{{ t('Photos', 'Galería') }}</span>
      </a>

      <div class="group-head static"><span>{{ t('Links', 'Enlaces') }}</span></div>
      <button type="button" class="item" :class="{ active: store.panel === 'shares' }" @click="store.panel = 'shares'">
        <Link :size="18" :stroke-width="1.9" class="item-icon" />
        <span class="item-label">{{ t('Shared links', 'Enlaces compartidos') }}</span>
      </button>
    </nav>

    <div v-if="summary.count" ref="transfersRef" class="transfers">
      <button type="button" class="item" :class="{ active: transfersOpen }" @click="transfersOpen = !transfersOpen">
        <svg class="ring" viewBox="0 0 20 20" aria-hidden="true">
          <circle cx="10" cy="10" r="8" class="ring-track" />
          <circle cx="10" cy="10" r="8" class="ring-fill" :style="{ strokeDashoffset: 50.3 * (1 - summary.progress) }" />
        </svg>
        <span class="item-label">{{ summary.label }}</span>
        <span class="count">{{ summary.count }}</span>
      </button>
      <TransferPanel :show="transfersOpen" />
    </div>

    <div v-if="store.disk" class="storage">
      <div class="meter"><i :style="{ width: `${usedPct}%` }" :class="{ full: usedPct > 90 }" /></div>
      <p>{{ t(`${formatSize(store.disk.free)} available of ${formatSize(store.disk.used + store.disk.free)}`, `${formatSize(store.disk.free)} disponibles de ${formatSize(store.disk.used + store.disk.free)}`) }}</p>
    </div>

    <button type="button" class="user" @click="userMenu">
      <span class="avatar" style="width: 30px; height: 30px; font-size: 12px">{{ initials }}</span>
      <span class="user-name">{{ store.user.display_name }}</span>
      <ChevronRight :size="14" :stroke-width="2.4" class="user-chevron" />
    </button>

    <BackupSheet v-if="showBackup" @close="showBackup = false" />
  </aside>
</template>

<style scoped>
.sidebar {
  width: 248px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  margin: 8px 0 8px 8px;
  padding: 10px 10px 8px;
  border-radius: 20px;
  background: color-mix(in srgb, var(--cell) 70%, transparent);
  backdrop-filter: blur(30px) saturate(180%);
  -webkit-backdrop-filter: blur(30px) saturate(180%);
  box-shadow: 0 0 0 0.5px var(--separator), 0 8px 30px rgba(0, 0, 0, 0.06);
}
@media (prefers-color-scheme: dark) {
  .sidebar { background: rgba(44, 44, 46, 0.55); box-shadow: 0 0 0 0.5px rgba(255, 255, 255, 0.08); }
}
.brand { display: flex; align-items: center; gap: 8px; padding: 6px 10px 10px; font-size: 14px; font-weight: 700; letter-spacing: -0.2px; }
.brand-icon { width: 24px; height: 24px; display: grid; place-items: center; border-radius: 7px; color: #fff; background: linear-gradient(180deg, #4aa8ff, #0a7cff); }
.nav { flex: 1; overflow-y: auto; }
.group-head {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 14px 10px 4px;
  font-size: 11px;
  font-weight: 700;
  color: var(--label-3);
  cursor: default;
  user-select: none;
}
.group-head span { flex: 1; }
.head-btn, .head-chevron { opacity: 0; transition: opacity 0.15s, transform 0.2s; color: var(--label-2); }
.head-btn { width: 18px; height: 18px; display: grid; place-items: center; border-radius: 5px; }
.head-btn:hover { background: var(--fill-3); }
.group-head:hover .head-btn, .group-head:hover .head-chevron { opacity: 1; }
.head-chevron.open { transform: rotate(90deg); }
.hint { padding: 2px 10px 6px; font-size: 12px; color: var(--label-3); }
.item-wrap { position: relative; }
.item {
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  height: 32px;
  padding: 0 10px;
  border-radius: 9px;
  font-size: 13.5px;
  letter-spacing: -0.1px;
  text-align: left;
  color: var(--label);
  cursor: default;
}
.item:hover { background: var(--fill-4); }
.item.active { background: var(--fill-2); font-weight: 600; }
.item-icon { flex-shrink: 0; color: var(--accent); }
.item-label { flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.remove { position: absolute; right: 6px; top: 7px; width: 18px; height: 18px; display: grid; place-items: center; border-radius: 50%; color: var(--label-2); opacity: 0; transition: opacity 0.15s; }
.item-wrap:hover .remove { opacity: 1; }
.remove:hover { color: #fff; background: var(--red); }
.transfers { position: relative; padding: 6px 0; border-top: 0.5px solid var(--separator); }
.ring { width: 18px; height: 18px; flex-shrink: 0; transform: rotate(-90deg); }
.ring-track { fill: none; stroke: var(--fill); stroke-width: 2.6; }
.ring-fill { fill: none; stroke: var(--accent); stroke-width: 2.6; stroke-linecap: round; stroke-dasharray: 50.3; transition: stroke-dashoffset 0.4s; }
.count { font-size: 12px; font-weight: 600; color: var(--label-2); }
.storage { padding: 10px 10px 8px; border-top: 0.5px solid var(--separator); }
.meter { height: 6px; border-radius: 3px; background: var(--fill-3); overflow: hidden; }
.meter i { display: block; height: 100%; border-radius: 3px; background: var(--accent); }
.meter i.full { background: var(--orange); }
.storage p { margin-top: 6px; font-size: 11.5px; color: var(--label-2); }
.user { display: flex; align-items: center; gap: 9px; padding: 6px 8px; border-radius: 12px; text-align: left; }
.user:hover { background: var(--fill-4); }
.user-name { flex: 1; min-width: 0; font-size: 13.5px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.user-chevron { color: var(--label-3); transform: rotate(-90deg); }
</style>
