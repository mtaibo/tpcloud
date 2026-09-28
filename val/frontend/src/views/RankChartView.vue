<script setup>
import { ref, computed, onMounted } from 'vue'

defineProps({ player: Object, linkedAccount: Object, isOwnAccount: Boolean, user: Object })

const history = ref([])
const loading = ref(true)

async function load() {
  try {
    const r = await fetch('/api/val/player/mmr-history')
    if (r.ok) history.value = await r.json()
  } finally { loading.value = false }
}

const chart = computed(() => {
  if (!history.value.length) return null
  const points = [...history.value].reverse()
  const min = Math.min(...points.map(p => p.mmr ?? 0)) - 100
  const max = Math.max(...points.map(p => p.mmr ?? 0)) + 100
  const w = 800, h = 300, pad = 24
  const stepX = (w - pad * 2) / Math.max(points.length - 1, 1)
  const scale = (v) => h - pad - ((v - min) / Math.max(max - min, 1)) * (h - pad * 2)
  const d = points.map((p, i) => `${i === 0 ? 'M' : 'L'} ${pad + i * stepX} ${scale(p.mmr ?? 0)}`).join(' ')
  return { d, w, h, pad, points, scale, stepX, min, max }
})

const currentMmr = computed(() => history.value[0]?.mmr ?? 0)
const change24h = computed(() => {
  if (history.value.length < 2) return 0
  return (history.value[0]?.mmr ?? 0) - (history.value.slice(0, 20).at(-1)?.mmr ?? 0)
})

onMounted(load)
</script>

<template>
  <div class="view">
    <div class="view-header">
      <h1 class="view-title">Rank Progression</h1>
      <div v-if="history.length" class="stats">
        <div class="stat">
          <span class="stat-label">MMR actual</span>
          <span class="stat-val">{{ currentMmr.toLocaleString() }}</span>
        </div>
        <div class="stat">
          <span class="stat-label">Últimas 20</span>
          <span class="stat-val" :class="{ pos: change24h > 0, neg: change24h < 0 }">
            {{ change24h >= 0 ? '+' : '' }}{{ change24h }}
          </span>
        </div>
      </div>
    </div>

    <p v-if="!loading && !history.length" class="empty">No hay histórico todavía. Juega partidas competitivas y volveremos.</p>

    <div v-if="chart" class="chart-wrap">
      <svg :viewBox="`0 0 ${chart.w} ${chart.h}`" class="chart">
        <defs>
          <linearGradient id="lineGrad" x1="0" x2="0" y1="0" y2="1">
            <stop offset="0%" stop-color="#FF4655" stop-opacity="0.4" />
            <stop offset="100%" stop-color="#FF4655" stop-opacity="0" />
          </linearGradient>
        </defs>
        <path
          :d="`${chart.d} L ${chart.pad + (chart.points.length - 1) * chart.stepX} ${chart.h - chart.pad} L ${chart.pad} ${chart.h - chart.pad} Z`"
          fill="url(#lineGrad)"
        />
        <path :d="chart.d" stroke="#FF4655" stroke-width="2" fill="none" />
        <circle
          v-for="(p, i) in chart.points"
          :key="i"
          :cx="chart.pad + i * chart.stepX"
          :cy="chart.scale(p.mmr ?? 0)"
          r="3"
          :fill="(p.mmr_change ?? 0) >= 0 ? '#30d158' : '#ff453a'"
        />
      </svg>
    </div>

    <section v-if="history.length" class="recent">
      <h2 class="section-title">Últimas partidas</h2>
      <div class="rows">
        <div v-for="m in history.slice(0, 10)" :key="m.match_id" class="row">
          <img v-if="m.images?.small" :src="m.images.small" class="tier-img" />
          <span class="tier-name">{{ m.tier }}</span>
          <span class="rr">{{ m.rr }} RR</span>
          <span class="change" :class="{ pos: m.mmr_change > 0, neg: m.mmr_change < 0 }">
            {{ m.mmr_change > 0 ? '+' : '' }}{{ m.mmr_change }}
          </span>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.view { flex: 1; overflow-y: auto; }
.view-header { display: flex; align-items: center; justify-content: space-between; padding: 32px 32px 24px; }
.view-title { font-size: 1.5rem; font-weight: 700; letter-spacing: -0.02em; }

.stats { display: flex; gap: 12px; }
.stat { display: flex; flex-direction: column; align-items: flex-end; gap: 2px; padding: 8px 14px; background: rgba(255,255,255,0.04); border-radius: 9px; }
.stat-label { font-size: 0.65rem; color: #636366; text-transform: uppercase; letter-spacing: 0.05em; }
.stat-val { font-size: 1rem; font-weight: 700; color: #d1d1d6; font-variant-numeric: tabular-nums; }
.stat-val.pos { color: #30d158; }
.stat-val.neg { color: #ff453a; }

.chart-wrap { padding: 0 32px 24px; }
.chart { width: 100%; height: auto; }

.section-title { font-size: 0.85rem; font-weight: 600; color: #8E8E93; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px; }
.recent { padding: 0 32px 32px; }
.rows { display: flex; flex-direction: column; gap: 6px; }
.row { display: flex; align-items: center; gap: 12px; padding: 8px 14px; background: rgba(255,255,255,0.03); border-radius: 8px; }
.tier-img { width: 22px; height: 22px; }
.tier-name { flex: 1; font-size: 0.82rem; color: #d1d1d6; font-weight: 500; }
.rr { font-size: 0.72rem; color: #636366; font-variant-numeric: tabular-nums; }
.change { font-size: 0.78rem; font-weight: 700; min-width: 46px; text-align: right; font-variant-numeric: tabular-nums; color: #636366; }
.change.pos { color: #30d158; }
.change.neg { color: #ff453a; }

.empty { padding: 60px 32px; text-align: center; color: #636366; font-size: 0.875rem; }
</style>
