<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import AlertHost from '@ui/AlertHost.vue'
import Icon from '@ui/Icon.vue'
import Spinner from '@ui/Spinner.vue'
import { api, signOut } from '@ui/api.js'
import { ask } from '@ui/dialog.js'
import { locale, syncLang, t } from '@ui/i18n.js'
import QuickAccess from './QuickAccess.vue'

const user = ref(null)
const profile = ref(null)
const activity = ref([])

// Only the modules this server runs (auth reports them in /me).
const allServices = () => [
  { module: 'store', name: t('Files', 'Archivos'), description: t('Your files and photos', 'Tus archivos y fotos'), icon: 'folder', tint: 'blue', url: '/store/' },
  { module: 'store', name: t('Photos', 'Galería'), description: t('The photos from your iPhone', 'Las fotos de tu iPhone'), icon: 'photo', tint: 'pink', url: '/photos/' },
  { module: 'store', name: t('Shared', 'Compartido'), description: t('Folders open with a link', 'Carpetas abiertas con enlace'), icon: 'link', tint: 'teal', url: '/share/' },
  { module: 'home', name: t('Home', 'Casa'), description: t('Lights, blinds and plugs', 'Luces, persianas y enchufes'), icon: 'house', tint: 'orange', url: '/home/' },
  { module: null, name: t('Account', 'Cuenta'), description: t('Security and devices', 'Seguridad y dispositivos'), icon: 'person', tint: 'gray', url: '/accounts' },
]
const services = computed(() => allServices().filter(s => !s.module || user.value?.modules?.includes(s.module)))

const greeting = computed(() => {
  const h = new Date().getHours()
  const part = h < 6 || h >= 21 ? t('Good evening', 'Buenas noches') : h < 13 ? t('Good morning', 'Buenos días') : t('Good afternoon', 'Buenas tardes')
  return `${part}, ${user.value.display_name.split(' ')[0]}`
})
const today = computed(() => {
  const text = new Intl.DateTimeFormat(locale(), { weekday: 'long', day: 'numeric', month: 'long' }).format(new Date())
  return text.charAt(0).toUpperCase() + text.slice(1)
})
// Failed sign-ins this week that the owner hasn't confirmed as their own (in Cuenta → Actividad).
const failures = computed(() => {
  const reviewed = profile.value?.failures_reviewed_at ? new Date(profile.value.failures_reviewed_at) : 0
  return activity.value.filter(e => e.kind === 'failed' && Date.now() - new Date(e.at) < 7 * 86400000 && new Date(e.at) > reviewed).length
})

// Quick access (blinds, server) is only for a desktop-sized window.
const wide = window.matchMedia('(min-width: 900px)')
const desktop = ref(wide.matches)
const onWide = e => { desktop.value = e.matches }
wide.addEventListener('change', onWide)
onBeforeUnmount(() => wide.removeEventListener('change', onWide))

// The dashboard doesn't scroll: it's one screen. Only a window too short to show all of it
// (a phone on its side, a small laptop window) gets scrolling back, so nothing is cut off.
const page = ref(null)
let fit = null
function lockScroll() {
  const last = page.value?.lastElementChild
  const fits = !last || last.getBoundingClientRect().bottom + window.scrollY + 16 <= window.innerHeight
  document.documentElement.classList.toggle('no-scroll', fits)
}
onMounted(() => {
  fit = new ResizeObserver(lockScroll)
  fit.observe(document.body)
  window.addEventListener('resize', lockScroll)
})
onBeforeUnmount(() => {
  fit?.disconnect()
  window.removeEventListener('resize', lockScroll)
  document.documentElement.classList.remove('no-scroll')
})

onMounted(async () => {
  window.history.replaceState(null, '', '/')
  user.value = await api('/auth/passkey/me')
  syncLang(user.value)
  const [p, a] = await Promise.allSettled([api('/auth/account/profile'), api('/auth/account/activity?limit=40')])
  if (p.status === 'fulfilled') profile.value = p.value
  if (a.status === 'fulfilled') activity.value = a.value
})

async function confirmSignOut() {
  const ok = await ask({
    title: t('Log out?', '¿Cerrar sesión?'),
    message: t('You’ll have to sign in again on this browser.', 'Tendrás que volver a iniciar sesión en este navegador.'),
    actions: [
      { label: t('Cancel', 'Cancelar'), role: 'cancel', value: false },
      { label: t('Log out', 'Cerrar sesión'), role: 'destructive', value: true },
    ],
  })
  if (ok) signOut()
}
</script>

<template>
  <div v-if="!user" class="splash"><Spinner :size="26" /></div>

  <template v-else>
    <main ref="page" class="page">
      <header class="page-header">
        <div class="top-line">
          <p class="subheadline secondary date">{{ today }}</p>
          <button class="btn-plain destructive sign-out" type="button" @click="confirmSignOut">{{ t('Log out', 'Cerrar sesión') }}</button>
        </div>
        <h1 class="large-title">{{ greeting }}</h1>
      </header>

      <div class="stack">
        <div class="grid">
          <a v-for="s in services" :key="s.name" :href="s.url" class="tile">
            <span class="tile-icon" :style="{ background: `var(--${s.tint})` }"><Icon :name="s.icon" :size="28" :weight="1.9" /></span>
            <span class="tile-text">
              <span class="headline">{{ s.name }}</span>
              <span class="footnote secondary">{{ s.description }}</span>
            </span>
            <Icon name="chevron-right" :size="17" :weight="2.4" class="row-chevron tile-chevron" />
          </a>
        </div>

        <QuickAccess v-if="desktop" :user="user" />
      </div>

      <a v-if="failures" href="/accounts/activity" class="banner">
        <Icon name="warning" :size="22" />
        <span class="subheadline">
          {{ failures === 1
            ? t('1 failed attempt to get into your account this week. Check it.', '1 intento fallido de entrar en tu cuenta esta semana. Revísalo.')
            : t(`${failures} failed attempts to get into your account this week. Check them.`, `${failures} intentos fallidos de entrar en tu cuenta esta semana. Revísalo.`) }}
        </span>
        <Icon name="chevron-right" :size="17" :weight="2.4" class="row-chevron" />
      </a>
    </main>

    <AlertHost />
  </template>
</template>

<style>
/* The dashboard is one screen: no bounce, no bar, and no scrolling while it all fits. */
html { overscroll-behavior: none; scrollbar-width: none; }
html::-webkit-scrollbar { display: none; }
html.no-scroll, html.no-scroll body { overflow: hidden; height: 100%; }
</style>

<style scoped>
.splash { min-height: 100dvh; display: grid; place-items: center; }
.date { font-weight: 600; text-transform: none; }
.page { padding-top: calc(24px + env(safe-area-inset-top)); }
.top-line { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; }
.page-header .large-title { margin-top: 2px; }
/* Plain red text over the large title, where iOS puts a bar's text buttons. */
.sign-out { min-height: 0; padding: 0; font-size: 17px; }

.stack { display: flex; flex-direction: column; gap: 14px; margin-bottom: 28px; }
.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}
.tile {
  display: flex;
  gap: 14px;
  border-radius: var(--radius-card);
  background: var(--cell);
  transition: transform 0.3s var(--spring);
}
.tile:active { transform: scale(0.97); }
@media (hover: hover) { .tile:hover { transform: translateY(-2px); } }
.tile-chevron { display: none; }

/* Phone: two by two, tall tiles; an odd one out takes the whole last row sideways. */
@media (max-width: 600px) {
  .tile { min-height: 150px; flex-direction: column; justify-content: space-between; gap: 16px; padding: 18px; }
  .tile:last-child:nth-child(odd) { grid-column: span 2; min-height: 0; flex-direction: row; align-items: center; justify-content: flex-start; }
}

/* Wider: sideways tiles, the first one across the top, and an odd one out across the bottom. */
@media (min-width: 601px) {
  .tile { align-items: center; padding: 12px 20px 12px 12px; }
  .tile:first-child { grid-column: span 2; padding: 16px 24px 16px 16px; }
  .tile:first-child .tile-icon { width: 60px; height: 60px; border-radius: 17px; }
  .tile:last-child:nth-child(even) { grid-column: span 2; }
  .tile-text { flex: 1; min-width: 0; }
  .tile-chevron { display: block; }
}

.tile-icon { width: 54px; height: 54px; display: grid; place-items: center; border-radius: 15px; color: #fff; }
.tile-text { display: flex; flex-direction: column; gap: 2px; }

.banner {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 20px;
  margin-bottom: 26px;
  border-radius: 20px;
  background: color-mix(in srgb, var(--orange) 16%, var(--cell));
}
.banner > .icon:first-child { flex-shrink: 0; color: var(--orange); }
.banner span { flex: 1; }
</style>
