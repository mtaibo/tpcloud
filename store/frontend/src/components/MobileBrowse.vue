<script setup>
import { ref, onMounted } from 'vue'
import { Folder, Star, ChevronLeft, ChevronRight } from 'lucide-vue-next'

const props = defineProps({
  user: Object,
  canGoBack: Boolean,
  canGoForward: Boolean,
})

const emit = defineEmits(['navigate', 'go-back', 'go-forward'])

const favourites = ref([])
const favouritesOpen = ref(true)

onMounted(() => {
  try {
    const stored = localStorage.getItem('tpcloud-favourites')
    if (stored) favourites.value = JSON.parse(stored)
  } catch {}
})

function go(fav) {
  emit('navigate', fav.location, fav.path)
}
</script>

<template>
  <div class="browse-panel">
    <div class="browse-header">
      <div class="nav-pill">
        <button class="nav-btn" :disabled="!canGoBack" @click="emit('go-back')">
          <ChevronLeft class="nav-icon" />
        </button>
        <div class="nav-divider" />
        <button class="nav-btn" :disabled="!canGoForward" @click="emit('go-forward')">
          <ChevronRight class="nav-icon" />
        </button>
      </div>
      <span class="browse-large-title">Browse</span>
    </div>

    <div class="browse-list">
      <div class="section-row" @click="favouritesOpen = !favouritesOpen">
        <span class="section-label">Favourites</span>
        <ChevronRight class="section-chevron" :class="{ open: favouritesOpen }" />
      </div>

      <template v-if="favouritesOpen">
        <p v-if="favourites.length === 0" class="empty-hint">
          No favourites yet.<br>Add them from desktop.
        </p>
        <button
          v-for="fav in favourites"
          :key="fav.id"
          class="fav-row"
          @click="go(fav)"
        >
          <span class="fav-icon-wrap">
            <Folder class="fav-folder-icon" />
            <Star class="fav-badge" />
          </span>
          <span class="fav-label">{{ fav.label }}</span>
        </button>
      </template>
    </div>
  </div>
</template>

<style scoped>
.browse-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: #000;
  overflow: hidden;
}

.browse-header {
  flex-shrink: 0;
  padding-top: calc(env(safe-area-inset-top) + 14px);
  padding-bottom: 14px;
  padding-left: 1.25rem;
  padding-right: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 10px;
  background: rgba(0, 0, 0, 0.65);
  backdrop-filter: blur(24px) saturate(180%);
  -webkit-backdrop-filter: blur(24px) saturate(180%);
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.07);
  position: sticky;
  top: 0;
  z-index: 10;
}

.nav-pill {
  display: flex;
  align-items: center;
  align-self: flex-start;
  background: rgba(255, 255, 255, 0.07);
  border: 0.5px solid rgba(255, 255, 255, 0.12);
  border-radius: 999px;
  overflow: hidden;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25), inset 0 0.5px 0 rgba(255, 255, 255, 0.08);
}

.nav-btn {
  width: 52px;
  height: 46px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.85);
  transition: background 0.12s, color 0.12s;
}

.nav-btn:disabled { color: rgba(255, 255, 255, 0.2); cursor: default; }
.nav-btn:not(:disabled):active { background: rgba(255, 255, 255, 0.04); }

.nav-divider {
  width: 0.5px;
  height: 14px;
  background: rgba(255, 255, 255, 0.15);
  flex-shrink: 0;
}

.nav-icon {
  width: 22px;
  height: 22px;
  stroke-width: 2.5;
}

.browse-large-title {
  font-size: 2rem;
  font-weight: 700;
  color: #fff;
  letter-spacing: -0.02em;
  line-height: 1;
}

.section-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 1.25rem 0.4rem;
  cursor: default;
}

.section-label {
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #636366;
  user-select: none;
}

.section-chevron {
  width: 14px;
  height: 14px;
  color: #636366;
  transition: transform 0.2s;
}

.section-chevron.open {
  transform: rotate(90deg);
}

.browse-list {
  flex: 1;
  overflow-y: auto;
  padding: 0.5rem 0;
  padding-bottom: 100px;
}

.empty-hint {
  padding: 2rem 1.25rem;
  font-size: 0.875rem;
  color: #525252;
  line-height: 1.6;
}

.fav-row {
  display: flex;
  align-items: center;
  gap: 0.875rem;
  width: 100%;
  padding: 0.75rem 1.25rem;
  background: none;
  border: none;
  text-align: left;
  transition: background 0.12s;
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.05);
}

.fav-row:hover {
  background: rgba(255, 255, 255, 0.04);
}

.fav-icon-wrap {
  position: relative;
  width: 22px;
  height: 22px;
  flex-shrink: 0;
}

.fav-folder-icon {
  width: 22px;
  height: 22px;
  color: #007AFF;
}

.fav-badge {
  position: absolute;
  bottom: -2px;
  right: -4px;
  width: 11px;
  height: 11px;
  color: #FFD60A;
  fill: #FFD60A;
  stroke-width: 2;
}

.fav-label {
  font-size: 0.9375rem;
  font-weight: 500;
  color: #d1d1d6;
}
</style>
