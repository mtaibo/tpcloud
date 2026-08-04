<script setup>
import { ref, onMounted } from 'vue'

const user = ref(null)
const loading = ref(true)

const services = [
  { name: 'Login', description: 'Auth · Passkeys', url: 'https://login.migueltaibo.com' },
  { name: 'TPHome', description: 'Home automation', url: 'https://tphome.migueltaibo.com' },
  { name: 'Portfolio', description: 'migueltaibo.com', url: 'https://migueltaibo.com' },
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
    <div v-if="!loading && user" class="container">
      <header class="header">
        <p class="label">tpcloud</p>
        <button class="logout" @click="logout">Cerrar sesión</button>
      </header>

      <div class="hero">
        <h1>Hola, {{ user.display_name }}.</h1>
        <p class="subtitle">{{ user.email }}</p>
      </div>

      <div class="section">
        <p class="section-label">Servicios</p>
        <div class="grid">
          <a
            v-for="s in services"
            :key="s.name"
            :href="s.url"
            class="card"
          >
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

html, body, #app {
  height: 100%;
  background: #000;
  color: #fff;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
}

::-webkit-scrollbar { display: none; }
* { -ms-overflow-style: none; scrollbar-width: none; }
</style>

<style scoped>
.page {
  min-height: 100dvh;
  display: flex;
  justify-content: center;
  padding: 3rem 1.5rem;
}

.container {
  width: 100%;
  max-width: 640px;
  display: flex;
  flex-direction: column;
  gap: 3rem;
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.label {
  font-size: 0.75rem;
  font-weight: 500;
  color: #737373;
  text-transform: uppercase;
  letter-spacing: 0.1em;
}

.logout {
  background: none;
  border: none;
  color: #737373;
  font-size: 0.8rem;
  cursor: pointer;
  transition: color 0.2s;
  padding: 0;
}

.logout:hover { color: #fff; }

.hero {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

h1 {
  font-size: 2rem;
  font-weight: 600;
  color: #fff;
  letter-spacing: -0.02em;
}

.subtitle {
  font-size: 0.95rem;
  color: #737373;
}

.section {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.section-label {
  font-size: 0.75rem;
  font-weight: 500;
  color: #737373;
  text-transform: uppercase;
  letter-spacing: 0.1em;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 1rem;
}

.card {
  border: 1px solid #262626;
  border-radius: 0.5rem;
  padding: 1rem;
  text-decoration: none;
  transition: border-color 0.3s;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.card:hover { border-color: #525252; }

.card-name {
  font-size: 0.95rem;
  font-weight: 500;
  color: #fff;
}

.card-desc {
  font-size: 0.8rem;
  color: #737373;
}
</style>
