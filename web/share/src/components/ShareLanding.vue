<script setup>
import { ref, onMounted } from 'vue'
import { ChevronRight, Lock } from 'lucide-vue-next'
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
      <h1 class="large-title">Shared</h1>
      <p class="subheadline secondary">Folders open to anyone with the link.</p>
    </header>

    <div v-if="loading" class="state"><Spinner :size="24" /></div>
    <p v-else-if="!shares.length" class="state subheadline secondary">Nothing is shared publicly yet.</p>
    <div v-else class="card">
      <a v-for="share in shares" :key="share.token" :href="`/share/${share.token}`" class="row share-row">
        <FileIcon :entry="{ type: 'directory', name: share.label }" :size="40" />
        <span class="row-body">
          <span class="row-title">{{ share.label }}</span>
          <span v-if="share.has_password" class="row-subtitle small lock"><Lock :size="11" :stroke-width="2.4" /> Password protected</span>
        </span>
        <ChevronRight :size="18" class="chev" />
      </a>
    </div>
  </main>
</template>

<style scoped>
.landing { max-width: 900px; margin: 0 auto; padding: calc(48px + env(safe-area-inset-top)) 20px 48px; }
.head { display: flex; flex-direction: column; align-items: center; gap: 8px; text-align: center; margin-bottom: 34px; }
.state { text-align: center; padding: 40px; display: grid; place-items: center; }
.card { max-width: 680px; margin: 0 auto; }
.share-row { --inset: 74px; padding-top: 10px; padding-bottom: 10px; color: inherit; }
.lock { display: inline-flex; align-items: center; gap: 4px; }
.chev { flex-shrink: 0; color: var(--label-3); }
</style>
