<script setup>
import { computed } from 'vue'
import { Compass, House, Users } from 'lucide-vue-next'
import { homePath, navigate, store } from '../store.js'
import { transferSummary } from '../transferSummary.js'

// The phone's floating tab bar (iOS 26), and the pill above it while something is uploading.
const emit = defineEmits(['transfers'])
const summary = transferSummary()

const tabs = [
  { id: 'home', label: 'Inicio', icon: House },
  { id: 'shared', label: 'Compartido', icon: Users },
  { id: 'browse', label: 'Explorar', icon: Compass },
]

const active = computed(() => {
  if (store.panel === 'browse' || store.panel === 'shares') return 'browse'
  if (store.path === 'shared' || store.path.startsWith('shared/')) return 'shared'
  return 'home'
})

function pick(id) {
  if (id === 'browse') store.panel = 'browse'
  else if (id === 'shared') navigate('external', 'shared')
  else navigate('external', homePath.value)
}
</script>

<template>
  <div class="dock">
    <Transition name="pill">
      <button v-if="summary.count" type="button" class="pill glass" @click="emit('transfers')">
        <svg class="ring" viewBox="0 0 20 20" aria-hidden="true">
          <circle cx="10" cy="10" r="8" class="track" />
          <circle cx="10" cy="10" r="8" class="fill" :style="{ strokeDashoffset: 50.3 * (1 - summary.progress) }" />
        </svg>
        {{ summary.label }}
      </button>
    </Transition>
    <nav class="tabbar glass">
      <button v-for="t in tabs" :key="t.id" type="button" class="tab" :class="{ on: active === t.id }" @click="pick(t.id)">
        <component :is="t.icon" :size="24" :stroke-width="active === t.id ? 2.2 : 1.8" />
        <span>{{ t.label }}</span>
      </button>
    </nav>
  </div>
</template>

<style scoped>
.dock {
  position: fixed;
  left: 0;
  right: 0;
  bottom: calc(10px + env(safe-area-inset-bottom));
  z-index: 50;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  pointer-events: none;
}
@media (min-width: 768px) { .dock { display: none; } }
.tabbar, .pill { pointer-events: auto; }
.tabbar { display: flex; padding: 4px; border-radius: 999px; }
.tab {
  width: 92px;
  height: 56px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  border-radius: 999px;
  font-size: 10px;
  font-weight: 600;
  color: var(--label);
  transition: background 0.25s, color 0.25s;
}
.tab.on { color: var(--accent); background: var(--fill-3); }
.tab:active { transform: scale(0.96); }
.pill { display: flex; align-items: center; gap: 8px; height: 40px; padding: 0 16px 0 12px; border-radius: 999px; font-size: 14px; font-weight: 600; color: var(--label); }
.ring { width: 18px; height: 18px; transform: rotate(-90deg); }
.track { fill: none; stroke: var(--fill); stroke-width: 2.6; }
.fill { fill: none; stroke: var(--accent); stroke-width: 2.6; stroke-linecap: round; stroke-dasharray: 50.3; transition: stroke-dashoffset 0.4s; }
.pill-enter-active { transition: transform 0.4s var(--spring), opacity 0.2s; }
.pill-leave-active { transition: transform 0.25s, opacity 0.2s; }
.pill-enter-from, .pill-leave-to { transform: translateY(12px) scale(0.9); opacity: 0; }
</style>
