<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { LayoutDashboard, TrendingUp, ShoppingBag, Layers, Trophy, User, LogOut, Cloud, Search, Link, Wallet, Moon, Heart, History, Sword, Award, LineChart } from 'lucide-vue-next'
import SessionStatus from './SessionStatus.vue'

const LOGIN_URL = 'https://login.migueltaibo.com'

const props = defineProps({
  user: Object,
  currentView: String,
  linkedAccount: Object,
  activePlayer: Object,
  isOwnAccount: Boolean,
  searchLoading: Boolean,
  searchError: String,
})
const emit = defineEmits(['navigate', 'search', 'open-account-setup', 'go-home', 'open-extension-setup'])

const menuOpen = ref(false)
const cardRef = ref(null)
const searchInput = ref('')

const allNavItems = [
  { id: 'dashboard', label: 'Overview', icon: LayoutDashboard, requiresOwn: false },
  { id: 'stats', label: 'Stats', icon: TrendingUp, requiresOwn: false },
  { id: 'rank-chart', label: 'Rank Chart', icon: LineChart, requiresOwn: true },
  { id: 'shop', label: 'Daily Shop', icon: ShoppingBag, requiresOwn: true },
  { id: 'night-market', label: 'Night Market', icon: Moon, requiresOwn: true },
  { id: 'wallet', label: 'Wallet', icon: Wallet, requiresOwn: true },
  { id: 'inventory', label: 'Inventory', icon: Layers, requiresOwn: true },
  { id: 'loadout', label: 'Loadout Builder', icon: Sword, requiresOwn: true },
  { id: 'wishlist', label: 'Wishlist', icon: Heart, requiresOwn: true },
  { id: 'shop-history', label: 'Shop History', icon: History, requiresOwn: true },
  { id: 'battle-pass', label: 'Battle Pass', icon: Award, requiresOwn: true },
  { id: 'esports', label: 'Esports', icon: Trophy, requiresOwn: false },
]

const navItems = computed(() =>
  allNavItems.filter(item => !item.requiresOwn || props.isOwnAccount)
)

function doSearch() {
  const raw = searchInput.value.trim()
  if (!raw) return
  const parts = raw.split('#')
  if (parts.length !== 2 || !parts[0] || !parts[1]) return
  emit('search', parts[0].trim(), parts[1].trim())
  searchInput.value = ''
}

function goHome() {
  emit('go-home')
  searchInput.value = ''
}

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
    <div class="sidebar-brand" @click="goHome" style="cursor:default">
      <span class="brand-text">TPVal</span>
    </div>

    <!-- Search box -->
    <div class="search-wrap">
      <div class="search-box" :class="{ loading: searchLoading }">
        <Search class="search-icon" />
        <input
          v-model="searchInput"
          class="search-input"
          placeholder="name#TAG"
          autocomplete="off"
          spellcheck="false"
          @keydown.enter="doSearch"
        />
      </div>
      <p v-if="searchError" class="search-error">{{ searchError }}</p>
    </div>

    <nav class="sidebar-nav">
      <!-- Nav items (filtered by isOwnAccount for shop/inventory) -->
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

      <!-- Active player indicator -->
      <div v-if="activePlayer" class="section-label-row section-mt">
        <p class="section-label">Viewing</p>
      </div>

      <div v-if="activePlayer" class="account-badge" :class="{ own: isOwnAccount }">
        <span class="account-name">{{ activePlayer.riot_name }}<span class="account-tag">#{{ activePlayer.riot_tag }}</span></span>
        <span class="account-region">{{ (activePlayer.region || '').toUpperCase() }}</span>
      </div>

      <!-- Own account section -->
      <div class="section-label-row section-mt">
        <p class="section-label">My Account</p>
      </div>

      <div v-if="linkedAccount" class="own-account-row">
        <button
          class="own-account-btn"
          :class="{ active: isOwnAccount }"
          @click="goHome"
        >
          <span class="own-name">{{ linkedAccount.riot_name }}<span class="account-tag">#{{ linkedAccount.riot_tag }}</span></span>
        </button>
        <button class="manage-btn" @click="emit('open-account-setup')" title="Manage account">
          <Link class="manage-icon" />
        </button>
      </div>

      <button v-else class="nav-item link-btn" @click="emit('open-account-setup')">
        <Link class="nav-icon" />
        <span>Link Riot Account</span>
      </button>

      <div v-if="linkedAccount" class="session-wrap">
        <SessionStatus @reconnect="emit('open-extension-setup')" />
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

/* Search */
.search-wrap {
  padding: 0 0.75rem 0.5rem;
  flex-shrink: 0;
}

.search-box {
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(255, 255, 255, 0.06);
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  padding: 0 10px;
  transition: border-color 0.15s;
}

.search-box:focus-within { border-color: rgba(255, 70, 85, 0.5); }
.search-box.loading { opacity: 0.6; }

.search-icon { width: 14px; height: 14px; color: #636366; flex-shrink: 0; }

.search-input {
  flex: 1;
  background: none;
  border: none;
  outline: none;
  font-size: 0.8rem;
  color: #fff;
  padding: 7px 0;
  min-width: 0;
}

.search-input::placeholder { color: #636366; }

.search-error {
  font-size: 0.7rem;
  color: #ff453a;
  margin-top: 4px;
  padding: 0 2px;
}

/* Nav */
.sidebar-nav {
  flex: 1;
  padding: 0.25rem 0.75rem 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.section-label-row { padding: 0.25rem 0.5rem; }
.section-mt { margin-top: 0.75rem; }

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

/* Viewing badge */
.account-badge {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.4rem 1rem;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.04);
  border: 0.5px solid rgba(255, 255, 255, 0.06);
}

.account-badge.own {
  background: rgba(255, 70, 85, 0.06);
  border-color: rgba(255, 70, 85, 0.2);
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

/* Own account row */
.own-account-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 0 0.25rem;
}

.own-account-btn {
  flex: 1;
  background: none;
  border: none;
  padding: 0.3rem 0.75rem;
  border-radius: 8px;
  text-align: left;
  cursor: default;
  transition: background 0.15s;
  min-width: 0;
}

.own-account-btn:hover { background: rgba(255, 255, 255, 0.04); }
.own-account-btn.active { background: rgba(255, 70, 85, 0.08); }

.own-name {
  font-size: 0.8rem;
  font-weight: 500;
  color: #8e8e93;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  display: block;
}

.own-account-btn.active .own-name { color: #FF4655; }

.manage-btn {
  background: none;
  border: none;
  padding: 6px;
  border-radius: 6px;
  cursor: default;
  color: #636366;
  transition: color 0.15s, background 0.15s;
  flex-shrink: 0;
}

.manage-btn:hover { color: #fff; background: rgba(255, 255, 255, 0.06); }
.manage-icon { width: 14px; height: 14px; }

.link-btn { color: #636366; }
.link-btn:hover { color: #fff; }

.session-wrap { padding: 8px 4px 0; }

/* Footer */
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
