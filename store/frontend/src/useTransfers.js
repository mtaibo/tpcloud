import { ref, computed } from 'vue'

const transfers = ref([])
let _id = 1

export function formatBytes(bytes) {
  if (!bytes) return '0 B'
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`
  return `${(bytes / 1073741824).toFixed(2)} GB`
}

export function formatEta(seconds) {
  if (!isFinite(seconds) || seconds < 0 || seconds > 86400 * 7) return '—'
  if (seconds < 60) return `${Math.round(seconds)}s`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${Math.round(seconds % 60)}s`
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`
}

export function useTransfers() {
  function add(type, name, totalSize = 0) {
    const id = _id++
    const now = Date.now()
    transfers.value.push({ id, type, name, totalSize, loaded: 0, speed: 0, status: 'active', error: null, _t: now, _l: 0 })
    return id
  }

  function update(id, loaded) {
    const t = transfers.value.find(t => t.id === id)
    if (!t) return
    const now = Date.now()
    const dt = (now - t._t) / 1000
    if (dt >= 0.5) {
      t.speed = (loaded - t._l) / dt
      t._t = now
      t._l = loaded
    }
    t.loaded = loaded
  }

  function setTotal(id, total) {
    const t = transfers.value.find(t => t.id === id)
    if (t) t.totalSize = total
  }

  function complete(id) {
    const t = transfers.value.find(t => t.id === id)
    if (!t) return
    if (t.totalSize > 0) t.loaded = t.totalSize
    t.status = 'done'
    setTimeout(() => { transfers.value = transfers.value.filter(x => x.id !== id) }, 4000)
  }

  function fail(id, msg) {
    const t = transfers.value.find(t => t.id === id)
    if (!t) return
    t.status = 'error'
    t.error = msg || 'Error'
    setTimeout(() => { transfers.value = transfers.value.filter(x => x.id !== id) }, 6000)
  }

  const uploads = computed(() => transfers.value.filter(t => t.type === 'upload'))
  const downloads = computed(() => transfers.value.filter(t => t.type === 'download'))
  const hasUploads = computed(() => uploads.value.length > 0)
  const hasDownloads = computed(() => downloads.value.length > 0)

  function _groupEta(list) {
    const active = list.filter(t => t.status === 'active' && t.speed > 0 && t.totalSize > 0)
    if (!active.length) return null
    const secs = active.reduce((s, t) => s + (t.totalSize - t.loaded) / t.speed, 0)
    return formatEta(secs)
  }

  const uploadEta = computed(() => _groupEta(uploads.value))
  const downloadEta = computed(() => _groupEta(downloads.value))

  return { uploads, downloads, hasUploads, hasDownloads, uploadEta, downloadEta, add, update, setTotal, complete, fail }
}
