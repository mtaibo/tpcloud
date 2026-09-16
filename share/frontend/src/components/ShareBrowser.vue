<script setup>
import { ref, computed, onMounted } from 'vue'
import { ChevronLeft, FolderDown } from 'lucide-vue-next'
import ShareFileRow from './ShareFileRow.vue'

const props = defineProps({
  token: String,
  shareInfo: Object,
  sessionToken: String,
})

const currentPath = ref('')
const entries = ref([])
const loading = ref(false)
const error = ref('')

const breadcrumbs = computed(() => currentPath.value.split('/').filter(Boolean))

const rootLabel = computed(() => props.shareInfo?.label || props.token)

function authHeaders() {
  return props.sessionToken ? { 'X-Share-Session': props.sessionToken } : {}
}

async function loadDir(path = '') {
  loading.value = true
  error.value = ''
  try {
    const params = new URLSearchParams({ path })
    const res = await fetch(`/api/share/${props.token}/files/list?${params}`, {
      headers: authHeaders(),
    })
    if (!res.ok) {
      error.value = 'Failed to load files'
      return
    }
    const data = await res.json()
    entries.value = data.entries
    currentPath.value = path
  } catch {
    error.value = 'Connection error'
  } finally {
    loading.value = false
  }
}

function navigateToCrumb(index) {
  const parts = breadcrumbs.value.slice(0, index + 1)
  loadDir(parts.join('/'))
}

function navigateUp() {
  const parts = breadcrumbs.value.slice(0, -1)
  loadDir(parts.join('/'))
}

function onOpen(entry) {
  const newPath = currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name
  loadDir(newPath)
}

async function downloadEntry(entry) {
  const path = currentPath.value ? `${currentPath.value}/${entry.name}` : entry.name
  const url = `/api/share/${props.token}/files/download?path=${encodeURIComponent(path)}`
  const res = await fetch(url, { headers: authHeaders() })
  const blob = await res.blob()
  const a = document.createElement('a')
  a.href = URL.createObjectURL(blob)
  a.download = entry.name
  a.click()
  URL.revokeObjectURL(a.href)
}

async function downloadZip() {
  const path = currentPath.value
  const url = `/api/share/${props.token}/files/download-zip?path=${encodeURIComponent(path)}`
  const res = await fetch(url, { headers: authHeaders() })
  const blob = await res.blob()
  const dirName = breadcrumbs.value.at(-1) || rootLabel.value
  const a = document.createElement('a')
  a.href = URL.createObjectURL(blob)
  a.download = `${dirName}.zip`
  a.click()
  URL.revokeObjectURL(a.href)
}

onMounted(() => loadDir(''))
</script>

<template>
  <div class="browser">
    <div class="topbar">
      <div class="breadcrumbs">
        <button
          v-if="breadcrumbs.length"
          class="back-btn"
          @click="navigateUp"
        >
          <ChevronLeft class="back-icon" />
        </button>

        <span
          class="crumb root-crumb"
          :class="{ active: !breadcrumbs.length }"
          @click="loadDir('')"
        >{{ rootLabel }}</span>

        <template v-for="(seg, i) in breadcrumbs" :key="i">
          <span class="crumb-sep">/</span>
          <span
            class="crumb"
            :class="{ active: i === breadcrumbs.length - 1 }"
            @click="navigateToCrumb(i)"
          >{{ seg }}</span>
        </template>
      </div>

      <button class="zip-btn" title="Download as ZIP" @click="downloadZip">
        <FolderDown class="zip-icon" />
      </button>
    </div>

    <div v-if="loading" class="state-msg">Loading…</div>
    <div v-else-if="error" class="state-msg err">{{ error }}</div>
    <div v-else-if="!entries.length" class="state-msg">Empty folder</div>
    <table v-else class="file-table">
      <tbody>
        <ShareFileRow
          v-for="entry in entries"
          :key="entry.name"
          :entry="entry"
          @open="onOpen"
          @download="downloadEntry"
        />
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.browser {
  display: flex;
  flex-direction: column;
  min-height: 100dvh;
}

.topbar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 10px 1.25rem;
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.08);
  position: sticky;
  top: 0;
  background: rgba(0, 0, 0, 0.85);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  z-index: 10;
}

.breadcrumbs {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.back-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.06);
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  color: #fff;
  flex-shrink: 0;
}
.back-btn:hover { background: rgba(255, 255, 255, 0.1); }
.back-icon { width: 16px; height: 16px; stroke-width: 2.5; }

.crumb {
  font-size: 0.95rem;
  font-weight: 500;
  color: #636366;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 180px;
}
.crumb:not(.active) { color: #636366; }
.crumb.active { color: #fff; }
.root-crumb:not(.active) { color: #636366; }
.crumb:not(.active):hover { color: #ababab; }

.crumb-sep {
  color: #3a3a3c;
  font-size: 0.85rem;
  flex-shrink: 0;
}

.zip-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.06);
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  color: #636366;
  flex-shrink: 0;
  transition: background 0.15s, color 0.15s;
}
.zip-btn:hover { background: rgba(255, 255, 255, 0.1); color: #fff; }
.zip-icon { width: 16px; height: 16px; }

.state-msg {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.875rem;
  color: #636366;
  padding: 4rem;
}
.err { color: #ff453a; }

.file-table {
  width: 100%;
  border-collapse: collapse;
  table-layout: fixed;
}
</style>
