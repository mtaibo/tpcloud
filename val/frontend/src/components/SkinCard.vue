<script setup>
import { ref } from 'vue'
import { Sword, Heart } from 'lucide-vue-next'

const props = defineProps({
  skin: Object,
})

const imgError = ref(false)
const saved = ref(false)
const saving = ref(false)

async function toggleWishlist() {
  if (!props.skin?.uuid) return
  saving.value = true
  try {
    const r = await fetch('/api/val/wishlist', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ skin_uuid: props.skin.uuid }),
    })
    if (r.ok || r.status === 409) saved.value = true
  } finally { saving.value = false }
}
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
      <div class="skin-text">
        <span class="skin-name">{{ skin?.name ?? 'Unknown Skin' }}</span>
        <span v-if="skin?.weapon_type" class="skin-weapon">{{ skin.weapon_type }}</span>
      </div>
      <button class="wishlist-btn" :class="{ saved }" :disabled="saving" @click="toggleWishlist" title="Add to wishlist">
        <Heart :size="13" :fill="saved ? '#FF4655' : 'none'" />
      </button>
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
  align-items: center;
  gap: 6px;
}
.skin-text {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.wishlist-btn {
  padding: 4px;
  background: none;
  border: none;
  color: #525252;
  cursor: pointer;
  border-radius: 4px;
  transition: color 0.15s, background 0.15s;
  opacity: 0;
}
.skin-card:hover .wishlist-btn, .wishlist-btn.saved { opacity: 1; }
.wishlist-btn:hover { color: #FF4655; background: rgba(255,255,255,0.05); }
.wishlist-btn.saved { color: #FF4655; }

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
