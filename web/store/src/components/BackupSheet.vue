<script setup>
import { onMounted, onUnmounted, ref } from 'vue'
import { AlertCircle, CheckCircle2, DatabaseBackup } from 'lucide-vue-next'
import Sheet from '../ui/Sheet.vue'
import Spinner from '../ui/Spinner.vue'
import ListSection from '../ui/ListSection.vue'
import ListRow from '../ui/ListRow.vue'

// The storage drive's copy onto the backup drive (admins only).
const emit = defineEmits(['close'])

const open = ref(true)
const status = ref(null)
const error = ref(null)
let timer = null

async function fetchStatus() {
  try {
    const res = await fetch('/store/api/backup/status')
    if (res.ok) status.value = await res.json()
  } catch { /* keep the last one */ }
}

async function run() {
  error.value = null
  const res = await fetch('/store/api/backup/run', { method: 'POST' })
  if (res.ok) await fetchStatus()
  else error.value = (await res.json().catch(() => ({}))).detail ?? 'No se pudo lanzar la copia'
}

function formatDate(iso) {
  return iso ? new Date(iso).toLocaleString('es-ES', { dateStyle: 'long', timeStyle: 'short' }) : '—'
}

onMounted(() => { fetchStatus(); timer = setInterval(fetchStatus, 3000) })
onUnmounted(() => clearInterval(timer))
</script>

<template>
  <Sheet v-model:open="open" title="Copia de seguridad" @close="emit('close')">
    <div v-if="!status" class="center"><Spinner :size="24" /></div>
    <template v-else>
      <div class="hero">
        <span class="badge-icon" :class="status.running ? 'blue' : status.last_exit_code === 0 ? 'green' : status.last_exit_code !== null ? 'red' : 'gray'">
          <Spinner v-if="status.running" :size="26" style="color: #fff" />
          <CheckCircle2 v-else-if="status.last_exit_code === 0" :size="34" />
          <AlertCircle v-else-if="status.last_exit_code !== null" :size="34" />
          <DatabaseBackup v-else :size="32" />
        </span>
        <p class="title-3">
          {{ status.running ? 'Copiando…' : status.last_exit_code === 0 ? 'Copia al día' : status.last_exit_code !== null ? `La última copia falló (${status.last_exit_code})` : 'Todavía no hay copias' }}
        </p>
      </div>
      <ListSection footer="Copia el disco de almacenamiento en el disco de copias con rsync. Se hace sola cada domingo a las 2:00.">
        <ListRow title="Última copia" :value="formatDate(status.last_run)" />
      </ListSection>
      <button type="button" class="btn btn-prominent block" :disabled="status.running" @click="run">
        {{ status.running ? 'En curso…' : 'Hacer una copia ahora' }}
      </button>
      <p v-if="error" class="footnote danger err">{{ error }}</p>
    </template>
  </Sheet>
</template>

<style scoped>
.center { display: grid; place-items: center; padding: 40px; }
.hero { display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 4px 0 24px; text-align: center; }
.badge-icon { width: 72px; height: 72px; display: grid; place-items: center; border-radius: 22px; color: #fff; }
.badge-icon.blue { background: var(--blue); }
.badge-icon.green { background: var(--green); }
.badge-icon.red { background: var(--red); }
.badge-icon.gray { background: var(--gray); }
.err { text-align: center; margin-top: 12px; }
</style>
