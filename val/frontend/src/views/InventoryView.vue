<script setup>
import { ref, computed, onMounted } from 'vue'
import { Lock } from 'lucide-vue-next'
import SkinCard from '../components/SkinCard.vue'
import CredentialsModal from '../components/CredentialsModal.vue'

defineProps({ player: Object, linkedAccount: Object, isOwnAccount: Boolean, user: Object })

const inventory = ref(null)
const loading = ref(true)
const error = ref('')
const activeFilter = ref('All')
const showCredentials = ref(false)

const filters = ['All', 'Vandal', 'Phantom', 'Operator', 'Spectre', 'Ghost', 'Classic', 'Sheriff', 'Knife']

const filteredSkins = computed(() => {
  if (!inventory.value?.skins) return []
  if (activeFilter.value === 'All') return inventory.value.skins
  return inventory.value.skins.filter(s =>
    (s.weapon_type ?? '').toLowerCase().includes(activeFilter.value.toLowerCase())
  )
})

async function load(weapon) {
  loading.value = true
  error.value = ''
  try {
    const url = weapon && weapon !== 'All' ? `/api/val/inventory?weapon=${weapon}` : '/api/val/inventory'
    const r = await fetch(url)
    if (r.ok) inventory.value = await r.json()
    else error.value = 'Failed to load inventory'
  } catch {
    error.value = 'Network error'
  } finally {
    loading.value = false
  }
}

function onCredentialsSaved() {
  showCredentials.value = false
  load()
}

onMounted(() => load())
</script>

<template>
  <div class="view">
    <div class="view-header">
      <h1 class="view-title">Inventory</h1>
      <span v-if="inventory?.total" class="total-badge">{{ inventory.total }} items</span>
    </div>

    <!-- Requires credentials -->
    <div v-if="!loading && inventory?.requires_credentials" class="cta-card">
      <Lock :size="28" color="#FF4655" />
      <h3 class="cta-title">Connect your Riot account</h3>
      <p class="cta-text">Enter your Riot credentials to view your owned skins and inventory.</p>
      <button class="btn-primary" @click="showCredentials = true">Connect Account</button>
    </div>

    <template v-else>
      <!-- Filter bar -->
      <div class="filter-bar">
        <button
          v-for="f in filters"
          :key="f"
          :class="['filter-pill', { active: activeFilter === f }]"
          @click="activeFilter = f"
        >{{ f }}</button>
      </div>

      <!-- Grid -->
      <div v-if="loading" class="skins-grid">
        <div v-for="i in 12" :key="i" class="skin-skeleton" />
      </div>

      <div v-else-if="filteredSkins.length" class="skins-grid">
        <SkinCard v-for="skin in filteredSkins" :key="skin.uuid" :skin="skin" />
      </div>

      <p v-else-if="!error" class="empty-hint">No skins found</p>

      <p v-if="error" class="error-banner">{{ error }}</p>
    </template>

    <CredentialsModal v-if="showCredentials" @close="showCredentials = false" @saved="onCredentialsSaved" />
  </div>
</template>

<style scoped>
.view { flex: 1; overflow-y: auto; }

.view-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 32px 32px 0;
}

.view-title { font-size: 1.5rem; font-weight: 700; letter-spacing: -0.02em; }

.total-badge {
  font-size: 0.75rem;
  color: #636366;
  background: rgba(255,255,255,0.06);
  border-radius: 6px;
  padding: 2px 8px;
}

.cta-card {
  margin: 24px 32px;
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

.filter-bar {
  display: flex;
  gap: 6px;
  padding: 20px 32px 16px;
  overflow-x: auto;
  flex-shrink: 0;
}

.filter-pill {
  padding: 5px 12px;
  border-radius: 20px;
  font-size: 0.775rem;
  font-weight: 500;
  background: rgba(255,255,255,0.06);
  border: 0.5px solid rgba(255,255,255,0.08);
  color: #8E8E93;
  white-space: nowrap;
  cursor: default;
  transition: background 0.12s, color 0.12s, border-color 0.12s;
  flex-shrink: 0;
}

.filter-pill:hover:not(.active) { background: rgba(255,255,255,0.1); color: #d1d1d6; }

.filter-pill.active {
  background: rgba(255, 70, 85, 0.15);
  border-color: rgba(255, 70, 85, 0.35);
  color: #FF4655;
}

.skins-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 10px;
  padding: 0 32px 32px;
}

.skin-skeleton { aspect-ratio: 16/10; border-radius: 10px; background: rgba(255,255,255,0.04); animation: pulse 1.5s infinite; }
.empty-hint { padding: 20px 32px; font-size: 0.875rem; color: #525252; }
.error-banner { padding: 20px 32px; font-size: 0.875rem; color: #ff453a; }

@keyframes pulse { 0%,100%{opacity:1}50%{opacity:.5} }
@media (max-width: 1100px) { .skins-grid { grid-template-columns: repeat(3, 1fr); } }
@media (max-width: 767px) { .skins-grid { grid-template-columns: repeat(2, 1fr); padding: 0 16px 16px; } .filter-bar { padding: 16px 16px 12px; } .view-header { padding: 16px 16px 0; } }
</style>
