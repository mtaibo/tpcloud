import { ref, computed } from 'vue'
import { formatBytes, formatEta } from './useTransfers.js'

export { formatBytes, formatEta }

const jobs = ref([])
const _dismissed = new Set()
const _speedTracker = new Map() // job_id -> { bytes, time }
let _pollTimer = null

export function useCompression() {
  async function fetchJobs() {
    try {
      const res = await fetch('/api/compression/jobs')
      if (!res.ok) return
      const raw = await res.json()
      const now = Date.now()

      const updated = raw
        .filter(j => !_dismissed.has(j.job_id))
        .map(j => {
          const prev = _speedTracker.get(j.job_id)
          let speed = 0
          if (prev && j.status === 'active') {
            const dt = (now - prev.time) / 1000
            if (dt > 0) speed = Math.max(0, (j.extracted_bytes - prev.bytes) / dt)
          }
          _speedTracker.set(j.job_id, { bytes: j.extracted_bytes, time: now })
          return { ...j, speed }
        })

      for (const j of updated) {
        if ((j.status === 'done' || j.status === 'error') && !_dismissed.has(j.job_id)) {
          const delay = j.status === 'done' ? 4000 : 8000
          setTimeout(() => {
            _dismissed.add(j.job_id)
            _speedTracker.delete(j.job_id)
            jobs.value = jobs.value.filter(x => x.job_id !== j.job_id)
            fetch(`/api/compression/jobs/${j.job_id}`, { method: 'DELETE' }).catch(() => {})
          }, delay)
        }
      }

      jobs.value = updated
    } catch {}
  }

  function _startPolling() {
    if (_pollTimer) return
    _pollTimer = setInterval(async () => {
      await fetchJobs()
      if (!jobs.value.some(j => j.status === 'active')) _stopPolling()
    }, 2000)
  }

  function _stopPolling() {
    clearInterval(_pollTimer)
    _pollTimer = null
  }

  async function init() {
    await fetchJobs()
    if (jobs.value.some(j => j.status === 'active')) _startPolling()
  }

  async function startDecompress(filename, path, location) {
    const params = new URLSearchParams({ filename, path, location })
    const res = await fetch(`/api/compression/jobs?${params}`, { method: 'POST' })
    if (res.ok) {
      await fetchJobs()
      _startPolling()
    }
    return res.ok
  }

  async function cancelJob(jobId) {
    _dismissed.add(jobId)
    _speedTracker.delete(jobId)
    jobs.value = jobs.value.filter(j => j.job_id !== jobId)
    if (!jobs.value.some(j => j.status === 'active')) _stopPolling()
    await fetch(`/api/compression/jobs/${jobId}`, { method: 'DELETE' }).catch(() => {})
  }

  const hasJobs = computed(() => jobs.value.length > 0)
  const activeCount = computed(() => jobs.value.filter(j => j.status === 'active').length)

  return { jobs, hasJobs, activeCount, init, fetchJobs, startDecompress, cancelJob }
}
