import { computed, reactive } from 'vue'
import { useTransfers } from './useTransfers.js'
import { useCompression } from './useCompression.js'
import { t as tr } from './ui/i18n.js'

// One line for everything moving: how many transfers, what they are and how far along, for the
// sidebar row and the phone's floating pill.
export function transferSummary() {
  const { uploads, downloads, visiblePendingUploads } = useTransfers()
  const { jobs, activeCount } = useCompression()

  const active = computed(() => [...uploads.value, ...downloads.value].filter(t => t.status === 'active'))
  return reactive({
    count: computed(() => uploads.value.length + downloads.value.length + visiblePendingUploads.value.length + jobs.value.length),
    progress: computed(() => {
      const total = active.value.reduce((n, t) => n + (t.totalSize || 0), 0)
      const loaded = active.value.reduce((n, t) => n + (t.loaded || 0), 0)
      return total ? loaded / total : active.value.length ? 0 : 1
    }),
    label: computed(() => {
      const up = uploads.value.filter(t => t.status === 'active').length
      const down = downloads.value.filter(t => t.status === 'active').length
      if (up && !down) return up === 1 ? tr('Uploading 1 file', 'Subiendo 1 archivo') : tr(`Uploading ${up} files`, `Subiendo ${up} archivos`)
      if (down && !up) return down === 1 ? tr('Downloading 1 file', 'Descargando 1 archivo') : tr(`Downloading ${down} files`, `Descargando ${down} archivos`)
      if (up || down) return tr('Transferring', 'Transfiriendo')
      if (activeCount.value) return tr('Decompressing', 'Descomprimiendo')
      if (visiblePendingUploads.value.length) return tr('Paused uploads', 'Subidas en pausa')
      return tr('Transfers', 'Transferencias')
    }),
  })
}
