<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import Icon from '@ui/Icon.vue'
import ListSection from '@ui/ListSection.vue'
import { api } from '@ui/api.js'
import { toast } from '@ui/dialog.js'

// The desktop dashboard's side column: the things worth doing without opening a module. Each
// panel asks its own module and quietly stays away if that module doesn't answer.
const props = defineProps({ user: { type: Object, required: true } })

const has = module => props.user.modules?.includes(module)

const blinds = ref(null)
const photos = ref(null)
const disk = ref(null)
const server = ref(null)

async function load(target, path, pick = data => data) {
  try {
    target.value = pick(await api(path))
  } catch {
    target.value = null
  }
}

const loadBlinds = () => load(blinds, '/home/api/devices/blinds')
const loadServer = () => load(server, '/admin/api/system')

// ---------- Persianas ----------

const usable = b => b.connection?.online && !b.locked
const anyUsable = computed(() => blinds.value?.some(usable))

function blindState(b) {
  if (b.locked) return 'Bloqueada'
  if (!b.connection?.online) return 'Sin conexión'
  if (b.state?.motor_state) return 'Moviéndose…'
  const pos = Math.round(b.state?.position ?? 0)
  return pos >= 100 ? 'Abierta' : pos <= 0 ? 'Cerrada' : `Abierta al ${pos} %`
}

let followUp = null
async function command(list, action) {
  const results = await Promise.allSettled(list.map(b => api(`/home/api/commands/${b.id}/${action}`, { method: 'POST' })))
  if (results.some(r => r.status === 'rejected')) toast('Alguna persiana no ha respondido', 'warning')
  // The states come back over the next seconds while they move.
  clearTimeout(followUp)
  loadBlinds()
  followUp = setTimeout(loadBlinds, 1500)
}
const all = action => command(blinds.value.filter(usable), action)

// ---------- Fotos ----------

function thumbnail(p) {
  const params = new URLSearchParams({ path: p.path, location: 'external', size: '320', v: p.checksum })
  return `/store/api/files/thumbnail?${params}`
}

// ---------- Almacenamiento y servidor ----------

function bytes(n) {
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let i = 0
  while (n >= 1000 && i < units.length - 1) { n /= 1000; i++ }
  return `${n.toFixed(n < 10 && i > 2 ? 1 : 0).replace('.', ',')} ${units[i]}`
}
const diskShare = computed(() => (disk.value ? disk.value.used / disk.value.total : 0))

function uptime(seconds) {
  const d = Math.floor(seconds / 86400)
  const h = Math.floor((seconds % 86400) / 3600)
  return d ? `${d} d ${h} h` : `${h} h ${Math.floor((seconds % 3600) / 60)} min`
}
const stats = computed(() => {
  const s = server.value
  if (!s) return []
  return [
    { label: 'CPU', value: s.cpu_percent == null ? '—' : `${Math.round(s.cpu_percent)} %` },
    { label: 'Temperatura', value: s.temperature_c == null ? '—' : `${Math.round(s.temperature_c)} °C` },
    { label: 'Memoria', value: `${Math.round((100 * s.memory.used) / s.memory.total)} %` },
    { label: 'Encendido', value: uptime(s.uptime_seconds) },
  ]
})

// Blinds and the server change while the page is open; the rest is read once.
let timer = null
function refresh() {
  if (document.hidden) return
  if (has('home')) loadBlinds()
  if (has('admin') && props.user.is_admin) loadServer()
}

onMounted(() => {
  if (has('store')) {
    load(photos, '/store/api/photos?limit=8', data => data.items)
    load(disk, '/store/api/files/disk')
  }
  refresh()
  timer = setInterval(refresh, 5000)
  document.addEventListener('visibilitychange', refresh)
})
onBeforeUnmount(() => {
  clearInterval(timer)
  clearTimeout(followUp)
  document.removeEventListener('visibilitychange', refresh)
})
</script>

<template>
  <aside class="quick">
    <ListSection v-if="blinds?.length">
      <template #header>Persianas</template>
      <template #action><a href="/home/" class="btn-plain">Abrir Casa</a></template>

      <div class="row blind all">
        <span class="row-body"><span class="row-title">Todas</span></span>
        <span class="controls">
          <button class="control" :disabled="!anyUsable" title="Subir todas" @click="all('up')"><Icon name="chevron-up" :size="18" :weight="2.4" /></button>
          <button class="control" :disabled="!anyUsable" title="Parar todas" @click="all('stop')"><Icon name="stop" :size="15" :weight="2.2" /></button>
          <button class="control" :disabled="!anyUsable" title="Bajar todas" @click="all('down')"><Icon name="chevron-down" :size="18" :weight="2.4" /></button>
        </span>
      </div>
      <div v-for="b in blinds" :key="b.id" class="row blind">
        <span class="level" :class="{ off: !usable(b) }"><i :style="{ height: `${100 - (b.state?.position ?? 0)}%` }" /></span>
        <span class="row-body">
          <span class="row-title">{{ b.name }}</span>
          <span class="row-subtitle small">{{ blindState(b) }}</span>
        </span>
        <span class="controls">
          <button class="control" :disabled="!usable(b)" title="Subir" @click="command([b], 'up')"><Icon name="chevron-up" :size="18" :weight="2.4" /></button>
          <button class="control" :disabled="!usable(b)" title="Parar" @click="command([b], 'stop')"><Icon name="stop" :size="15" :weight="2.2" /></button>
          <button class="control" :disabled="!usable(b)" title="Bajar" @click="command([b], 'down')"><Icon name="chevron-down" :size="18" :weight="2.4" /></button>
        </span>
      </div>
    </ListSection>

    <ListSection v-if="photos?.length">
      <template #header>Fotos recientes</template>
      <template #action><a href="/photos/" class="btn-plain">Ver todas</a></template>
      <div class="photos">
        <a v-for="p in photos" :key="p.id" href="/photos/" class="photo">
          <img :src="thumbnail(p)" :alt="p.original_filename" loading="lazy" @error="e => e.target.classList.add('missing')" />
        </a>
      </div>
    </ListSection>

    <ListSection v-if="disk">
      <template #header>Almacenamiento</template>
      <template #action><a href="/store/" class="btn-plain">Archivos</a></template>
      <div class="panel">
        <div class="meter"><i :style="{ width: `${diskShare * 100}%` }" :class="{ full: diskShare > 0.9 }" /></div>
        <p class="footnote secondary">{{ bytes(disk.free) }} libres de {{ bytes(disk.total) }}</p>
      </div>
    </ListSection>

    <ListSection v-if="server">
      <template #header>Servidor</template>
      <div class="stats">
        <div v-for="s in stats" :key="s.label" class="stat">
          <span class="caption secondary">{{ s.label }}</span>
          <span class="headline">{{ s.value }}</span>
        </div>
      </div>
    </ListSection>
  </aside>
</template>

<style scoped>
/* Its first card lines up with the first service tile; the header sits above it. */
.quick { min-width: 0; margin-top: -28px; }

.blind { gap: 12px; padding-right: 12px; }
.blind.all { --inset: 20px; }
.blind:not(.all) { --inset: 62px; }
/* A little window that fills from the top as the blind comes down. */
.level {
  position: relative;
  flex-shrink: 0;
  width: 30px;
  height: 30px;
  border-radius: 8px;
  overflow: hidden;
  background: color-mix(in srgb, var(--yellow) 55%, var(--cell));
}
.level i { position: absolute; inset: 0 0 auto; background: var(--gray); transition: height 0.4s var(--ease); }
.level.off { opacity: 0.4; }
.controls { display: flex; gap: 6px; flex-shrink: 0; }
.control {
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  border-radius: 999px;
  background: var(--fill-3);
  color: var(--label);
  transition: transform 0.25s var(--spring), background-color 0.15s;
}
.control:hover:not(:disabled) { background: var(--fill-2); }
.control:active:not(:disabled) { transform: scale(0.9); }
.control:disabled { opacity: 0.35; cursor: default; }

.photos { display: grid; grid-template-columns: repeat(4, 1fr); gap: 2px; }
.photo { aspect-ratio: 1; overflow: hidden; background: var(--fill-3); }
.photo img { width: 100%; height: 100%; object-fit: cover; transition: transform 0.3s var(--ease); }
.photo img.missing { visibility: hidden; }
@media (hover: hover) { .photo:hover img { transform: scale(1.05); } }

.panel { padding: 16px 20px; display: flex; flex-direction: column; gap: 8px; }
.meter { height: 8px; border-radius: 999px; background: var(--fill-3); overflow: hidden; }
.meter i { display: block; height: 100%; border-radius: inherit; background: var(--blue); }
.meter i.full { background: var(--orange); }

.stats { display: grid; grid-template-columns: 1fr 1fr; }
.stat { display: flex; flex-direction: column; gap: 2px; padding: 12px 20px; }
.stat:nth-child(n + 3) { border-top: 0.5px solid var(--separator); }
.stat:nth-child(even) { border-left: 0.5px solid var(--separator); }
</style>
