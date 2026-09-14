<script setup>
import { ref, onMounted } from 'vue'
import { Folder, Star } from 'lucide-vue-next'

const props = defineProps({
  user: Object,
})

const emit = defineEmits(['navigate'])

const favourites = ref([])

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
      <span class="browse-title">Favourites</span>
    </div>

    <div class="browse-list">
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
  height: calc(80px + env(safe-area-inset-top));
  padding-top: env(safe-area-inset-top);
  display: flex;
  align-items: center;
  padding-left: 1.25rem;
  padding-right: 1.25rem;
  background: rgba(0, 0, 0, 0.65);
  backdrop-filter: blur(24px) saturate(180%);
  -webkit-backdrop-filter: blur(24px) saturate(180%);
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.07);
  position: sticky;
  top: 0;
  z-index: 10;
}

.browse-title {
  font-size: 0.9rem;
  font-weight: 700;
  color: #fff;
  letter-spacing: -0.01em;
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
  cursor: pointer;
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
