<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import Icon from '@ui/Icon.vue'
import { api } from '@ui/api.js'
import { toast } from '@ui/dialog.js'
import { t } from '@ui/i18n.js'

// The desktop dashboard's quick access, under the services: the blinds used every day. The card
// asks tphome itself and stays away if it doesn't answer.
const props = defineProps({ user: { type: Object, required: true } })

// The blinds worth a button here (tphome ids); any others are a click away in Casa. If none of
// these exist on a server, its first three are shown instead.
const PINNED_BLINDS = ['B0101', 'B0102', 'B0301']

const has = module => props.user.modules?.includes(module)

const blinds = ref(null)

async function load(target, path) {
  try {
    target.value = await api(path)
  } catch {
    target.value = null
  }
}
const loadBlinds = () => load(blinds, '/home/api/devices/blinds')

// ---------- {{ t('Blinds', 'Persianas') }} ----------

const pinned = computed(() => {
  const list = blinds.value ?? []
  const chosen = PINNED_BLINDS.map(id => list.find(b => b.id === id)).filter(Boolean)
  return chosen.length ? chosen : list.slice(0, 3)
})

const usable = b => b.connection?.online && !b.locked

function blindState(b) {
  if (b.locked) return t('Locked', 'Bloqueada')
  if (!b.connection?.online) return t('Offline', 'Sin conexión')
  if (b.state?.motor_state) return t('Moving…', 'Moviéndose…')
  const pos = openness(b)
  return pos >= 100 ? t('Open', 'Abierta') : pos <= 0 ? t('Closed', 'Cerrada') : t(`${pos}%`, `${pos} %`)
}
// How open it is, 0 (down) to 100 (up), as tphome reports it.
const openness = b => Math.min(100, Math.max(0, Math.round(b.state?.position ?? 0)))

let followUp = null
async function command(b, action) {
  try {
    await api(`/home/api/commands/${b.id}/${action}`, { method: 'POST' })
  } catch {
    toast(t(`${b.name} didn’t respond`, `${b.name} no ha respondido`), 'warning')
  }
  // The state comes back over the next seconds while it moves.
  clearTimeout(followUp)
  loadBlinds()
  followUp = setTimeout(loadBlinds, 1500)
}

// Admins only (tphome checks it too): a locked switch turns its lights off and ignores its
// buttons and every command until it's unlocked.
async function toggleLock(b) {
  const locking = !b.locked
  try {
    await api(`/home/api/admin/${b.id}/${locking ? 'lock' : 'unlock'}`, { method: 'POST' })
    b.locked = locking
    toast(locking ? t(`${b.name} locked`, `${b.name} bloqueada`) : t(`${b.name} unlocked`, `${b.name} desbloqueada`))
  } catch (e) {
    toast(e.message, 'warning')
  }
  loadBlinds()
}

// The blinds change while the page is open.
let timer = null
function refresh() {
  if (document.hidden) return
  if (has('home')) loadBlinds()
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
      <span class="quick-icon" style="background: var(--green)"><Icon name="blinds" :size="20" :weight="2" /></span>
      <span class="headline">{{ t('Blinds', 'Persianas') }}</span>
      <span class="footnote secondary">{{ t('Open Home', 'Abrir Casa') }}</span>
      <Icon name="chevron-right" :size="17" :weight="2.4" class="row-chevron" />
    </a>
    <div class="cells">
      <div v-for="b in pinned" :key="b.id" class="cell">
        <span class="blind-top">
          <span class="cell-text">
            <span class="subheadline name">{{ b.name }}</span>
            <span class="footnote secondary">{{ blindState(b) }}</span>
          </span>
        </span>
        <span class="meter open" :class="{ off: !usable(b) }" role="meter" :aria-valuenow="openness(b)" aria-valuemin="0" aria-valuemax="100">
          <i :style="{ width: `${openness(b)}%` }" />
        </span>
        <span class="controls">
          <button class="control" :disabled="!usable(b)" :title="t('Raise', 'Subir')" @click="command(b, 'up')"><Icon name="chevron-up" :size="18" :weight="2.4" /></button>
          <button class="control" :disabled="!usable(b)" :title="t('Stop', 'Parar')" @click="command(b, 'stop')"><Icon name="stop" :size="15" :weight="2.2" /></button>
          <button class="control" :disabled="!usable(b)" :title="t('Lower', 'Bajar')" @click="command(b, 'down')"><Icon name="chevron-down" :size="18" :weight="2.4" /></button>
          <button
            v-if="user.is_admin"
            class="control lock"
            :class="{ on: b.locked }"
            :title="b.locked ? t('Unlock', 'Desbloquear') : t('Lock', 'Bloquear')"
            :aria-pressed="b.locked"
            @click="toggleLock(b)"
          ><Icon :name="b.locked ? 'lock' : 'lock-open'" :size="15" :weight="2.2" /></button>
        </span>
      </div>
    </div>
  </section>
</template>

<style scoped>
.quick-card {
  padding: 16px 20px 20px;
  border-radius: var(--radius-card);
  background: var(--cell);
}
.quick-head { display: flex; align-items: center; gap: 12px; margin-bottom: 14px; }
.quick-head .headline { flex: 1; }
.quick-icon { width: 36px; height: 36px; display: grid; place-items: center; border-radius: 10px; color: #fff; flex-shrink: 0; }

.cells { display: grid; grid-template-columns: repeat(auto-fit, minmax(170px, 1fr)); gap: 10px; }
.cell {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px 14px;
  border-radius: 18px;
  background: var(--fill-4);
}
.blind-top { display: flex; align-items: center; gap: 12px; }
.cell-text { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
.name { font-weight: 600; }

.controls { display: flex; gap: 6px; margin-top: auto; }
.control {
  flex: 1;
  height: 32px;
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
/* The admin's lock: round, beside the three, orange while the blind is locked. */
.control.lock { flex: 0 0 32px; color: var(--label-2); }
.control.lock.on { background: var(--orange); color: #fff; }

.meter { height: 8px; border-radius: 999px; background: var(--fill-3); overflow: hidden; }
.meter i { display: block; height: 100%; border-radius: inherit; background: var(--blue); }
/* A blind's bar: how far up it is, green while it can be moved. */
.meter.open { height: 6px; }
.meter.open i { background: var(--green); transition: width 0.4s var(--ease); }
.meter.open.off i { background: var(--gray); }
</style>
