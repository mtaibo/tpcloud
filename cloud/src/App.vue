<script setup>
import { computed, onMounted, ref } from 'vue'
import AlertHost from '@ui/AlertHost.vue'
import Icon from '@ui/Icon.vue'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import NavBar from '@ui/NavBar.vue'
import Spinner from '@ui/Spinner.vue'
import { api, signOut } from '@ui/api.js'
import { ask } from '@ui/dialog.js'

const user = ref(null)
const profile = ref(null)
const activity = ref([])

// Only the modules this server runs (auth reports them in /me).
const allServices = [
  { module: 'store', name: 'Archivos', description: 'Tus archivos y fotos', icon: 'folder', tint: 'blue', url: '/store/' },
  { module: 'home', name: 'Casa', description: 'Luces, persianas y enchufes', icon: 'house', tint: 'orange', url: '/home/' },
  { module: null, name: 'Cuenta', description: 'Seguridad y dispositivos', icon: 'person', tint: 'gray', url: '/accounts' },
]
const services = computed(() => allServices.filter(s => !s.module || user.value?.modules?.includes(s.module)))

const greeting = computed(() => {
  const h = new Date().getHours()
  const part = h < 6 ? 'Buenas noches' : h < 13 ? 'Buenos días' : h < 21 ? 'Buenas tardes' : 'Buenas noches'
  return `${part}, ${user.value.display_name.split(' ')[0]}`
})
const today = computed(() => {
  const text = new Intl.DateTimeFormat('es-ES', { weekday: 'long', day: 'numeric', month: 'long' }).format(new Date())
  return text.charAt(0).toUpperCase() + text.slice(1)
})
const others = computed(() => (profile.value?.sessions.length ?? 1) - 1)
const failures = computed(() => activity.value.filter(e => e.kind === 'failed' && Date.now() - new Date(e.at) < 7 * 86400000).length)

onMounted(async () => {
  window.history.replaceState(null, '', '/')
  user.value = await api('/auth/passkey/me')
  const [p, a] = await Promise.allSettled([api('/auth/account/profile'), api('/auth/account/activity?limit=40')])
  if (p.status === 'fulfilled') profile.value = p.value
  if (a.status === 'fulfilled') activity.value = a.value
})

async function confirmSignOut() {
  const ok = await ask({
    title: '¿Cerrar sesión?',
    message: 'Tendrás que volver a iniciar sesión en este navegador.',
    actions: [
      { label: 'Cancelar', role: 'cancel', value: false },
      { label: 'Cerrar sesión', role: 'destructive', value: true },
    ],
  })
  if (ok) signOut()
}
</script>

<template>
  <div v-if="!user" class="splash"><Spinner :size="26" /></div>

  <template v-else>
    <NavBar :title="greeting" />

    <main class="page">
      <header class="page-header">
        <p class="subheadline secondary date">{{ today }}</p>
        <h1 class="large-title">{{ greeting }}</h1>
      </header>

      <div class="grid">
        <a v-for="s in services" :key="s.name" :href="s.url" class="tile">
          <span class="tile-icon" :style="{ background: `var(--${s.tint})` }"><Icon :name="s.icon" :size="28" :weight="1.9" /></span>
          <span class="tile-text">
            <span class="headline">{{ s.name }}</span>
            <span class="footnote secondary">{{ s.description }}</span>
          </span>
        </a>
      </div>

      <a v-if="failures" href="/accounts/activity" class="banner">
        <Icon name="warning" :size="22" />
        <span class="subheadline">
          {{ failures === 1 ? '1 intento fallido' : `${failures} intentos fallidos` }} de entrar en tu cuenta esta semana. Revísalo.
        </span>
        <Icon name="chevron-right" :size="17" :weight="2.4" class="row-chevron" />
      </a>

      <ListSection v-if="profile" header="Tu cuenta">
        <ListRow
          href="/accounts/sessions"
          icon="laptop"
          tint="green"
          title="Dispositivos"
          :subtitle="others > 0 ? `Sesión iniciada aquí y en ${others} ${others === 1 ? 'sitio más' : 'sitios más'}` : 'Solo en este dispositivo'"
        />
        <ListRow
          href="/accounts/security"
          icon="shield"
          tint="blue"
          title="Seguridad"
          :subtitle="profile.passkeys.length ? `${profile.passkeys.length} passkey${profile.passkeys.length === 1 ? '' : 's'}${profile.totp_enabled ? ' · 2FA activado' : ''}` : 'Añade una passkey para entrar sin contraseña'"
        />
      </ListSection>

      <ListSection>
        <ListRow title="Cerrar sesión" clickable destructive centered @click="confirmSignOut" />
      </ListSection>
    </main>

    <AlertHost />
  </template>
</template>

<style>
/* The dashboard is one screen: no bounce, and if a short window ever needs to scroll, no bar. */
html { overscroll-behavior: none; scrollbar-width: none; }
html::-webkit-scrollbar { display: none; }
</style>

<style scoped>
.splash { min-height: 100dvh; display: grid; place-items: center; }
.date { font-weight: 600; text-transform: none; }
.page-header .large-title { margin-top: 2px; }

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(min(100%, 200px), 1fr));
  gap: 14px;
  margin-bottom: 28px;
}
@media (max-width: 480px) { .grid { grid-template-columns: 1fr 1fr; } }
.tile {
  min-height: 150px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 16px;
  padding: 18px;
  border-radius: var(--radius-card);
  background: var(--cell);
  transition: transform 0.3s var(--spring);
}
.tile:active { transform: scale(0.97); }
@media (hover: hover) { .tile:hover { transform: translateY(-2px); } }
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
