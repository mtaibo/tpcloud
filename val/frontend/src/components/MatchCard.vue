<script setup>
import AgentPortrait from './AgentPortrait.vue'

const props = defineProps({
  match: Object,
})

function formatDuration(ms) {
  if (!ms) return ''
  const min = Math.floor(ms / 60000)
  return `${min}m`
}

function timeAgo(dateStr) {
  if (!dateStr) return ''
  const diff = Date.now() - new Date(dateStr).getTime()
  const h = Math.floor(diff / 3600000)
  if (h < 1) return 'Just now'
  if (h < 24) return `${h}h ago`
  return `${Math.floor(h / 24)}d ago`
}

const stats = props.match?.stats ?? {}
const kda = stats.kills !== undefined
  ? `${stats.kills}/${stats.deaths}/${stats.assists}`
  : '-/-/-'
</script>

<template>
  <div :class="['match-card', match?.won ? 'won' : 'lost']">
    <div class="match-result-bar" />

    <AgentPortrait :agent-id="match?.agent_id" :agent-name="match?.agent" class="match-agent" />

    <div class="match-info">
      <div class="match-top">
        <span class="match-map">{{ match?.map ?? 'Unknown' }}</span>
        <span class="match-queue">{{ match?.queue }}</span>
      </div>
      <div class="match-bottom">
        <span class="match-score">{{ match?.score }}</span>
        <span class="match-time">{{ timeAgo(match?.started_at) }}</span>
      </div>
    </div>

    <div class="match-stats">
      <span class="kda">{{ kda }}</span>
      <span v-if="match?.game_length_ms" class="duration">{{ formatDuration(match.game_length_ms) }}</span>
    </div>
  </div>
</template>

<style scoped>
.match-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.03);
  border: 0.5px solid rgba(255, 255, 255, 0.06);
  position: relative;
  overflow: hidden;
}

.match-result-bar {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
}

.won .match-result-bar { background: #30d158; }
.lost .match-result-bar { background: #ff453a; }

.match-agent { width: 36px; height: 36px; flex-shrink: 0; }

.match-info { flex: 1; min-width: 0; }

.match-top {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 2px;
}

.match-map { font-size: 0.875rem; font-weight: 600; color: #fff; }
.match-queue { font-size: 0.7rem; color: #636366; background: rgba(255,255,255,0.06); padding: 1px 5px; border-radius: 4px; }

.match-bottom { display: flex; align-items: center; gap: 6px; }
.match-score { font-size: 0.75rem; color: #8E8E93; }
.match-time { font-size: 0.7rem; color: #525252; }

.match-stats {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
  flex-shrink: 0;
}

.kda { font-size: 0.8rem; font-weight: 600; color: #d1d1d6; font-variant-numeric: tabular-nums; }
.duration { font-size: 0.7rem; color: #636366; }
</style>
