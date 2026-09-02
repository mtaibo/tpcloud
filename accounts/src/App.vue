<script setup>
import { ref, onMounted, shallowRef } from 'vue'
import AccountView from './views/AccountView.vue'
import UsersView from './views/UsersView.vue'
import SessionsView from './views/SessionsView.vue'

const user = ref(null)
const loading = ref(true)
const currentView = shallowRef(AccountView)
const activeNav = ref('account')

onMounted(async () => {
  try {
    const res = await fetch('/auth/passkey/me')
    if (res.ok) {
      user.value = await res.json()
    } else {
      window.location.href = 'https://login.migueltaibo.com/?redirect=accounts.migueltaibo.com'
    }
  } catch {
    window.location.href = 'https://login.migueltaibo.com/?redirect=accounts.migueltaibo.com'
  } finally {
    loading.value = false
  }
})

async function logout() {
  await fetch('/auth/passkey/logout', { method: 'POST' })
  window.location.href = 'https://login.migueltaibo.com'
}

function navigate(view, name) {
  currentView.value = view
  activeNav.value = name
}
</script>

<template>
  <div class="page">
    <div v-if="loading" class="loading">
      <div class="spinner" />
    </div>

    <template v-else-if="user">
      <aside class="sidebar">
        <p class="brand">tpcloud</p>

        <nav class="nav">
          <p class="nav-section">Cuenta</p>
          <button
            class="nav-item"
            :class="{ active: activeNav === 'account' }"
            @click="navigate(AccountView, 'account')"
          >Mi cuenta</button>

          <template v-if="user.is_admin">
            <p class="nav-section">Administración</p>
            <button
              class="nav-item"
              :class="{ active: activeNav === 'users' }"
              @click="navigate(UsersView, 'users')"
            >Usuarios</button>
            <button
              class="nav-item"
              :class="{ active: activeNav === 'sessions' }"
              @click="navigate(SessionsView, 'sessions')"
            >Sesiones</button>
          </template>
        </nav>

        <div class="sidebar-footer">
          <p class="user-email">{{ user.email }}</p>
          <button class="logout" @click="logout">Cerrar sesión</button>
        </div>
      </aside>

      <main class="content">
        <component :is="currentView" :user="user" @user-updated="u => user = { ...user, ...u }" />
      </main>
    </template>
  </div>
</template>

<style>
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
html, body, #app { height: 100%; background: #000; color: #fff; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif; }
::-webkit-scrollbar { display: none; }
* { -ms-overflow-style: none; scrollbar-width: none; }
</style>

<style scoped>
.page {
  display: flex;
  min-height: 100dvh;
}

.loading {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.sidebar {
  width: 220px;
  min-height: 100dvh;
  border-right: 1px solid #1a1a1a;
  padding: 2rem 1rem;
  display: flex;
  flex-direction: column;
  gap: 2rem;
  flex-shrink: 0;
}

.brand {
  font-size: 0.75rem;
  font-weight: 500;
  color: #737373;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  padding: 0 0.5rem;
}

.nav {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  flex: 1;
}

.nav-section {
  font-size: 0.65rem;
  font-weight: 500;
  color: #525252;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  padding: 0.75rem 0.5rem 0.25rem;
}

.nav-item {
  background: none;
  border: none;
  color: #737373;
  font-size: 0.85rem;
  cursor: pointer;
  padding: 0.4rem 0.5rem;
  border-radius: 0.375rem;
  text-align: left;
  width: 100%;
  transition: color 0.2s, background 0.2s;
}

.nav-item:hover { color: #fff; background: #111; }
.nav-item.active { color: #fff; background: #1a1a1a; }

.sidebar-footer {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.user-email {
  font-size: 0.75rem;
  color: #525252;
  padding: 0 0.5rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.logout {
  background: none;
  border: none;
  color: #737373;
  font-size: 0.8rem;
  cursor: pointer;
  padding: 0.4rem 0.5rem;
  border-radius: 0.375rem;
  text-align: left;
  width: 100%;
  transition: color 0.2s;
}

.logout:hover { color: #fca5a5; }

.content {
  flex: 1;
  padding: 3rem 2.5rem;
  max-width: 700px;
}

.spinner {
  width: 24px;
  height: 24px;
  border: 2px solid #262626;
  border-top-color: #737373;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }
</style>
