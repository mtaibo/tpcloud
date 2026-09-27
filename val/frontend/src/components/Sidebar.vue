<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { LayoutDashboard, TrendingUp, ShoppingBag, Layers, Trophy, User, LogOut, Cloud } from 'lucide-vue-next'

const LOGIN_URL = 'https://login.migueltaibo.com'

const props = defineProps({
  user: Object,
  currentView: String,
  linkedAccount: Object,
})
const emit = defineEmits(['navigate'])

const menuOpen = ref(false)
const cardRef = ref(null)

const navItems = [
  { id: 'dashboard', label: 'Overview', icon: LayoutDashboard },
  { id: 'stats', label: 'Stats', icon: TrendingUp },
  { id: 'shop', label: 'Daily Shop', icon: ShoppingBag },
  { id: 'inventory', label: 'Inventory', icon: Layers },
  { id: 'esports', label: 'Esports', icon: Trophy },
]

function onClickOutside(e) {
  if (cardRef.value && !cardRef.value.contains(e.target)) menuOpen.value = false
}

async function logout() {
  await fetch('/auth/passkey/logout', { method: 'POST' })
  window.location.href = `${LOGIN_URL}/`
}

onMounted(() => document.addEventListener('click', onClickOutside))
onUnmounted(() => document.removeEventListener('click', onClickOutside))
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-brand">
      <span class="brand-text">TPVal</span>
    </div>

    <nav class="sidebar-nav">
      <div class="section-label-row">
        <p class="section-label">Navigation</p>
      </div>

      <button
        v-for="item in navItems"
        :key="item.id"
        :class="['nav-item', { active: currentView === item.id }]"
        @click="emit('navigate', item.id)"
      >
        <component :is="item.icon" class="nav-icon" />
        <span>{{ item.label }}</span>
      </button>

      <div v-if="linkedAccount" class="section-label-row section-mt">
        <p class="section-label">Account</p>
      </div>

      <div v-if="linkedAccount" class="account-badge">
        <span class="account-name">{{ linkedAccount.riot_name }}<span class="account-tag">#{{ linkedAccount.riot_tag }}</span></span>
        <span class="account-region">{{ linkedAccount.region.toUpperCase() }}</span>
      </div>
    </nav>

    <footer class="sidebar-footer">
      <div ref="cardRef" class="user-card">
        <Transition name="fade">
          <div v-if="menuOpen" class="user-menu">
            <a href="https://accounts.migueltaibo.com" class="user-menu-item">
              <User class="menu-icon" />
              Account
            </a>
            <a href="https://cloud.migueltaibo.com" class="user-menu-item">
              <Cloud class="menu-icon" />
              TPCloud
            </a>
            <button class="user-menu-item logout-item" @click="logout">
              <LogOut class="menu-icon" />
              Log out
            </button>
          </div>
        </Transition>

        <button class="user-btn" @click="menuOpen = !menuOpen">
          <div class="user-avatar">
            <User class="avatar-icon" />
          </div>
          <span class="user-display-name">{{ user?.display_name ?? '…' }}</span>
        </button>
      </div>
    </footer>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 240px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: #111113;
  box-shadow: 1px 0 0 rgba(255, 255, 255, 0.06);
}

.sidebar-brand {
  height: 56px;
  display: flex;
  align-items: center;
  padding: 0 1.25rem;
  flex-shrink: 0;
}

.brand-text {
  font-size: 1.125rem;
  font-weight: 700;
  color: #FF4655;
  letter-spacing: -0.02em;
}

.sidebar-nav {
  flex: 1;
  padding: 0.75rem;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.section-label-row {
  padding: 0.25rem 0.5rem;
}

.section-mt { margin-top: 1rem; }

.section-label {
  font-size: 0.7rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  color: #636366;
  user-select: none;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 1rem;
  width: 100%;
  padding: 0.35rem 1rem;
  border-radius: 8px;
  font-size: 0.875rem;
  font-weight: 500;
  text-align: left;
  color: #fff;
  background: transparent;
  border: none;
  cursor: default;
  transition: background 0.15s;
}

.nav-item:hover:not(.active) { background: rgba(255, 255, 255, 0.04); }
.nav-item.active { background: rgba(255, 70, 85, 0.12); font-weight: 600; }
.nav-item.active .nav-icon { color: #FF4655; }

.nav-icon { width: 20px; height: 20px; flex-shrink: 0; color: #8E8E93; transition: color 0.15s; }

.account-badge {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.4rem 1rem;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.04);
  border: 0.5px solid rgba(255, 255, 255, 0.06);
}

.account-name {
  font-size: 0.8rem;
  font-weight: 500;
  color: #d1d1d6;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.account-tag { color: #636366; }

.account-region {
  font-size: 0.65rem;
  font-weight: 600;
  color: #636366;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 4px;
  padding: 1px 5px;
  flex-shrink: 0;
  margin-left: 6px;
}

.sidebar-footer { padding: 0.5rem 1rem 1.5rem; flex-shrink: 0; }
.user-card { position: relative; }

.user-menu {
  position: absolute;
  bottom: 100%;
  left: 0;
  right: 0;
  margin-bottom: 0.5rem;
  background: #1c1c1e;
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.user-menu-item {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  width: 100%;
  padding: 0.625rem 0.75rem;
  font-size: 0.875rem;
  color: rgba(255, 255, 255, 0.7);
  background: none;
  border: none;
  cursor: default;
  transition: background 0.15s, color 0.15s;
  text-align: left;
  text-decoration: none;
}

.user-menu-item:hover { background: rgba(255, 255, 255, 0.06); color: #fff; }
.logout-item:hover { color: #f87171; }
.menu-icon { width: 16px; height: 16px; flex-shrink: 0; }

.user-btn {
  display: flex;
  align-items: center;
  width: 100%;
  padding: 0.625rem 1rem;
  border-radius: 8px;
  background: none;
  border: none;
  cursor: default;
  transition: background 0.15s;
}

.user-btn:hover { background: rgba(255, 255, 255, 0.04); }

.user-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: #1c1c1e;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.avatar-icon { width: 15px; height: 15px; color: #8E8E93; }

.user-display-name {
  margin-left: 1rem;
  font-size: 0.875rem;
  font-weight: 500;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-align: left;
}

@media (max-width: 767px) { .sidebar { display: none; } }

.fade-enter-active, .fade-leave-active { transition: opacity 0.15s, transform 0.15s; }
.fade-enter-from, .fade-leave-to { opacity: 0; transform: translateY(4px); }
</style>
