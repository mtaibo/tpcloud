<script setup>
import { ref, onMounted } from 'vue'
import { ChevronRight, Lock } from 'lucide-vue-next'
import Spinner from '../ui/Spinner.vue'
import FileIcon from './FileIcon.vue'

const shares = ref([])
const brokenThumbs = ref(new Set())

// A share of one photo or video shows its thumbnail (unless it needs a password first);
// other single files get their document icon and the rest are folders.
function iconEntry(share) {
  return share.is_file ? { type: 'file', name: share.label } : { type: 'directory', name: share.label }
}
function thumbnailUrl(share) {
  if (!share.is_file || share.has_password || brokenThumbs.value.has(share.token)) return null
  if (!/^(image|video)\//.test(share.content_type || '')) return null
  const params = new URLSearchParams({ path: '', size: '120', v: String(share.modified ?? 0) })
  return `/share/api/share/${share.token}/files/thumbnail?${params}`
}
function thumbFailed(share) {
  brokenThumbs.value = new Set(brokenThumbs.value).add(share.token)
}
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
      <p class="subheadline secondary">Folders and files open to anyone with the link.</p>
    </header>

    <div v-if="loading" class="state"><Spinner :size="24" /></div>
    <p v-else-if="!shares.length" class="state subheadline secondary">Nothing is shared publicly yet.</p>
    <div v-else class="card">
      <a v-for="share in shares" :key="share.token" :href="`/share/${share.token}`" class="row share-row">
        <img v-if="thumbnailUrl(share)" :src="thumbnailUrl(share)" alt="" class="thumb" loading="lazy" @error="thumbFailed(share)" />
        <FileIcon v-else :entry="iconEntry(share)" :size="40" />
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
.thumb { width: 40px; height: 40px; flex-shrink: 0; object-fit: cover; border-radius: 8px; background: var(--fill-3); }
.chev { flex-shrink: 0; color: var(--label-3); }
</style>
