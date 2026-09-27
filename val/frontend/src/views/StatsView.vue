<script setup>
import { ref, onMounted, computed } from 'vue'

defineProps({ linkedAccount: Object })

const history = ref([])
const loading = ref(true)

onMounted(async () => {
  try {
    const r = await fetch('/api/val/player/mmr-history')
    if (r.ok) history.value = (await r.json()).slice(0, 30).reverse()
  } catch { /* ignore */ }
  finally { loading.value = false }
})

const chartData = computed(() => {
  if (!history.value.length) return null
  const mmrValues = history.value.map(e => e.mmr).filter(Boolean)
  if (!mmrValues.length) return null

  const minMmr = Math.min(...mmrValues) - 50
  const maxMmr = Math.max(...mmrValues) + 50
  const range = maxMmr - minMmr || 1

  const W = 600
  const H = 160
  const PAD = 16

  const points = history.value.map((entry, i) => {
    const x = PAD + (i / Math.max(history.value.length - 1, 1)) * (W - PAD * 2)
    const y = H - PAD - ((entry.mmr - minMmr) / range) * (H - PAD * 2)
    return { x, y, entry }
  })

  const polyline = points.map(p => `${p.x},${p.y}`).join(' ')

  return { points, polyline, W, H, minMmr, maxMmr }
})

function mmrChangeColor(change) {
  if (change > 0) return '#30d158'
  if (change < 0) return '#ff453a'
  return '#636366'
}
</script>

<template>
  <div class="view">
    <div class="view-header">
      <h1 class="view-title">Stats</h1>
    </div>

    <!-- MMR Chart -->
    <section class="section">
      <h2 class="section-title">Rank Progression</h2>

      <div v-if="loading" class="chart-skeleton" />

      <div v-else-if="chartData" class="chart-wrap">
        <svg :viewBox="`0 0 ${chartData.W} ${chartData.H}`" class="chart-svg" preserveAspectRatio="none">
          <!-- Grid lines -->
          <line x1="16" :x2="chartData.W - 16" :y1="chartData.H - 16" :y2="chartData.H - 16" stroke="rgba(255,255,255,0.06)" stroke-width="1"/>
          <line x1="16" :x2="chartData.W - 16" :y1="(chartData.H - 16) / 2 + 8" :y2="(chartData.H - 16) / 2 + 8" stroke="rgba(255,255,255,0.04)" stroke-width="1" stroke-dasharray="4 4"/>

          <!-- Gradient fill -->
          <defs>
            <linearGradient id="mmrGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stop-color="#FF4655" stop-opacity="0.2"/>
              <stop offset="100%" stop-color="#FF4655" stop-opacity="0"/>
            </linearGradient>
          </defs>
          <polygon
            :points="chartData.polyline + ` ${chartData.points[chartData.points.length-1].x},${chartData.H - 16} 16,${chartData.H - 16}`"
            fill="url(#mmrGrad)"
          />

          <!-- Line -->
          <polyline :points="chartData.polyline" fill="none" stroke="#FF4655" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>

          <!-- Points -->
          <circle
            v-for="(p, i) in chartData.points"
            :key="i"
            :cx="p.x"
            :cy="p.y"
            r="3"
            :fill="(p.entry.mmr_change ?? 0) >= 0 ? '#30d158' : '#ff453a'"
          />
        </svg>

        <div class="chart-labels">
          <span class="chart-label">{{ chartData.minMmr + 50 }} MMR</span>
          <span class="chart-label">{{ chartData.maxMmr - 50 }} MMR</span>
        </div>
      </div>

      <p v-else class="empty-hint">No rank history available</p>
    </section>

    <!-- Recent MMR entries -->
    <section class="section">
      <h2 class="section-title">MMR History</h2>

      <div v-if="loading" class="history-list">
        <div v-for="i in 8" :key="i" class="history-skeleton" />
      </div>

      <div v-else-if="history.length" class="history-list">
        <div v-for="entry in [...history].reverse().slice(0, 20)" :key="entry.match_id" class="history-row">
          <img v-if="entry.images?.small" :src="entry.images.small" class="tier-img" :alt="entry.tier" />
          <div v-else class="tier-placeholder" />
          <div class="history-info">
            <span class="tier-name">{{ entry.tier ?? 'Unrated' }}</span>
            <span class="tier-rr">{{ entry.rr }} RR · {{ entry.mmr }} MMR</span>
          </div>
          <span :class="['mmr-change', { positive: (entry.mmr_change ?? 0) > 0, negative: (entry.mmr_change ?? 0) < 0 }]">
            {{ (entry.mmr_change ?? 0) > 0 ? '+' : '' }}{{ entry.mmr_change ?? 0 }}
          </span>
        </div>
      </div>

      <p v-else class="empty-hint">No history yet</p>
    </section>
  </div>
</template>

<style scoped>
.view { flex: 1; overflow-y: auto; }

.view-header {
  padding: 32px 32px 0;
}

.view-title { font-size: 1.5rem; font-weight: 700; letter-spacing: -0.02em; }

.section { padding: 24px 32px; }

.section-title { font-size: 0.875rem; font-weight: 600; color: #8E8E93; letter-spacing: 0.02em; text-transform: uppercase; margin-bottom: 14px; }

.chart-wrap { background: rgba(255,255,255,0.02); border-radius: 12px; border: 0.5px solid rgba(255,255,255,0.06); overflow: hidden; }
.chart-svg { width: 100%; height: 160px; display: block; }
.chart-labels { display: flex; justify-content: space-between; padding: 6px 16px 8px; }
.chart-label { font-size: 0.65rem; color: #525252; }
.chart-skeleton { height: 176px; border-radius: 12px; background: rgba(255,255,255,0.04); animation: pulse 1.5s infinite; }

.history-list { display: flex; flex-direction: column; gap: 4px; }

.history-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: 8px;
  background: rgba(255,255,255,0.02);
}

.tier-img { width: 28px; height: 28px; object-fit: contain; flex-shrink: 0; }
.tier-placeholder { width: 28px; height: 28px; border-radius: 50%; background: rgba(255,255,255,0.06); flex-shrink: 0; }

.history-info { flex: 1; }
.tier-name { font-size: 0.8rem; font-weight: 500; color: #d1d1d6; display: block; }
.tier-rr { font-size: 0.7rem; color: #636366; }

.mmr-change { font-size: 0.8rem; font-weight: 600; }
.mmr-change.positive { color: #30d158; }
.mmr-change.negative { color: #ff453a; }

.history-skeleton { height: 44px; border-radius: 8px; background: rgba(255,255,255,0.04); animation: pulse 1.5s infinite; }
.empty-hint { font-size: 0.875rem; color: #525252; }

@keyframes pulse { 0%,100%{opacity:1}50%{opacity:.5} }
@media (max-width: 767px) { .section { padding: 16px; } .view-header { padding: 16px 16px 0; } }
</style>
