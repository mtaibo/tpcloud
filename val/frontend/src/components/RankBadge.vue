<script setup>
defineProps({
  tier: String,
  tierId: Number,
  rr: Number,
  mmrChange: Number,
  images: Object,
  size: { type: String, default: 'md' },
})
</script>

<template>
  <div :class="['rank-badge', `rank-badge--${size}`]">
    <img
      v-if="images?.small"
      :src="images.small"
      :alt="tier"
      class="rank-img"
      loading="lazy"
    />
    <div v-else class="rank-placeholder" />

    <div class="rank-info">
      <span class="rank-name">{{ tier ?? 'Unrated' }}</span>
      <span v-if="rr !== undefined" class="rank-rr">{{ rr }} RR</span>
    </div>

    <span
      v-if="mmrChange !== undefined && mmrChange !== 0"
      :class="['rr-delta', mmrChange > 0 ? 'positive' : 'negative']"
    >{{ mmrChange > 0 ? '+' : '' }}{{ mmrChange }}</span>
  </div>
</template>

<style scoped>
.rank-badge {
  display: flex;
  align-items: center;
  gap: 10px;
}

.rank-badge--lg .rank-img { width: 56px; height: 56px; }
.rank-badge--md .rank-img { width: 40px; height: 40px; }
.rank-badge--sm .rank-img { width: 28px; height: 28px; }

.rank-img { object-fit: contain; }

.rank-placeholder {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.06);
}

.rank-info { display: flex; flex-direction: column; gap: 2px; }

.rank-name { font-size: 0.875rem; font-weight: 600; color: #fff; }
.rank-rr { font-size: 0.75rem; color: #8E8E93; }

.rr-delta {
  font-size: 0.75rem;
  font-weight: 600;
  padding: 2px 6px;
  border-radius: 4px;
  margin-left: auto;
}

.rr-delta.positive { color: #30d158; background: rgba(48, 209, 88, 0.12); }
.rr-delta.negative { color: #ff453a; background: rgba(255, 69, 58, 0.12); }
</style>
