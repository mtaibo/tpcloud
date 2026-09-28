<script setup>
import { ref, onMounted } from 'vue'
import { Lock, Moon } from 'lucide-vue-next'
import ExtensionSetupModal from '../components/ExtensionSetupModal.vue'

defineProps({ player: Object, linkedAccount: Object, isOwnAccount: Boolean, user: Object })

const market = ref(null)
const loading = ref(true)
const error = ref('')
const showCredentials = ref(false)

async function load() {
  loading.value = true
  try {
    const r = await fetch('/api/val/night-market')
    if (r.ok) market.value = await r.json()
    else error.value = 'Failed to load'
  } catch { error.value = 'Network error' } finally { loading.value = false }
}

function onCredentialsSaved() { showCredentials.value = false; load() }

function tierColor(uuid) {
  const map = {
    '12683d76-48d7-84a3-4e09-6985794f0445': '#009587',
    '0cebb8be-46d7-c12a-d306-e9907bfc5a25': '#d1548d',
    'e046854e-406c-37f4-6607-19a9ba8426fc': '#ff7044',
    '411e4a55-4e59-7757-41f0-86a53f101bb5': '#f4d03f',
    'e50bf55b-4161-234d-361b-344f74993e55': '#f1c40f',
  }
  return map[uuid] ?? '#636366'
}

onMounted(load)
</script>

<template>
  <div class="view">
    <div class="view-header">
      <h1 class="view-title">Night Market</h1>
    </div>

    <div v-if="!loading && market?.requires_credentials" class="cta-card">
      <Lock :size="28" color="#FF4655" />
      <h3 class="cta-title">Conecta tu cuenta Riot</h3>
      <button class="btn-primary" @click="showCredentials = true">Connect</button>
    </div>

    <div v-else-if="loading" class="offers-grid">
      <div v-for="i in 6" :key="i" class="offer-skeleton" />
    </div>

    <div v-else-if="market && !market.active" class="empty">
      <Moon :size="36" color="#3a3a3c" />
      <p class="empty-title">Night Market cerrado</p>
      <p class="empty-sub">Riot activa el Night Market periódicamente. Vuelve cuando esté disponible.</p>
    </div>

    <div v-else-if="market?.offers?.length" class="offers-grid">
      <div v-for="o in market.offers" :key="o.offer_id" class="offer-card">
        <div class="offer-art">
          <img v-if="o.display_icon" :src="o.display_icon" :alt="o.skin_name" loading="lazy" />
          <span v-if="o.discount_percent" class="badge">-{{ o.discount_percent }}%</span>
        </div>
        <div class="offer-footer">
          <div class="offer-tier-dot" :style="{ background: tierColor(o.content_tier_uuid) }" />
          <span class="offer-name">{{ o.skin_name }}</span>
          <div class="offer-prices">
            <span class="orig">{{ o.vp_cost?.toLocaleString() }}</span>
            <span class="disc">{{ o.discounted_cost?.toLocaleString() }} VP</span>
          </div>
        </div>
      </div>
    </div>

    <p v-if="error" class="error-banner">{{ error }}</p>

    <ExtensionSetupModal v-if="showCredentials" @close="showCredentials = false" @saved="onCredentialsSaved" />
  </div>
</template>

<style scoped>
.view { flex: 1; overflow-y: auto; }
.view-header { padding: 32px 32px 24px; }
.view-title { font-size: 1.5rem; font-weight: 700; letter-spacing: -0.02em; }

.offers-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; padding: 0 32px 32px; }
.offer-card { border-radius: 12px; background: rgba(255,255,255,0.03); border: 0.5px solid rgba(255,255,255,0.06); overflow: hidden; }
.offer-art { position: relative; aspect-ratio: 16/9; display: flex; align-items: center; justify-content: center; padding: 12px; }
.offer-art img { width: 100%; height: 100%; object-fit: contain; }
.badge { position: absolute; top: 8px; right: 8px; background: #FF4655; color: #fff; font-size: 0.68rem; font-weight: 700; padding: 3px 6px; border-radius: 5px; }
.offer-footer { display: flex; align-items: center; gap: 8px; padding: 10px 12px; background: rgba(255,255,255,0.03); }
.offer-tier-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
.offer-name { font-size: 0.78rem; font-weight: 500; color: #d1d1d6; flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.offer-prices { display: flex; flex-direction: column; align-items: flex-end; gap: 1px; }
.orig { font-size: 0.66rem; color: #525252; text-decoration: line-through; }
.disc { font-size: 0.72rem; font-weight: 700; color: #FF4655; }
.offer-skeleton { aspect-ratio: 16/10; border-radius: 12px; background: rgba(255,255,255,0.04); animation: pulse 1.5s infinite; }

.empty { margin: 0 32px; padding: 60px 40px; border-radius: 14px; background: rgba(255,255,255,0.03); display: flex; flex-direction: column; align-items: center; gap: 10px; text-align: center; }
.empty-title { font-size: 1rem; font-weight: 600; color: #d1d1d6; }
.empty-sub { font-size: 0.82rem; color: #636366; max-width: 320px; }

.cta-card { margin: 0 32px; padding: 40px; border-radius: 14px; background: rgba(255,255,255,0.03); display: flex; flex-direction: column; align-items: center; gap: 12px; text-align: center; }
.cta-title { font-size: 1.125rem; font-weight: 600; }
.btn-primary { padding: 10px 22px; border-radius: 8px; background: #FF4655; border: none; color: #fff; font-size: 0.82rem; font-weight: 700; cursor: pointer; }
.error-banner { padding: 20px 32px; font-size: 0.875rem; color: #ff453a; }

@keyframes pulse { 0%,100%{opacity:1} 50%{opacity:.5} }
@media (max-width: 1100px) { .offers-grid { grid-template-columns: repeat(2, 1fr); } }
@media (max-width: 767px) { .offers-grid { grid-template-columns: 1fr; padding: 0 16px 16px; } }
</style>
