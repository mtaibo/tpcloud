<script setup>
import { ref, onMounted } from 'vue'
import { Trash2, Star } from 'lucide-vue-next'

defineProps({ player: Object, linkedAccount: Object, isOwnAccount: Boolean, user: Object })

const items = ref([])
const matches = ref([])
const loading = ref(true)

async function load() {
  loading.value = true
  try {
    const [r1, r2] = await Promise.all([
      fetch('/api/val/wishlist'),
      fetch('/api/val/wishlist/matches'),
    ])
    if (r1.ok) items.value = (await r1.json()).items
    if (r2.ok) matches.value = (await r2.json()).matches
  } finally { loading.value = false }
}

async function remove(id) {
  await fetch(`/api/val/wishlist/${id}`, { method: 'DELETE' })
  items.value = items.value.filter(i => i.id !== id)
}

async function bumpPriority(item) {
  const p = (item.priority + 1) % 4
  await fetch(`/api/val/wishlist/${item.id}`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ priority: p }),
  })
  item.priority = p
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

function isMatch(skinUuid) {
  return matches.value.some(m => m.skin_uuid === skinUuid)
}

onMounted(load)
</script>

<template>
  <div class="view">
    <div class="view-header">
      <h1 class="view-title">Wishlist</h1>
      <span v-if="matches.length" class="match-badge">{{ matches.length }} en tu shop hoy</span>
    </div>

    <p v-if="!loading && !items.length" class="empty">
      Añade skins a tu wishlist desde el inventario o la vista de skin. Te avisaremos cuando aparezcan en tu tienda.
    </p>

    <div class="grid">
      <div v-for="item in items" :key="item.id" class="card" :class="{ hot: isMatch(item.skin_uuid) }">
        <div class="art">
          <img v-if="item.display_icon" :src="item.display_icon" :alt="item.skin_name" loading="lazy" />
          <span v-if="isMatch(item.skin_uuid)" class="hot-badge">En tu shop</span>
        </div>
        <div class="footer">
          <div class="tier-dot" :style="{ background: tierColor(item.content_tier_uuid) }" />
          <span class="name">{{ item.skin_name }}</span>
          <button class="btn-icon" @click="bumpPriority(item)" :title="`Priority ${item.priority}`">
            <Star :size="14" :fill="item.priority ? '#ffd60a' : 'none'" :stroke="item.priority ? '#ffd60a' : '#636366'" />
          </button>
          <button class="btn-icon" @click="remove(item.id)"><Trash2 :size="14" /></button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.view { flex: 1; overflow-y: auto; }
.view-header { display: flex; align-items: center; justify-content: space-between; padding: 32px 32px 24px; }
.view-title { font-size: 1.5rem; font-weight: 700; letter-spacing: -0.02em; }
.match-badge { font-size: 0.75rem; color: #FF4655; background: rgba(255,70,85,0.12); padding: 3px 10px; border-radius: 999px; font-weight: 600; }
.empty { padding: 60px 32px; text-align: center; color: #636366; font-size: 0.875rem; }
.grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; padding: 0 32px 32px; }
.card { border-radius: 12px; background: rgba(255,255,255,0.03); border: 0.5px solid rgba(255,255,255,0.06); overflow: hidden; }
.card.hot { border-color: rgba(255,70,85,0.5); background: rgba(255,70,85,0.05); }
.art { position: relative; aspect-ratio: 16/9; display: flex; align-items: center; justify-content: center; padding: 12px; }
.art img { width: 100%; height: 100%; object-fit: contain; }
.hot-badge { position: absolute; top: 6px; right: 6px; font-size: 0.65rem; background: #FF4655; color: #fff; padding: 2px 6px; border-radius: 4px; font-weight: 700; }
.footer { display: flex; align-items: center; gap: 6px; padding: 8px 10px; background: rgba(255,255,255,0.03); }
.tier-dot { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }
.name { flex: 1; font-size: 0.75rem; color: #d1d1d6; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.btn-icon { padding: 4px; background: none; border: none; color: #636366; cursor: pointer; border-radius: 4px; transition: color 0.15s, background 0.15s; }
.btn-icon:hover { color: #ff453a; background: rgba(255,255,255,0.05); }

@media (max-width: 1100px) { .grid { grid-template-columns: repeat(3, 1fr); } }
@media (max-width: 767px) { .grid { grid-template-columns: repeat(2, 1fr); padding: 0 16px 16px; } }
</style>
