<script setup>
import { ref } from 'vue'
import { User } from 'lucide-vue-next'

const props = defineProps({
  agentId: String,
  agentName: String,
  agentImg: String,
  class: String,
})

const imgError = ref(false)
const src = props.agentImg
  || (props.agentId ? `https://media.valorant-api.com/agents/${props.agentId}/displayiconsmall.png` : null)
</script>

<template>
  <div class="agent-wrap">
    <img
      v-if="src && !imgError"
      :src="src"
      :alt="agentName"
      class="agent-img"
      loading="lazy"
      @error="imgError = true"
    />
    <div v-else class="agent-fallback">
      <User :size="14" color="#636366" />
    </div>
  </div>
</template>

<style scoped>
.agent-wrap {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.06);
  flex-shrink: 0;
}

.agent-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.agent-fallback {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
