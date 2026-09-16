<script setup>
import { ref, computed, watch } from 'vue'
import { Folder, ChevronLeft, ChevronRight } from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const props = defineProps({
  entry: Object,
  location: String,
  currentPath: String,
  mode: { type: String, default: 'move' },
})
const emit = defineEmits(['close', 'moved'])

const browsePath = ref(props.currentPath)
const dirs = ref([])
const loading = ref(false)

async function load(path) {
  loading.value = true
  try {
    const params = new URLSearchParams({ path, location: props.location })
    const res = await fetch(`/api/files/list?${params}`)
    if (!res.ok) { dirs.value = []; return }
    const data = await res.json()
    const sourceName = props.entry.name
    dirs.value = data.entries.filter(e => {
      if (e.type !== 'directory') return false
      if (props.entry.type === 'directory' && e.name === sourceName && path === props.currentPath) return false
      return true
    })
  } finally {
    loading.value = false
  }
}

watch(browsePath, load, { immediate: true })

function enter(name) {
  browsePath.value = browsePath.value ? `${browsePath.value}/${name}` : name
}

function goUp() {
  if (!browsePath.value) return
  const parts = browsePath.value.split('/')
  parts.pop()
  browsePath.value = parts.join('/')
}

const displayPath = computed(() => {
  if (!browsePath.value) return '/'
  const parts = browsePath.value.split('/')
  return parts[parts.length - 1] || '/'
})

const isSameDir = computed(() => browsePath.value === props.currentPath)
const canGoUp = computed(() => !!browsePath.value)
const actionLabel = computed(() => props.mode === 'copy' ? 'Copy Here' : 'Move Here')
const modalTitle = computed(() => props.mode === 'copy' ? 'Copy' : 'Move')

async function doAction() {
  const sourcePath = props.currentPath
    ? `${props.currentPath}/${props.entry.name}`
    : props.entry.name
  const endpoint = props.mode === 'copy' ? '/api/files/copy' : '/api/files/move'
  const res = await fetch(endpoint, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path: sourcePath, dest_dir: browsePath.value, location: props.location }),
  })
  if (res.ok) {
    emit('moved')
  } else {
    const data = await res.json().catch(() => ({}))
    alert(data.detail || `Error ${props.mode === 'copy' ? 'copying' : 'moving'} item`)
  }
}
</script>

<template>
  <BaseModal width="380px" max-height="70vh" @close="emit('close')">
    <div class="modal-header">
      <div class="header-text">
        <span class="modal-title">{{ modalTitle }}</span>
        <span class="modal-subtitle">{{ entry.name }}</span>
      </div>
    </div>

    <div class="modal-nav">
      <button class="nav-btn" :disabled="!canGoUp" @click="goUp">
        <ChevronLeft class="nav-icon" />
      </button>
      <span class="nav-label">{{ displayPath }}</span>
    </div>

    <div class="modal-list">
      <div v-if="loading" class="list-empty">Loading…</div>
      <div v-else-if="!dirs.length" class="list-empty">No folders here</div>
      <button v-for="dir in dirs" :key="dir.name" class="dir-row" @click="enter(dir.name)">
        <Folder class="dir-icon" />
        <span class="dir-name">{{ dir.name }}</span>
        <ChevronRight class="dir-arrow" />
      </button>
    </div>

    <div class="modal-footer">
      <button class="btn-cancel" @click="emit('close')">Cancel</button>
      <button class="btn-primary" :disabled="isSameDir" @click="doAction">{{ actionLabel }}</button>
    </div>
  </BaseModal>
</template>

<style scoped>
.modal-nav {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.06);
}

.nav-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.06);
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  color: #adadad;
  flex-shrink: 0;
  transition: background 0.12s, color 0.12s;
}

.nav-btn:hover:not(:disabled) { background: rgba(255, 255, 255, 0.12); color: #fff; }
.nav-btn:disabled { opacity: 0.3; cursor: default; }
.nav-icon { width: 14px; height: 14px; }

.nav-label {
  font-size: 13px;
  font-weight: 500;
  color: #d1d1d6;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.modal-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
  min-height: 120px;
  max-height: 320px;
}

.list-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 80px;
  font-size: 13px;
  color: #636366;
}

.dir-row {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 7px 16px;
  background: none;
  border: none;
  transition: background 0.08s;
}

.dir-row:hover { background: rgba(255, 255, 255, 0.05); }

.dir-icon { width: 16px; height: 16px; color: #007AFF; flex-shrink: 0; }

.dir-name {
  flex: 1;
  font-size: 13px;
  color: #d1d1d6;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dir-arrow { width: 13px; height: 13px; color: #636366; flex-shrink: 0; }
</style>
