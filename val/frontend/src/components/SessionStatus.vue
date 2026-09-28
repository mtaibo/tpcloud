<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'

const emit = defineEmits(['reconnect'])

const session = ref(null)
let timer = null

async function fetchSession() {
  try {
    const r = await fetch('/api/val/account/session')
    if (r.ok) session.value = await r.json()
  } catch { /* ignore */ }
}

const state = computed(() => {
  if (!session.value?.has_extension_synced) return 'idle'
  if (session.value.needs_resync) return 'err'
  if ((session.value.days_remaining ?? 0) <= 3) return 'warn'
  return 'ok'
})

const label = computed(() => {
  if (!session.value?.has_extension_synced) return 'Sin sync'
  if (session.value.needs_resync) return 'Resync necesario'
  const d = session.value.days_remaining ?? 0
  return `${d}d restantes`
})

onMounted(() => {
  fetchSession()
  timer = setInterval(fetchSession, 60_000)
})
onUnmounted(() => { if (timer) clearInterval(timer) })

defineExpose({ refresh: fetchSession })
</script>

<template>
  <button v-if="session" class="chip" :class="state" @click="emit('reconnect')" :title="label">
    <span class="dot"></span>
    <span class="txt">Riot · {{ label }}</span>
  </button>
</template>

<style scoped>
.chip {
  display: flex; align-items: center; gap: 6px;
  padding: 4px 10px; border-radius: 999px;
  background: rgba(255,255,255,0.05);
  border: 0.5px solid rgba(255,255,255,0.08);
  color: #8E8E93; font-size: 0.7rem; font-weight: 500;
  cursor: pointer; transition: background 0.15s, color 0.15s;
  width: 100%;
  justify-content: flex-start;
}
.chip:hover { background: rgba(255,255,255,0.09); color: #d1d1d6; }
.chip.ok { color: #30d158; }
.chip.warn { color: #ffd60a; }
.chip.err { color: #ff453a; }
.chip.idle { color: #636366; }
.dot { width: 6px; height: 6px; border-radius: 50%; background: currentColor; flex-shrink: 0; }
.txt { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
