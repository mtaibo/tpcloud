<script setup>
import { ref, computed, onMounted } from 'vue'
import { Trophy, Radio } from 'lucide-vue-next'

defineProps({ linkedAccount: Object })

const data = ref(null)
const loading = ref(true)
const error = ref('')
const selectedLeague = ref(null)

onMounted(async () => {
  try {
    const r = await fetch('/api/val/esports/overview')
    if (r.ok) {
      data.value = await r.json()
      if (data.value?.leagues?.length) {
        selectedLeague.value = data.value.leagues[0].id
      }
    } else {
      error.value = 'Esports data unavailable'
    }
  } catch {
    error.value = 'Network error'
  } finally {
    loading.value = false
  }
})

const currentLeagueEvents = computed(() => {
  if (!data.value?.schedule || !selectedLeague.value) return []
  return data.value.schedule.filter(e => {
    const lid = e.league?.id ?? ''
    return lid === selectedLeague.value
  })
})

const recentResults = computed(() => {
  if (!currentLeagueEvents.value.length) return []
  return currentLeagueEvents.value
    .filter(e => e.state === 'completed')
    .slice(-10)
    .reverse()
})

const upcoming = computed(() => {
  if (!currentLeagueEvents.value.length) return []
  return currentLeagueEvents.value
    .filter(e => e.state === 'unstarted' || e.state === 'inProgress')
    .slice(0, 6)
})

function formatDate(str) {
  if (!str) return ''
  return new Date(str).toLocaleDateString('en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
}

function teamName(match, side) {
  return match?.match?.teams?.[side]?.name ?? match?.match?.teams?.[side]?.code ?? '?'
}
</script>

<template>
  <div class="view">
    <div class="view-header">
      <h1 class="view-title">Esports</h1>
    </div>

    <!-- League tabs -->
    <div v-if="data?.leagues?.length" class="league-tabs">
      <button
        v-for="league in data.leagues"
        :key="league.id"
        :class="['league-tab', { active: selectedLeague === league.id }]"
        @click="selectedLeague = league.id"
      >{{ league.name }}</button>
    </div>

    <!-- Live indicator -->
    <div v-if="data?.live?.length" class="live-banner">
      <Radio :size="14" color="#30d158" class="live-icon" />
      <span class="live-text">{{ data.live.length }} live event{{ data.live.length > 1 ? 's' : '' }}</span>
    </div>

    <div v-if="loading" class="loading-state">
      <div v-for="i in 6" :key="i" class="event-skeleton" />
    </div>

    <template v-else-if="error">
      <div class="error-banner">{{ error }}</div>
    </template>

    <template v-else-if="data">
      <!-- Upcoming -->
      <section v-if="upcoming.length" class="section">
        <h2 class="section-title">Upcoming</h2>
        <div class="events-list">
          <div v-for="event in upcoming" :key="event.id" class="event-row">
            <div class="event-teams">
              <span class="team-name">{{ teamName(event, 0) }}</span>
              <span class="vs">vs</span>
              <span class="team-name">{{ teamName(event, 1) }}</span>
            </div>
            <div class="event-meta">
              <span class="event-date">{{ formatDate(event.startTime) }}</span>
              <span v-if="event.state === 'inProgress'" class="live-pill">LIVE</span>
            </div>
          </div>
        </div>
      </section>

      <!-- Recent results -->
      <section v-if="recentResults.length" class="section">
        <h2 class="section-title">Recent Results</h2>
        <div class="events-list">
          <div v-for="event in recentResults" :key="event.id" class="event-row result-row">
            <div class="event-teams">
              <span class="team-name">{{ teamName(event, 0) }}</span>
              <span class="score-badge">
                {{ event.match?.teams?.[0]?.result?.gameWins ?? '-' }} - {{ event.match?.teams?.[1]?.result?.gameWins ?? '-' }}
              </span>
              <span class="team-name">{{ teamName(event, 1) }}</span>
            </div>
            <span class="event-date">{{ formatDate(event.startTime) }}</span>
          </div>
        </div>
      </section>

      <!-- Empty -->
      <p v-if="!upcoming.length && !recentResults.length" class="empty-hint">No events found for this league</p>
    </template>

    <p v-else class="empty-hint">No esports data available</p>
  </div>
</template>

<style scoped>
.view { flex: 1; overflow-y: auto; }

.view-header { padding: 32px 32px 0; }
.view-title { font-size: 1.5rem; font-weight: 700; letter-spacing: -0.02em; }

.league-tabs {
  display: flex;
  gap: 6px;
  padding: 20px 32px 0;
  overflow-x: auto;
}

.league-tab {
  padding: 6px 14px;
  border-radius: 20px;
  font-size: 0.8rem;
  font-weight: 500;
  background: rgba(255,255,255,0.06);
  border: 0.5px solid rgba(255,255,255,0.08);
  color: #8E8E93;
  white-space: nowrap;
  cursor: default;
  transition: all 0.12s;
  flex-shrink: 0;
}

.league-tab:hover:not(.active) { background: rgba(255,255,255,0.1); color: #d1d1d6; }
.league-tab.active { background: rgba(255,70,85,0.15); border-color: rgba(255,70,85,0.35); color: #FF4655; }

.live-banner {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 12px 32px 0;
}

.live-icon { animation: pulse-icon 2s ease-in-out infinite; }
@keyframes pulse-icon { 0%,100%{opacity:1}50%{opacity:.4} }

.live-text { font-size: 0.75rem; font-weight: 600; color: #30d158; }

.section { padding: 24px 32px; }
.section-title { font-size: 0.875rem; font-weight: 600; color: #8E8E93; letter-spacing: 0.02em; text-transform: uppercase; margin-bottom: 10px; }

.events-list { display: flex; flex-direction: column; gap: 4px; }

.event-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border-radius: 8px;
  background: rgba(255,255,255,0.03);
  border: 0.5px solid rgba(255,255,255,0.05);
}

.event-teams { display: flex; align-items: center; gap: 10px; }
.team-name { font-size: 0.875rem; font-weight: 500; color: #d1d1d6; }
.vs { font-size: 0.7rem; color: #525252; }

.score-badge {
  font-size: 0.775rem;
  font-weight: 700;
  color: #fff;
  background: rgba(255,255,255,0.08);
  border-radius: 4px;
  padding: 1px 6px;
  font-variant-numeric: tabular-nums;
}

.event-meta { display: flex; align-items: center; gap: 8px; }
.event-date { font-size: 0.75rem; color: #636366; }

.live-pill {
  font-size: 0.65rem;
  font-weight: 700;
  color: #30d158;
  background: rgba(48, 209, 88, 0.12);
  border-radius: 4px;
  padding: 1px 5px;
  letter-spacing: 0.04em;
}

.loading-state { display: flex; flex-direction: column; gap: 4px; padding: 24px 32px; }
.event-skeleton { height: 42px; border-radius: 8px; background: rgba(255,255,255,0.04); animation: pulse 1.5s infinite; }

.error-banner { padding: 20px 32px; font-size: 0.875rem; color: #ff453a; }
.empty-hint { padding: 20px 32px; font-size: 0.875rem; color: #525252; }

@keyframes pulse { 0%,100%{opacity:1}50%{opacity:.5} }
@media (max-width: 767px) { .section { padding: 16px; } .league-tabs { padding: 16px 16px 0; } .view-header { padding: 16px 16px 0; } }
</style>
