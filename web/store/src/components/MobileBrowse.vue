<script setup>
import { computed, onMounted } from 'vue'
import { Cloud, DatabaseBackup, Folder, House, Link, LogOut, User, Users } from 'lucide-vue-next'
import ListSection from '../ui/ListSection.vue'
import { useFavourites } from '../useFavourites.js'
import { FOLDER_ICONS_MAP } from '../folderIcons.js'
import { folderTitle, homePath, navigate, store } from '../store.js'
import { formatSize } from '../utils/formatSize.js'
import { t } from '../ui/i18n.js'

// The phone's "{{ t('Browse', 'Explorar') }}" tab, like Files' Browse: places, favourites, links and storage.
const emit = defineEmits(['backup'])
const { favourites, load } = useFavourites()
onMounted(load)

const usedPct = computed(() => (store.disk ? Math.round((store.disk.used / (store.disk.used + store.disk.free)) * 100) : 0))

async function logout() {
  await fetch('/auth/passkey/logout', { method: 'POST' })
  window.location.href = '/login'
}
</script>

<template>
  <section class="browse">
    <h1 class="large-title">{{ t('Browse', 'Explorar') }}</h1>

    <ListSection header="Ubicaciones">
      <button type="button" class="row" @click="navigate('external', homePath)">
        <House class="lead" :size="24" /><span class="row-body"><span class="row-title">{{ t('Home', 'Inicio') }}</span></span>
      </button>
      <button type="button" class="row" @click="navigate('external', 'shared')">
        <Users class="lead" :size="24" /><span class="row-body"><span class="row-title">Compartido</span></span>
      </button>
      <button type="button" class="row" @click="store.panel = 'shares'">
        <Link class="lead" :size="24" /><span class="row-body"><span class="row-title">{{ t('Shared links', 'Enlaces compartidos') }}</span></span>
      </button>
    </ListSection>

    <ListSection :header="t('Favourites', 'Favoritos')" :footer="favourites.length ? '' : t('Touch and hold a folder and choose “Add to Favourites”.', 'Mantén pulsada una carpeta y elige «Añadir a Favoritos».')">
      <button v-for="f in favourites" :key="f.id" type="button" class="row" @click="navigate(f.location, f.path)">
        <component :is="FOLDER_ICONS_MAP[f.icon_name] || (f.path === homePath ? House : Folder)" class="lead" :size="24" />
        <span class="row-body"><span class="row-title">{{ folderTitle(f.path) }}</span></span>
      </button>
      <div v-if="!favourites.length" class="row"><span class="row-body"><span class="row-title secondary">{{ t('None yet', 'Ninguno todavía') }}</span></span></div>
    </ListSection>

    <ListSection v-if="store.disk" :header="t('Storage', 'Almacenamiento')">
      <div class="row storage">
        <span class="row-body">
          <span class="row-title">{{ t(`${formatSize(store.disk.free)} available`, `${formatSize(store.disk.free)} disponibles`) }}</span>
          <span class="meter"><i :style="{ width: `${usedPct}%` }" /></span>
          <span class="row-subtitle small">{{ t(`${formatSize(store.disk.used)} used of ${formatSize(store.disk.used + store.disk.free)}`, `${formatSize(store.disk.used)} usados de ${formatSize(store.disk.used + store.disk.free)}`) }}</span>
        </span>
      </div>
    </ListSection>

    <ListSection>
      <a href="/accounts" class="row"><User class="lead" :size="24" /><span class="row-body"><span class="row-title">{{ t('Your account', 'Tu cuenta') }}</span></span></a>
      <a href="/" class="row"><Cloud class="lead" :size="24" /><span class="row-body"><span class="row-title">{{ t('Back to TPCloud', 'Volver a TPCloud') }}</span></span></a>
      <button v-if="store.user.is_admin" type="button" class="row" @click="emit('backup')"><DatabaseBackup class="lead" :size="24" /><span class="row-body"><span class="row-title">{{ t('Backup', 'Copia de seguridad') }}</span></span></button>
      <button type="button" class="row destructive" @click="logout"><LogOut class="lead" :size="24" /><span class="row-body"><span class="row-title">{{ t('Log out', 'Cerrar sesión') }}</span></span></button>
    </ListSection>
  </section>
</template>

<style scoped>
.browse { flex: 1; overflow-y: auto; padding: calc(24px + env(safe-area-inset-top)) 16px calc(110px + env(safe-area-inset-bottom)); }
.large-title { padding: 0 4px 18px; }
.lead { flex-shrink: 0; color: var(--accent); }
.row { --inset: 58px; }
.row.destructive .lead { color: var(--red); }
.storage { --inset: 20px; }
.meter { display: block; height: 8px; margin: 8px 0 4px; border-radius: 4px; background: var(--fill-3); overflow: hidden; }
.meter i { display: block; height: 100%; background: var(--accent); border-radius: 4px; }
</style>
