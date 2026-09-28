<script setup>
import { ref, computed, onMounted } from 'vue'
import Sidebar from './components/Sidebar.vue'
import AccountSetupModal from './components/AccountSetupModal.vue'
import ExtensionSetupModal from './components/ExtensionSetupModal.vue'
import DashboardView from './views/DashboardView.vue'
import StatsView from './views/StatsView.vue'
import ShopView from './views/ShopView.vue'
import InventoryView from './views/InventoryView.vue'
import EsportsView from './views/EsportsView.vue'
import WalletView from './views/WalletView.vue'
import NightMarketView from './views/NightMarketView.vue'
import WishlistView from './views/WishlistView.vue'
import ShopHistoryView from './views/ShopHistoryView.vue'
import LoadoutBuilderView from './views/LoadoutBuilderView.vue'
import BattlePassView from './views/BattlePassView.vue'
import RankChartView from './views/RankChartView.vue'

const LOGIN_URL = 'https://login.migueltaibo.com'

const user = ref(null)
const appReady = ref(false)
const linkedAccount = ref(null)     // user's own linked account
const activePlayer = ref(null)      // player currently being viewed
const currentView = ref('dashboard')
const showAccountSetup = ref(false)
const showExtensionSetup = ref(false)
const searchLoading = ref(false)
const searchError = ref('')

const isOwnAccount = computed(() =>
  !!(activePlayer.value && linkedAccount.value &&
  activePlayer.value.riot_name?.toLowerCase() === linkedAccount.value.riot_name?.toLowerCase() &&
  activePlayer.value.riot_tag?.toLowerCase() === linkedAccount.value.riot_tag?.toLowerCase())
)

const viewComponent = computed(() => {
  switch (currentView.value) {
    case 'stats': return StatsView
    case 'shop': return ShopView
    case 'inventory': return InventoryView
    case 'esports': return EsportsView
    case 'wallet': return WalletView
    case 'night-market': return NightMarketView
    case 'wishlist': return WishlistView
    case 'shop-history': return ShopHistoryView
    case 'loadout': return LoadoutBuilderView
    case 'battle-pass': return BattlePassView
    case 'rank-chart': return RankChartView
    default: return DashboardView
  }
})

async function loadLinkedAccount() {
  try {
    const r = await fetch('/api/val/account')
    if (r.ok) {
      const acc = await r.json()
      linkedAccount.value = acc
      if (acc && !activePlayer.value) {
        activePlayer.value = { riot_name: acc.riot_name, riot_tag: acc.riot_tag, region: acc.region, puuid: acc.puuid }
      }
    }
  } catch { /* ignore */ }
}

async function onSearch(name, tag) {
  searchError.value = ''
  searchLoading.value = true
  try {
    const r = await fetch(`/api/val/player/profile?name=${encodeURIComponent(name)}&tag=${encodeURIComponent(tag)}`)
    if (!r.ok) {
      const d = await r.json()
      searchError.value = d.detail || 'Player not found'
      return
    }
    const data = await r.json()
    activePlayer.value = {
      riot_name: data.name,
      riot_tag: data.tag,
      region: data.region,
      puuid: data.puuid,
    }
    currentView.value = 'dashboard'
  } catch {
    searchError.value = 'Network error'
  } finally {
    searchLoading.value = false
  }
}

function onAccountLinked(acc) {
  linkedAccount.value = acc
  activePlayer.value = { riot_name: acc.riot_name, riot_tag: acc.riot_tag, region: acc.region, puuid: acc.puuid }
  showAccountSetup.value = false
}

onMounted(async () => {
  try {
    const res = await fetch('/auth/passkey/me')
    if (res.ok) {
      user.value = await res.json()
    } else {
      window.location.href = `${LOGIN_URL}/?redirect=val.migueltaibo.com`
      return
    }
  } catch {
    window.location.href = `${LOGIN_URL}/?redirect=val.migueltaibo.com`
    return
  }

  await loadLinkedAccount()
  appReady.value = true
})
</script>

<template>
  <AccountSetupModal
    v-if="showAccountSetup"
    @linked="onAccountLinked"
    @close="showAccountSetup = false"
  />

  <ExtensionSetupModal
    v-if="showExtensionSetup"
    @saved="showExtensionSetup = false"
    @close="showExtensionSetup = false"
  />

  <div v-if="appReady && user" class="app">
    <Sidebar
      :user="user"
      :current-view="currentView"
      :linked-account="linkedAccount"
      :active-player="activePlayer"
      :is-own-account="isOwnAccount"
      :search-loading="searchLoading"
      :search-error="searchError"
      @navigate="currentView = $event"
      @search="onSearch"
      @open-account-setup="showAccountSetup = true"
      @open-extension-setup="showExtensionSetup = true"
      @go-home="activePlayer = linkedAccount ? { riot_name: linkedAccount.riot_name, riot_tag: linkedAccount.riot_tag, region: linkedAccount.region, puuid: linkedAccount.puuid } : null"
    />
    <main class="main-content">
      <div v-if="!activePlayer" class="search-landing">
        <div class="search-landing-inner">
          <div class="landing-logo">
            <svg viewBox="0 0 32 32" width="48" height="48">
              <rect width="32" height="32" rx="8" fill="#FF4655"/>
              <path d="M8 10 L16 22 L24 10" stroke="white" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
            </svg>
          </div>
          <h1 class="landing-title">Search a player</h1>
          <p class="landing-sub">Enter a Riot ID in the sidebar to see stats, rank and match history</p>
          <p v-if="!linkedAccount" class="landing-link" @click="showAccountSetup = true">
            Link your own account →
          </p>
        </div>
      </div>

      <component
        v-else
        :is="viewComponent"
        :player="activePlayer"
        :linked-account="linkedAccount"
        :is-own-account="isOwnAccount"
        :user="user"
        @reload-account="loadLinkedAccount"
      />
    </main>
  </div>

  <div v-else class="loading-screen">
    <span>Loading…</span>
  </div>
</template>

<style>
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; user-select: none; -webkit-user-select: none; }
html, body, #app { height: 100%; background: #000; color: #fff; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif; -webkit-font-smoothing: antialiased; }
::-webkit-scrollbar { display: none; }
* { -ms-overflow-style: none; scrollbar-width: none; }
input, textarea, [contenteditable] { user-select: text; -webkit-user-select: text; }
</style>

<style scoped>
.app { display: flex; height: 100dvh; background: #000; overflow: hidden; }

.main-content {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

.search-landing {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.search-landing-inner {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  text-align: center;
  padding: 32px;
}

.landing-logo { margin-bottom: 8px; }

.landing-title {
  font-size: 1.5rem;
  font-weight: 700;
  color: #fff;
  letter-spacing: -0.02em;
}

.landing-sub {
  font-size: 0.875rem;
  color: #636366;
  max-width: 280px;
  line-height: 1.5;
}

.landing-link {
  font-size: 0.8rem;
  color: #FF4655;
  cursor: default;
  margin-top: 8px;
}

.landing-link:hover { text-decoration: underline; }

.loading-screen {
  height: 100dvh;
  background: #000;
  display: flex;
  align-items: center;
  justify-content: center;
}

.loading-screen span { font-size: 0.875rem; color: #525252; }
</style>
