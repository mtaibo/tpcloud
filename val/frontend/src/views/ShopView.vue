<script setup>
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { ShoppingBag, RefreshCw, Lock } from 'lucide-vue-next'
import ExtensionSetupModal from '../components/ExtensionSetupModal.vue'

defineProps({ player: Object, linkedAccount: Object, isOwnAccount: Boolean, user: Object })

const shop = ref(null)
const loading = ref(true)
const error = ref('')
const refreshing = ref(false)
const showCredentials = ref(false)
const countdown = ref('')
let timer = null

function updateCountdown(expiresAt) {
  if (!expiresAt) return
  const diff = new Date(expiresAt).getTime() - Date.now()
  if (diff <= 0) { countdown.value = 'Refreshing…'; return }
  const h = Math.floor(diff / 3600000)
  const m = Math.floor((diff % 3600000) / 60000)
  const s = Math.floor((diff % 60000) / 1000)
  countdown.value = `${String(h).padStart(2,'0')}:${String(m).padStart(2,'0')}:${String(s).padStart(2,'0')}`
}

async function loadShop() {
  try {
    const r = await fetch('/api/val/shop')
    if (r.ok) {
      shop.value = await r.json()
      if (shop.value?.expires_at) {
        updateCountdown(shop.value.expires_at)
        timer = setInterval(() => updateCountdown(shop.value?.expires_at), 1000)
      }
    } else {
      error.value = 'Failed to load shop'
    }
  } catch {
    error.value = 'Network error'
  } finally {
    loading.value = false
  }
}

async function refresh() {
  refreshing.value = true
  await fetch('/api/val/shop/refresh', { method: 'POST' })
  shop.value = null
  loading.value = true
  if (timer) { clearInterval(timer); timer = null }
  await loadShop()
  refreshing.value = false
}

function onCredentialsSaved() {
  showCredentials.value = false
  loading.value = true
  error.value = ''
  loadShop()
}

// Tier color mapping
function tierColor(uuid) {
  if (!uuid) return '#636366'
  const map = {
    '12683d76-48d7-84a3-4e09-6985794f0445': '#009587', // Select
    '0cebb8be-46d7-c12a-d306-e9907bfc5a25': '#d1548d', // Deluxe
    'e046854e-406c-37f4-6607-19a9ba8426fc': '#ff7044', // Premium
    '411e4a55-4e59-7757-41f0-86a53f101bb5': '#f4d03f', // Exclusive
    'e50bf55b-4161-234d-361b-344f74993e55': '#f1c40f', // Ultra
  }
  return map[uuid] ?? '#636366'
}

onMounted(loadShop)
onUnmounted(() => { if (timer) clearInterval(timer) })
</script>

<template>
  <div class="view">
    <div class="view-header">
      <div class="header-left">
        <h1 class="view-title">Daily Shop</h1>
        <span v-if="countdown" class="countdown">Resets in {{ countdown }}</span>
      </div>
      <button v-if="shop && !shop.requires_credentials" :disabled="refreshing" class="btn-refresh" @click="refresh">
        <RefreshCw :size="14" :class="{ spinning: refreshing }" />
      </button>
    </div>

    <!-- Requires credentials -->
    <div v-if="!loading && shop?.requires_credentials" class="cta-card">
      <Lock :size="28" color="#FF4655" />
      <h3 class="cta-title">Connect your Riot account</h3>
      <p class="cta-text">Enter your Riot credentials to view your personal daily shop and skin offers.</p>
      <button class="btn-primary" @click="showCredentials = true">Connect Account</button>
    </div>

    <!-- Loading -->
    <div v-else-if="loading" class="offers-grid">
      <div v-for="i in 4" :key="i" class="offer-skeleton" />
    </div>

    <!-- Offers -->
    <div v-else-if="shop?.offers?.length" class="offers-grid">
      <div v-for="offer in shop.offers" :key="offer.offer_id" class="offer-card">
        <div class="offer-art">
          <img v-if="offer.display_icon" :src="offer.display_icon" :alt="offer.skin_name" class="offer-img" loading="lazy" />
          <ShoppingBag v-else :size="32" color="#3a3a3c" />
        </div>
        <div class="offer-footer">
          <div class="offer-tier-dot" :style="{ background: tierColor(offer.content_tier_uuid) }" />
          <span class="offer-name">{{ offer.skin_name }}</span>
          <span class="offer-price">{{ offer.vp_cost?.toLocaleString() }} VP</span>
        </div>
      </div>
    </div>

    <!-- Error -->
    <div v-else-if="error" class="error-banner">{{ error }}</div>

    <!-- Bundle -->
    <section v-if="shop?.bundle" class="section">
      <h2 class="section-title">Featured Bundle</h2>
      <div class="bundle-card">
        <span class="bundle-name">{{ shop.bundle.name ?? 'Bundle' }}</span>
        <span class="bundle-items">{{ shop.bundle.items_count }} items</span>
      </div>
    </section>

    <ExtensionSetupModal v-if="showCredentials" @close="showCredentials = false" @saved="onCredentialsSaved" />
  </div>
</template>

<style scoped>
.view { flex: 1; overflow-y: auto; }

.view-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 32px 32px 24px;
}

.header-left { display: flex; flex-direction: column; gap: 4px; }
.view-title { font-size: 1.5rem; font-weight: 700; letter-spacing: -0.02em; }
.countdown { font-size: 0.75rem; color: #636366; font-variant-numeric: tabular-nums; }

.btn-refresh {
  padding: 6px 10px;
  background: rgba(255,255,255,0.06);
  border: 0.5px solid rgba(255,255,255,0.1);
  border-radius: 8px;
  color: #8E8E93;
  cursor: default;
  transition: background 0.12s;
}

.btn-refresh:hover:not(:disabled) { background: rgba(255,255,255,0.1); color: #fff; }

.spinning { animation: spin 1s linear infinite; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }

.cta-card {
  margin: 0 32px;
  padding: 40px;
  border-radius: 14px;
  background: rgba(255,255,255,0.03);
  border: 0.5px solid rgba(255,255,255,0.06);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  text-align: center;
}

.cta-title { font-size: 1.125rem; font-weight: 600; }
.cta-text { font-size: 0.875rem; color: #636366; max-width: 360px; line-height: 1.5; }

.offers-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
  padding: 0 32px 32px;
}

.offer-card {
  border-radius: 12px;
  background: rgba(255,255,255,0.03);
  border: 0.5px solid rgba(255,255,255,0.06);
  overflow: hidden;
  transition: border-color 0.15s;
}

.offer-card:hover { border-color: rgba(255,255,255,0.12); }

.offer-art {
  aspect-ratio: 16/7;
  background: rgba(255,255,255,0.03);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
}

.offer-img { width: 100%; height: 100%; object-fit: contain; }

.offer-footer {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  background: rgba(255,255,255,0.03);
}

.offer-tier-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
.offer-name { font-size: 0.8rem; font-weight: 500; color: #d1d1d6; flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.offer-price { font-size: 0.75rem; font-weight: 600; color: #8E8E93; flex-shrink: 0; }

.offer-skeleton { aspect-ratio: 16/10; border-radius: 12px; background: rgba(255,255,255,0.04); animation: pulse 1.5s infinite; }

.section { padding: 0 32px 32px; }
.section-title { font-size: 0.875rem; font-weight: 600; color: #8E8E93; letter-spacing: 0.02em; text-transform: uppercase; margin-bottom: 10px; }

.bundle-card {
  padding: 14px 16px;
  border-radius: 10px;
  background: rgba(255,255,255,0.03);
  border: 0.5px solid rgba(255,255,255,0.06);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.bundle-name { font-size: 0.875rem; font-weight: 500; color: #d1d1d6; }
.bundle-items { font-size: 0.75rem; color: #636366; }

.error-banner { padding: 20px 32px; font-size: 0.875rem; color: #ff453a; }

@keyframes pulse { 0%,100%{opacity:1}50%{opacity:.5} }
@media (max-width: 767px) { .offers-grid { grid-template-columns: 1fr; padding: 0 16px 16px; } .view-header { padding: 16px; } .cta-card { margin: 0 16px; } }
</style>
