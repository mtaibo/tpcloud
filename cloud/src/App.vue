<script setup>
import { ref, onMounted } from 'vue'

const user = ref(null)
const loading = ref(true)

const services = [
  { name: 'TPHome',    icon: 'home',      description: 'Domótica',  url: 'https://tphome.migueltaibo.com' },
  { name: 'Accounts', icon: 'person',    description: 'Cuentas',   url: 'https://accounts.migueltaibo.com' },
  { name: 'Store',    icon: 'database',  description: 'Tienda',    url: 'https://store.migueltaibo.com' },
  { name: 'Portfolio',icon: 'portfolio', description: 'Portfolio',  url: 'https://migueltaibo.com' },
]

onMounted(async () => {
  try {
    const res = await fetch('/auth/passkey/me')
    if (res.ok) {
      user.value = await res.json()
    } else {
      window.location.href = 'https://login.migueltaibo.com/?redirect=cloud.migueltaibo.com'
    }
  } catch {
    window.location.href = 'https://login.migueltaibo.com/?redirect=cloud.migueltaibo.com'
  } finally {
    loading.value = false
  }
})

async function logout() {
  await fetch('/auth/passkey/logout', { method: 'POST' })
  window.location.href = 'https://login.migueltaibo.com'
}
</script>

<template>
  <div class="page">

    <!-- Logo: top-left corner, desktop only -->
    <div class="logo-corner">
      <svg width="28" height="28" viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
        <rect x="7" y="8.5" width="18" height="4" rx="2" fill="#fff"/>
        <circle cx="16" cy="18.5" r="3" fill="#fff"/>
        <circle cx="16" cy="25" r="2.2" fill="#fff"/>
      </svg>
    </div>

    <div v-if="!loading && user" class="container">

      <div class="hero">
        <h1>Hello, {{ user.display_name }}.</h1>
        <p class="muted">{{ user.email }}</p>
      </div>

      <div class="section">
        <div class="section-top">
          <button class="logout" @click="logout">Log out</button>
        </div>
        <div class="grid">
          <a v-for="s in services" :key="s.name" :href="s.url" class="card">
            <div class="card-icon">
              <!-- home -->
              <svg v-if="s.icon === 'home'" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                <path d="M3 10.5L12 3l9 7.5V20a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V10.5z"/>
                <path d="M9 21V13h6v8"/>
              </svg>
              <!-- person -->
              <svg v-if="s.icon === 'person'" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="8" r="4"/>
                <path d="M4 20c0-4 3.6-7 8-7s8 3 8 7"/>
              </svg>
              <!-- database -->
              <svg v-if="s.icon === 'database'" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                <ellipse cx="12" cy="5" rx="9" ry="3"/>
                <path d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5"/>
                <path d="M3 12c0 1.66 4.03 3 9 3s9-1.34 9-3"/>
              </svg>
              <!-- portfolio -->
              <svg v-if="s.icon === 'portfolio'" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                <rect x="2" y="7" width="20" height="14" rx="2"/>
                <path d="M16 7V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v2"/>
              </svg>
            </div>
            <p class="card-name">{{ s.name }}</p>
            <p class="card-desc">{{ s.description }}</p>
          </a>
        </div>
      </div>

    </div>
  </div>
</template>

<style>
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
html, body, #app { height: 100%; background: #000; color: #fff; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif; }
::-webkit-scrollbar { display: none; }
* { -ms-overflow-style: none; scrollbar-width: none; }
</style>

<style scoped>
.page { min-height: 100dvh; display: flex; justify-content: center; padding: 3rem 1.5rem; }
.container { width: 100%; max-width: 640px; display: flex; flex-direction: column; gap: 2.5rem; }

/* Logo corner */
.logo-corner {
  display: none;
  position: fixed;
  top: 1.5rem;
  left: 1.5rem;
  opacity: 0.85;
  transition: opacity 0.2s;
}
.logo-corner:hover { opacity: 1; }
@media (min-width: 768px) {
  .logo-corner { display: block; }
}

/* Logout */
.logout {
  background: none;
  border: none;
  color: #525252;
  font-size: 0.8rem;
  cursor: pointer;
  transition: color 0.2s;
}
.logout:hover { color: #fff; }

/* Hero */
.hero { display: flex; flex-direction: column; gap: 0.4rem; }
h1 { font-size: 2rem; font-weight: 600; letter-spacing: -0.02em; }
.muted { color: #737373; font-size: 0.875rem; }

/* Sections */
.section { display: flex; flex-direction: column; gap: 0.75rem; }
.section-top { display: flex; justify-content: flex-end; }

/* Service cards */
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 0.75rem; }
.card {
  border: 0.5px solid rgba(255, 255, 255, 0.14);
  border-radius: 18px;
  padding: 1rem;
  text-decoration: none;
  transition: border-color 0.2s, box-shadow 0.2s;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  background: rgba(255, 255, 255, 0.065);
  backdrop-filter: blur(24px) saturate(160%);
  -webkit-backdrop-filter: blur(24px) saturate(160%);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.35),
    inset 0 1px 0 rgba(255, 255, 255, 0.12),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15);
}
.card:hover {
  border-color: rgba(255, 255, 255, 0.25);
  box-shadow:
    0 12px 40px rgba(0, 0, 0, 0.45),
    inset 0 1px 0 rgba(255, 255, 255, 0.16),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15);
}
.card-icon { color: rgba(255, 255, 255, 0.55); margin-bottom: 0.35rem; }
.card-name { font-size: 0.95rem; font-weight: 500; color: #fff; }
.card-desc { font-size: 0.8rem; color: #737373; }
</style>
