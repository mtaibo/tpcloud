<script setup>
import { watch } from 'vue'
import { Archive, ArrowDown, ArrowUp, Check, Play, Square, X } from 'lucide-vue-next'
import { useTransfers, formatBytes, formatEta } from '../useTransfers.js'
import { useCompression } from '../useCompression.js'
import { t as tr } from '../ui/i18n.js'

// Every upload, download and extraction with its progress. A popover by the sidebar, or inside
// a sheet on the phone (`inline`).
const props = defineProps({ show: Boolean, inline: Boolean })

const { uploads, downloads, visiblePendingUploads, cancelTransfer, resumeUpload, fetchPendingUploads } = useTransfers()
const { jobs, cancelJob, fetchJobs } = useCompression()

watch(() => props.show || props.inline, on => { if (on) { fetchPendingUploads(); fetchJobs() } }, { immediate: true })

const pct = (a, b) => (b > 0 ? Math.min(100, (a / b) * 100) : null)

function meta(loaded, total, speed, status) {
  if (status !== 'active') return ''
  const parts = [total ? tr(`${formatBytes(loaded)} of ${formatBytes(total)}`, `${formatBytes(loaded)} de ${formatBytes(total)}`) : formatBytes(loaded)]
  if (speed > 0) parts.push(`${formatBytes(speed)}/s`)
  if (speed > 0 && total) parts.push(tr(`${formatEta((total - loaded) / speed)} left`, `quedan ${formatEta((total - loaded) / speed)}`))
  return parts.join(' · ')
}

const groups = [
  { key: 'up', title: 'Subidas', icon: ArrowUp },
  { key: 'down', title: 'Descargas', icon: ArrowDown },
]
const listFor = key => (key === 'up' ? uploads.value : downloads.value)
</script>

<template>
  <Teleport to="body" :disabled="inline">
    <Transition name="pop">
      <div v-if="show || inline" class="tp" :class="{ inline }">
        <template v-for="g in groups" :key="g.key">
          <section v-if="listFor(g.key).length" class="sec">
            <p class="sec-head"><component :is="g.icon" :size="13" :stroke-width="2.6" /> {{ g.title }}</p>
            <div v-for="t in listFor(g.key)" :key="t.id" class="item">
              <div class="top">
                <span class="name" :title="t.name">{{ t.name }}</span>
                <button v-if="t.status === 'active'" type="button" class="act stop" :title="tr('Cancel', 'Cancelar')" @click="cancelTransfer(t.id)"><Square :size="10" fill="currentColor" /></button>
              </div>
              <div class="bar"><i :class="[t.status, { indet: !t.totalSize && t.status === 'active' }]" :style="t.totalSize ? { width: `${pct(t.loaded, t.totalSize)}%` } : null" /></div>
              <p class="meta">
                <template v-if="t.status === 'active'">{{ meta(t.loaded, t.totalSize, t.speed, t.status) }}</template>
                <span v-else-if="t.status === 'done'" class="ok"><Check :size="12" :stroke-width="3" /> {{ formatBytes(t.totalSize || t.loaded) }}</span>
                <span v-else class="bad"><X :size="12" :stroke-width="3" /> {{ t.error }}</span>
              </p>
            </div>
          </section>
        </template>

        <section v-if="visiblePendingUploads.length" class="sec">
          <p class="sec-head"><ArrowUp :size="13" :stroke-width="2.6" /> {{ tr('Paused', 'En pausa') }}</p>
          <div v-for="p in visiblePendingUploads" :key="p.upload_id" class="item">
            <div class="top">
              <span class="name" :title="p.filename">{{ p.filename }}</span>
              <button type="button" class="act play" :title="tr('Resume', 'Reanudar')" @click="resumeUpload(p)"><Play :size="11" fill="currentColor" /></button>
            </div>
            <div class="bar"><i class="paused" :style="{ width: `${pct(p.bytes_received, p.file_size) || 0}%` }" /></div>
            <p class="meta">{{ tr(`${formatBytes(p.bytes_received)} of ${formatBytes(p.file_size)} · resume it by choosing the same file`, `${formatBytes(p.bytes_received)} de ${formatBytes(p.file_size)} · reanúdala eligiendo el mismo archivo`) }}</p>
          </div>
        </section>

        <section v-if="jobs.length" class="sec">
          <p class="sec-head"><Archive :size="13" :stroke-width="2.4" /> Descomprimiendo</p>
          <div v-for="j in jobs" :key="j.job_id" class="item">
            <div class="top">
              <span class="name" :title="j.filename">{{ j.filename }}</span>
              <button v-if="j.status === 'active'" type="button" class="act stop" :title="tr('Cancel', 'Cancelar')" @click="cancelJob(j.job_id)"><Square :size="10" fill="currentColor" /></button>
            </div>
            <div class="bar"><i class="extract" :class="[j.status, { indet: !j.total_bytes && j.status === 'active' }]" :style="j.total_bytes ? { width: `${pct(j.extracted_bytes, j.total_bytes)}%` } : null" /></div>
            <p class="meta">
              <template v-if="j.status === 'active'">{{ meta(j.extracted_bytes, j.total_bytes, j.speed, j.status) }}</template>
              <span v-else-if="j.status === 'done'" class="ok"><Check :size="12" :stroke-width="3" /> {{ formatBytes(j.total_bytes) }}</span>
              <span v-else class="bad"><X :size="12" :stroke-width="3" /> {{ j.error || 'Error' }}</span>
            </p>
          </div>
        </section>

        <p v-if="!uploads.length && !downloads.length && !visiblePendingUploads.length && !jobs.length" class="empty">{{ tr('Nothing in progress.', 'Nada en curso.') }}</p>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.tp {
  position: fixed;
  left: 268px;
  bottom: 70px;
  z-index: 200;
  width: 340px;
  max-height: 60vh;
  overflow-y: auto;
  padding: 8px 0;
  border-radius: 18px;
  background: var(--glass-strong);
  backdrop-filter: blur(30px) saturate(190%);
  -webkit-backdrop-filter: blur(30px) saturate(190%);
  box-shadow: 0 16px 50px rgba(0, 0, 0, 0.2), 0 0 0 0.5px var(--separator);
}
.tp.inline { position: static; width: auto; max-height: none; padding: 0; border-radius: var(--radius-card); background: var(--cell); backdrop-filter: none; box-shadow: none; }
.sec { padding: 4px 16px 8px; }
.sec + .sec { border-top: 0.5px solid var(--separator); padding-top: 10px; }
.sec-head { display: flex; align-items: center; gap: 6px; padding: 4px 0 6px; font-size: 12px; font-weight: 700; color: var(--label-2); }
.item { padding: 6px 0; }
.top { display: flex; align-items: center; gap: 8px; }
.name { flex: 1; min-width: 0; font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.act { width: 22px; height: 22px; display: grid; place-items: center; border-radius: 50%; background: var(--fill-3); flex-shrink: 0; }
.act.stop { color: var(--red); }
.act.play { color: var(--green); }
.bar { position: relative; height: 4px; margin: 6px 0 4px; border-radius: 2px; background: var(--fill-3); overflow: hidden; }
.bar i { position: absolute; left: 0; top: 0; bottom: 0; border-radius: 2px; background: var(--accent); transition: width 0.4s; }
.bar i.done { width: 100% !important; background: var(--green); }
.bar i.error { width: 100% !important; background: var(--red); }
.bar i.paused { background: var(--orange); }
.bar i.extract { background: var(--purple); }
.bar i.indet { width: 35%; animation: indet 1.4s ease-in-out infinite; }
@keyframes indet { from { left: -35%; } to { left: 100%; } }
.meta { font-size: 12px; color: var(--label-2); min-height: 16px; }
.ok, .bad { display: inline-flex; align-items: center; gap: 4px; }
.ok { color: var(--green); }
.bad { color: var(--red); }
.empty { padding: 20px; text-align: center; color: var(--label-2); font-size: 14px; }
.pop-enter-active { transition: opacity 0.15s, transform 0.25s var(--spring); }
.pop-leave-active { transition: opacity 0.12s; }
.pop-enter-from { opacity: 0; transform: translateY(8px) scale(0.97); }
.pop-leave-to { opacity: 0; }
</style>
