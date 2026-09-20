import { ref, computed } from 'vue'

const CHUNK_SIZE = 32 * 1024 * 1024
const CHUNK_THRESHOLD = 10 * 1024 * 1024

const transfers = ref([])
const pendingServerUploads = ref([])
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
    transfers.value.push({
      id, type, name, totalSize, loaded: 0, speed: 0,
      status: 'active', error: null,
      _t: now, _l: 0,
      uploadId: null, controller: null, path: null, location: null,
    })
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

  function setUploadMeta(id, uploadId, controller, path, location) {
    const t = transfers.value.find(t => t.id === id)
    if (!t) return
    t.uploadId = uploadId
    t.controller = controller
    t.path = path
    t.location = location
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

  async function cancelTransfer(id) {
    const t = transfers.value.find(t => t.id === id)
    if (!t) return
    t.controller?.abort()
    if (t.uploadId) {
      try { await fetch(`/api/files/cancel-upload/${t.uploadId}`, { method: 'DELETE' }) } catch {}
    }
    transfers.value = transfers.value.filter(x => x.id !== id)
    await fetchPendingUploads()
    window.dispatchEvent(new CustomEvent('tpstore:refresh-directory'))
  }

  async function cancelPendingUpload(uploadId) {
    try { await fetch(`/api/files/cancel-upload/${uploadId}`, { method: 'DELETE' }) } catch {}
    await fetchPendingUploads()
    window.dispatchEvent(new CustomEvent('tpstore:refresh-directory'))
  }

  async function fetchPendingUploads() {
    try {
      const res = await fetch('/api/files/pending-uploads')
      pendingServerUploads.value = res.ok ? await res.json() : []
    } catch {
      pendingServerUploads.value = []
    }
  }

  async function _doChunkedUpload(file, uploadId, startByte, path, location) {
    const id = add('upload', file.name, file.size)
    const controller = new AbortController()
    setUploadMeta(id, uploadId, controller, path, location)

    if (startByte > 0) update(id, startByte)

    const totalChunks = Math.max(1, Math.ceil(file.size / CHUNK_SIZE))
    const startChunk = Math.floor(startByte / CHUNK_SIZE)
    let failed = false
    let firstChunkDone = false

    for (let i = startChunk; i < totalChunks; i++) {
      const start = i * CHUNK_SIZE
      const end = Math.min(start + CHUNK_SIZE, file.size)
      const params = new URLSearchParams({
        upload_id: uploadId,
        chunk_index: i,
        total_chunks: totalChunks,
        filename: file.name,
        file_size: file.size,
        path,
        location,
      })
      try {
        const res = await fetch(`/api/files/upload-chunk?${params}`, {
          method: 'POST',
          body: file.slice(start, end),
          headers: { 'Content-Type': 'application/octet-stream' },
          signal: controller.signal,
        })
        if (!res.ok) {
          let msg = `Error ${res.status}`
          try { msg = (await res.json()).detail || msg } catch {}
          fail(id, msg)
          failed = true
          break
        }
        update(id, end)
        if (!firstChunkDone) {
          firstChunkDone = true
          window.dispatchEvent(new CustomEvent('tpstore:refresh-directory'))
        }
      } catch (e) {
        if (e.name !== 'AbortError') fail(id, e.message || 'Network error')
        failed = true
        break
      }
    }

    if (!failed) {
      complete(id)
      await fetchPendingUploads()
      window.dispatchEvent(new CustomEvent('tpstore:refresh-directory'))
    }
  }

  async function _doSimpleUpload(file, path, location) {
    const params = new URLSearchParams({ path, location })
    const form = new FormData()
    form.append('file', file)
    try {
      const res = await fetch(`/api/files/upload?${params}`, { method: 'POST', body: form })
      if (!res.ok) {
        let msg = `Error ${res.status}`
        try { msg = (await res.json()).detail || msg } catch {}
        console.error('Upload failed:', msg)
      }
    } catch (e) {
      console.error('Upload error:', e)
    }
    window.dispatchEvent(new CustomEvent('tpstore:refresh-directory'))
  }

  async function startUpload(file, path, location) {
    if (file.size <= CHUNK_THRESHOLD) {
      await _doSimpleUpload(file, path, location)
    } else {
      await _doChunkedUpload(file, crypto.randomUUID(), 0, path, location)
    }
  }

  async function resumeUpload(pending) {
    return new Promise(resolve => {
      const input = document.createElement('input')
      input.type = 'file'
      input.onchange = async () => {
        const file = input.files?.[0]
        if (!file) { resolve(); return }
        if (file.name !== pending.filename || file.size !== pending.file_size) {
          alert(`Archivo incorrecto. Se esperaba "${pending.filename}" (${formatBytes(pending.file_size)})`)
          resolve()
          return
        }
        resolve()
        await _doChunkedUpload(file, pending.upload_id, pending.bytes_received, pending.path, pending.location)
      }
      input.oncancel = () => resolve()
      input.click()
    })
  }

  const uploads = computed(() => transfers.value.filter(t => t.type === 'upload'))
  const downloads = computed(() => transfers.value.filter(t => t.type === 'download'))

  // Excludes uploads that are currently active (to avoid duplicates in the panel)
  const visiblePendingUploads = computed(() => {
    const activeIds = new Set(transfers.value.map(t => t.uploadId).filter(Boolean))
    return pendingServerUploads.value.filter(p => !activeIds.has(p.upload_id))
  })

  const hasUploads = computed(() => uploads.value.length > 0 || visiblePendingUploads.value.length > 0)
  const hasDownloads = computed(() => downloads.value.length > 0)

  function _groupEta(list) {
    const active = list.filter(t => t.status === 'active' && t.speed > 0 && t.totalSize > 0)
    if (!active.length) return null
    const secs = active.reduce((s, t) => s + (t.totalSize - t.loaded) / t.speed, 0)
    return formatEta(secs)
  }

  const uploadEta = computed(() => _groupEta(uploads.value))
  const downloadEta = computed(() => _groupEta(downloads.value))

  return {
    uploads, downloads, visiblePendingUploads,
    hasUploads, hasDownloads,
    uploadEta, downloadEta,
    add, update, setTotal, complete, fail,
    cancelTransfer, cancelPendingUpload,
    fetchPendingUploads, startUpload, resumeUpload,
  }
}
