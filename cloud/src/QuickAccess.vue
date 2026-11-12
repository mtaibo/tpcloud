<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import Icon from '@ui/Icon.vue'
import { api } from '@ui/api.js'
import { toast } from '@ui/dialog.js'

// The desktop dashboard's quick access, under the services: the blinds used every day and how
// the server is doing. Each card asks its own module and stays away if that module doesn't answer.
const props = defineProps({ user: { type: Object, required: true } })

// The blinds worth a button here (tphome ids); any others are a click away in Casa. If none of
// these exist on a server, its first three are shown instead.
const PINNED_BLINDS = ['B0101', 'B0102', 'B0301']

const has = module => props.user.modules?.includes(module)

const blinds = ref(null)
const server = ref(null)

async function load(target, path) {
  try {
    target.value = await api(path)
  } catch {
    target.value = null
  }
}
const loadBlinds = () => load(blinds, '/home/api/devices/blinds')
const loadServer = () => load(server, '/admin/api/system')

// ---------- Persianas ----------

const pinned = computed(() => {
  const list = blinds.value ?? []
  const chosen = PINNED_BLINDS.map(id => list.find(b => b.id === id)).filter(Boolean)
  return chosen.length ? chosen : list.slice(0, 3)
})

const usable = b => b.connection?.online && !b.locked

function blindState(b) {
  if (b.locked) return 'Bloqueada'
  if (!b.connection?.online) return 'Sin conexión'
  if (b.state?.motor_state) return 'Moviéndose…'
  const pos = Math.round(b.state?.position ?? 0)
  return pos >= 100 ? 'Abierta' : pos <= 0 ? 'Cerrada' : `Abierta al ${pos} %`
}

let followUp = null
async function command(b, action) {
  try {
    await api(`/home/api/commands/${b.id}/${action}`, { method: 'POST' })
  } catch {
    toast(`${b.name} no ha respondido`, 'warning')
  }
  // The state comes back over the next seconds while it moves.
  clearTimeout(followUp)
  loadBlinds()
  followUp = setTimeout(loadBlinds, 1500)
}

// ---------- Servidor ----------

function bytes(n) {
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let i = 0
  while (n >= 1000 && i < units.length - 1) { n /= 1000; i++ }
  return `${n.toFixed(n < 10 && i > 2 ? 1 : 0).replace('.', ',')} ${units[i]}`
}

function uptime(seconds) {
  const d = Math.floor(seconds / 86400)
  const h = Math.floor((seconds % 86400) / 3600)
  return d ? `${d} d ${h} h` : `${h} h ${Math.floor((seconds % 3600) / 60)} min`
}

// tpadmin's disks by name: the SD card the system runs from and the external storage drive.
const DISK_NAMES = { system: 'Raspberry', storage: 'Disco externo' }
const disks = computed(() =>
  (server.value?.disks ?? [])
    .filter(d => DISK_NAMES[d.name])
    .map(d => ({ ...d, label: DISK_NAMES[d.name], share: d.used / d.total })),
)

// Both cards change while the page is open.
let timer = null
function refresh() {
  if (document.hidden) return
  if (has('home')) loadBlinds()
  if (has('admin') && props.user.is_admin) loadServer()
}

onMounted(() => {
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
  <section v-if="pinned.length" class="quick-card">
    <a href="/home/" class="quick-head">
      <span class="quick-icon" style="background: var(--orange)"><Icon name="house" :size="24" :weight="1.9" /></span>
      <span class="headline">Persianas</span>
      <span class="footnote secondary">Abrir Casa</span>
      <Icon name="chevron-right" :size="17" :weight="2.4" class="row-chevron" />
    </a>
    <div class="cells">
      <div v-for="b in pinned" :key="b.id" class="cell" :class="{ off: !usable(b) }">
        <span class="blind-top">
          <span class="level"><i :style="{ height: `${100 - (b.state?.position ?? 0)}%` }" /></span>
          <span class="cell-text">
            <span class="subheadline name">{{ b.name }}</span>
            <span class="footnote secondary">{{ blindState(b) }}</span>
          </span>
        </span>
        <span class="controls">
          <button class="control" :disabled="!usable(b)" title="Subir" @click="command(b, 'up')"><Icon name="chevron-up" :size="18" :weight="2.4" /></button>
          <button class="control" :disabled="!usable(b)" title="Parar" @click="command(b, 'stop')"><Icon name="stop" :size="15" :weight="2.2" /></button>
          <button class="control" :disabled="!usable(b)" title="Bajar" @click="command(b, 'down')"><Icon name="chevron-down" :size="18" :weight="2.4" /></button>
        </span>
      </div>
    </div>
  </section>

  <section v-if="server" class="quick-card">
    <div class="quick-head">
      <span class="quick-icon" style="background: var(--indigo)"><Icon name="server" :size="24" :weight="1.9" /></span>
      <span class="headline">Servidor</span>
    </div>
    <div class="cells server">
      <div class="cell stat">
        <span class="footnote secondary">Temperatura</span>
        <span class="stat-value">{{ server.temperature_c == null ? '—' : `${Math.round(server.temperature_c)} °C` }}</span>
      </div>
      <div class="cell stat">
        <span class="footnote secondary">Encendido</span>
        <span class="stat-value">{{ uptime(server.uptime_seconds) }}</span>
      </div>
      <div v-for="d in disks" :key="d.name" class="cell disk">
        <span class="disk-top">
          <span class="subheadline name">{{ d.label }}</span>
          <span class="footnote secondary">{{ bytes(d.free) }} libres de {{ bytes(d.total) }}</span>
        </span>
        <span class="meter"><i :style="{ width: `${d.share * 100}%` }" :class="{ full: d.share > 0.9 }" /></span>
      </div>
    </div>
  </section>
</template>

<style scoped>
.quick-card {
  padding: 20px;
  border-radius: var(--radius-card);
  background: var(--cell);
}
.quick-head { display: flex; align-items: center; gap: 14px; margin-bottom: 16px; }
.quick-head .headline { flex: 1; }
.quick-icon { width: 44px; height: 44px; display: grid; place-items: center; border-radius: 12px; color: #fff; flex-shrink: 0; }

.cells { display: grid; grid-template-columns: repeat(auto-fit, minmax(170px, 1fr)); gap: 10px; }
.cell {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 14px;
  border-radius: 18px;
  background: var(--fill-4);
}
.blind-top { display: flex; align-items: center; gap: 12px; }
.cell-text { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
.name { font-weight: 600; }

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
.cell.off .level { opacity: 0.4; }
.controls { display: flex; gap: 6px; margin-top: auto; }
.control {
  flex: 1;
  height: 34px;
  display: grid;
  place-items: center;
  border-radius: 999px;
  background: var(--fill-3);
  color: var(--label);
  transition: transform 0.25s var(--spring), background-color 0.15s;
}
.control:hover:not(:disabled) { background: var(--fill-2); }
.control:active:not(:disabled) { transform: scale(0.92); }
.control:disabled { opacity: 0.35; cursor: default; }

.cells.server { grid-template-columns: 1fr 1fr; }
.stat { gap: 4px; }
.stat-value { font-size: 22px; line-height: 28px; font-weight: 600; letter-spacing: 0.35px; }
/* Each disk takes a whole row, so its bar has room to read. */
.disk { grid-column: 1 / -1; gap: 10px; }
.disk-top { display: flex; justify-content: space-between; align-items: baseline; gap: 12px; }
.meter { height: 8px; border-radius: 999px; background: var(--fill-3); overflow: hidden; }
.meter i { display: block; height: 100%; border-radius: inherit; background: var(--blue); }
.meter i.full { background: var(--orange); }
</style>
