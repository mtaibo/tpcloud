<script setup>
import { ref, onMounted } from 'vue'
import RankBadge from '../components/RankBadge.vue'
import MatchCard from '../components/MatchCard.vue'

const props = defineProps({
  linkedAccount: Object,
  user: Object,
})

const profile = ref(null)
const rank = ref(null)
const matches = ref([])
const loading = ref(true)
const error = ref('')

onMounted(async () => {
  try {
    const [profileRes, rankRes, matchRes] = await Promise.all([
      fetch('/api/val/player/profile'),
      fetch('/api/val/player/rank'),
      fetch('/api/val/matches?mode=competitive&size=5'),
    ])

    if (profileRes.ok) profile.value = await profileRes.json()
    if (rankRes.ok) rank.value = await rankRes.json()
    if (matchRes.ok) matches.value = await matchRes.json()
  } catch {
    error.value = 'Failed to load data'
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div class="view">

    <!-- Player header -->
    <div v-if="!loading && profile" class="player-header" :style="profile.card?.wide ? `background-image: url('${profile.card.wide}')` : ''">
      <div class="player-header-overlay">
        <div class="player-info">
          <img v-if="profile.card?.small" :src="profile.card.small" class="player-card-img" alt="Player card" />
          <div class="player-text">
            <h1 class="player-name">{{ profile.name }}<span class="player-tag">#{{ profile.tag }}</span></h1>
            <p class="player-meta">Level {{ profile.account_level }} · {{ linkedAccount?.region?.toUpperCase() }}</p>
          </div>
        </div>
        <RankBadge v-if="rank" :tier="rank.tier" :tier-id="rank.tier_id" :rr="rank.rr" :mmr-change="rank.mmr_change" :images="rank.images" size="lg" />
      </div>
    </div>

    <div v-else-if="loading" class="player-header skeleton" />

    <div v-else-if="error" class="error-banner">{{ error }}</div>

    <!-- Recent matches -->
    <section class="section">
      <div class="section-header">
        <h2 class="section-title">Recent Matches</h2>
        <span v-if="rank?.peak?.tier" class="peak-badge">Peak: {{ rank.peak.tier }}</span>
      </div>

      <div v-if="loading" class="matches-list">
        <div v-for="i in 5" :key="i" class="match-skeleton" />
      </div>

      <div v-else-if="matches.length" class="matches-list">
        <MatchCard v-for="match in matches" :key="match.match_id" :match="match" />
      </div>

      <p v-else class="empty-hint">No recent competitive matches found</p>
    </section>

  </div>
</template>

<style scoped>
.view { flex: 1; display: flex; flex-direction: column; overflow-y: auto; }

.player-header {
  height: 180px;
  background: #1c1c1e;
  background-size: cover;
  background-position: center;
  flex-shrink: 0;
  position: relative;
}

.player-header.skeleton { background: rgba(255, 255, 255, 0.04); animation: pulse 1.5s infinite; }

.player-header-overlay {
  position: absolute;
  inset: 0;
  background: linear-gradient(to right, rgba(0,0,0,0.85) 0%, rgba(0,0,0,0.4) 60%, rgba(0,0,0,0.6) 100%);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 24px 32px;
}

.player-info { display: flex; align-items: center; gap: 14px; }

.player-card-img {
  width: 52px;
  height: 52px;
  border-radius: 8px;
  object-fit: cover;
  border: 1px solid rgba(255, 255, 255, 0.15);
}

.player-name {
  font-size: 1.375rem;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: #fff;
}

.player-tag { color: #8E8E93; font-weight: 400; font-size: 1rem; }

.player-meta { font-size: 0.8rem; color: #8E8E93; margin-top: 3px; }

.error-banner {
  padding: 20px 32px;
  font-size: 0.875rem;
  color: #ff453a;
}

.section { padding: 24px 32px; }

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.section-title { font-size: 1rem; font-weight: 600; color: #fff; }

.peak-badge {
  font-size: 0.7rem;
  font-weight: 600;
  color: #8E8E93;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 6px;
  padding: 2px 8px;
}

.matches-list { display: flex; flex-direction: column; gap: 6px; }

.match-skeleton {
  height: 58px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.04);
  animation: pulse 1.5s infinite;
}

.empty-hint { font-size: 0.875rem; color: #525252; }

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

@media (max-width: 767px) {
  .player-header-overlay { padding: 16px; flex-direction: column; align-items: flex-start; gap: 12px; }
  .section { padding: 16px; }
}
</style>
