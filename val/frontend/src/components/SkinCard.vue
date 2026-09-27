<script setup>
import { ref } from 'vue'
import { Sword } from 'lucide-vue-next'

defineProps({
  skin: Object,
})

const imgError = ref(false)
</script>

<template>
  <div class="skin-card">
    <div class="skin-art">
      <img
        v-if="skin?.display_icon && !imgError"
        :src="skin.display_icon"
        :alt="skin.name"
        class="skin-img"
        loading="lazy"
        @error="imgError = true"
      />
      <Sword v-else :size="28" color="#3a3a3c" />
    </div>
    <div class="skin-info">
      <span class="skin-name">{{ skin?.name ?? 'Unknown Skin' }}</span>
      <span v-if="skin?.weapon_type" class="skin-weapon">{{ skin.weapon_type }}</span>
    </div>
  </div>
</template>

<style scoped>
.skin-card {
  display: flex;
  flex-direction: column;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.03);
  border: 0.5px solid rgba(255, 255, 255, 0.06);
  overflow: hidden;
  transition: border-color 0.15s;
}

.skin-card:hover { border-color: rgba(255, 255, 255, 0.12); }

.skin-art {
  aspect-ratio: 16/7;
  background: rgba(255, 255, 255, 0.03);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 12px;
}

.skin-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.skin-info {
  padding: 8px 10px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.skin-name {
  font-size: 0.75rem;
  font-weight: 500;
  color: #d1d1d6;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.skin-weapon {
  font-size: 0.65rem;
  color: #636366;
}
</style>
