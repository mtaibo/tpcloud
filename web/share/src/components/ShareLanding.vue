<script setup>
import { ref, onMounted } from 'vue'
import { Lock } from 'lucide-vue-next'
import Spinner from '../ui/Spinner.vue'
import FileIcon from './FileIcon.vue'

const shares = ref([])
const loading = ref(true)

onMounted(async () => {
  try {
    const res = await fetch('/share/api/share')
    if (res.ok) shares.value = await res.json()
  } finally {
    loading.value = false
  }
})

</script>

<template>
  <main class="landing">
    <header class="head">
      <FileIcon :entry="{ type: 'share-link', name: '' }" :size="84" />
      <h1 class="large-title">Compartido</h1>
      <p class="subheadline secondary">Carpetas abiertas a cualquiera con el enlace.</p>
    </header>

    <div v-if="loading" class="state"><Spinner :size="24" /></div>
    <p v-else-if="!shares.length" class="state subheadline secondary">Aún no hay nada compartido públicamente.</p>
    <div v-else class="grid">
      <a v-for="share in shares" :key="share.token" :href="`/share/${share.token}`" class="tile">
        <FileIcon :entry="{ type: 'directory', name: share.label }" :size="72" />
        <span class="name">{{ share.label }}</span>
        <span v-if="share.has_password" class="badge"><Lock :size="10" :stroke-width="2.6" /> Con contraseña</span>
      </a>
    </div>
  </main>
</template>

<style scoped>
.landing { max-width: 900px; margin: 0 auto; padding: calc(48px + env(safe-area-inset-top)) 20px 48px; }
.head { display: flex; flex-direction: column; align-items: center; gap: 8px; text-align: center; margin-bottom: 34px; }
.state { text-align: center; padding: 40px; display: grid; place-items: center; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 12px; }
.tile { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 20px 10px 16px; border-radius: var(--radius-card); background: var(--cell); text-align: center; transition: transform 0.25s var(--spring); }
.tile:active { transform: scale(0.96); }
@media (hover: hover) { .tile:hover { transform: translateY(-2px); } }
.name { font-size: 15px; font-weight: 600; overflow-wrap: anywhere; }
</style>
