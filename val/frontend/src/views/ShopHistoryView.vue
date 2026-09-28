<script setup>
import { ref, onMounted } from 'vue'

defineProps({ player: Object, linkedAccount: Object, isOwnAccount: Boolean, user: Object })

const entries = ref([])
const loading = ref(true)

async function load() {
  loading.value = true
  try {
    const r = await fetch('/api/val/shop-history?limit=60')
    if (r.ok) entries.value = (await r.json()).entries
  } finally { loading.value = false }
}

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

function formatDate(iso) {
  return new Date(iso).toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short' })
}

onMounted(load)
</script>

<template>
  <div class="view">
    <div class="view-header">
      <h1 class="view-title">Shop History</h1>
    </div>

    <p v-if="!loading && !entries.length" class="empty">
      Aún no hay histórico. Cada vez que refresques la tienda, se guardará aquí una copia.
    </p>

    <div class="stack">
      <div v-for="e in entries" :key="e.shop_date" class="day">
        <p class="day-label">{{ formatDate(e.shop_date) }}</p>
        <div class="offers">
          <div v-for="o in e.offers" :key="o.offer_id" class="offer">
            <div class="art">
              <img v-if="o.display_icon" :src="o.display_icon" :alt="o.skin_name" loading="lazy" />
            </div>
            <div class="foot">
              <div class="tier-dot" :style="{ background: tierColor(o.content_tier_uuid) }" />
              <span class="name">{{ o.skin_name }}</span>
              <span class="price">{{ o.vp_cost?.toLocaleString() }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.view { flex: 1; overflow-y: auto; }
.view-header { padding: 32px 32px 24px; }
.view-title { font-size: 1.5rem; font-weight: 700; letter-spacing: -0.02em; }
.empty { padding: 60px 32px; text-align: center; color: #636366; font-size: 0.875rem; }

.stack { display: flex; flex-direction: column; gap: 20px; padding: 0 32px 32px; }
.day { }
.day-label { font-size: 0.75rem; color: #636366; text-transform: uppercase; letter-spacing: 0.06em; margin-bottom: 8px; font-weight: 600; }
.offers { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
.offer { border-radius: 10px; background: rgba(255,255,255,0.03); border: 0.5px solid rgba(255,255,255,0.05); overflow: hidden; }
.art { aspect-ratio: 16/9; display: flex; align-items: center; justify-content: center; padding: 8px; }
.art img { width: 100%; height: 100%; object-fit: contain; }
.foot { display: flex; align-items: center; gap: 6px; padding: 6px 10px; background: rgba(255,255,255,0.02); }
.tier-dot { width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0; }
.name { flex: 1; font-size: 0.7rem; color: #8E8E93; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.price { font-size: 0.65rem; color: #525252; font-weight: 600; }

@media (max-width: 767px) { .offers { grid-template-columns: repeat(2, 1fr); } .stack { padding: 0 16px 16px; } }
</style>
