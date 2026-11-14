<script setup>
import { computed, ref, watch } from 'vue'
import { ChevronLeft } from 'lucide-vue-next'
import Sheet from '../ui/Sheet.vue'
import Spinner from '../ui/Spinner.vue'
import ListSection from '../ui/ListSection.vue'
import ListRow from '../ui/ListRow.vue'
import { toast } from '../ui/dialog.js'
import { folderTitle, rootOf } from '../store.js'
import FileIcon from './FileIcon.vue'
import { t } from '../ui/i18n.js'
import { apiError } from '../apiErrors.js'

// Pick a folder to move or copy an item into, browsing down from where it is.
const props = defineProps({
  entry: Object,
  location: String,
  currentPath: String,
  mode: { type: String, default: 'move' },
})
const emit = defineEmits(['close', 'moved'])

const open = ref(true)
const browsePath = ref(props.currentPath)
const dirs = ref([])
const loading = ref(false)
const busy = ref(false)

async function load(path) {
  loading.value = true
  try {
    const params = new URLSearchParams({ path, location: props.location })
    const res = await fetch(`/store/api/files/list?${params}`)
    const data = res.ok ? await res.json() : { entries: [] }
    dirs.value = data.entries.filter(e =>
      e.type === 'directory' && !(props.entry.type === 'directory' && e.name === props.entry.name && path === props.currentPath),
    )
  } finally {
    loading.value = false
  }
}
watch(browsePath, load, { immediate: true })

const root = computed(() => rootOf(props.currentPath))
const canGoUp = computed(() => browsePath.value !== root.value && browsePath.value.length > root.value.length)
const parentPath = computed(() => browsePath.value.split('/').slice(0, -1).join('/'))
const verb = computed(() => (props.mode === 'copy' ? t('Copy', 'Copiar') : t('Move', 'Mover')))

function enter(name) {
  browsePath.value = browsePath.value ? `${browsePath.value}/${name}` : name
}

async function confirm() {
  busy.value = true
  try {
    const res = await fetch(props.mode === 'copy' ? '/store/api/files/copy' : '/store/api/files/move', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: props.currentPath ? `${props.currentPath}/${props.entry.name}` : props.entry.name, dest_dir: browsePath.value, location: props.location }),
    })
    if (!res.ok) {
      const data = await res.json().catch(() => ({}))
      throw new Error(apiError(data.detail) || t(`Couldn’t ${verb.value.toLowerCase()}`, `No se pudo ${verb.value.toLowerCase()}`))
    }
    toast(props.mode === 'copy' ? t(`Copied to “${folderTitle(browsePath.value)}”`, `Copiado a «${folderTitle(browsePath.value)}»`) : t(`Moved to “${folderTitle(browsePath.value)}”`, `Movido a «${folderTitle(browsePath.value)}»`))
    emit('moved')
  } catch (e) {
    toast(e.message, 'warning')
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <Sheet
    v-model:open="open"
    :title="t(`${verb} “${entry.name}”`, `${verb} «${entry.name}»`)"
    confirmable
    :can-confirm="browsePath !== currentPath"
    :busy="busy"
    @confirm="confirm"
    @close="emit('close')"
  >
    <div class="where">
      <button v-if="canGoUp" type="button" class="up" @click="browsePath = parentPath">
        <ChevronLeft :size="20" :stroke-width="2.4" />
        {{ folderTitle(parentPath) }}
      </button>
      <div class="here">
        <FileIcon :entry="{ type: 'directory', name: '' }" :size="44" />
        <div>
          <p class="headline">{{ folderTitle(browsePath) }}</p>
          <p class="footnote secondary">{{ browsePath === currentPath ? t('It’s already here', 'Ya está aquí') : t(`${verb} here with ✓`, `${verb} aquí con ✓`) }}</p>
        </div>
      </div>
    </div>

    <div v-if="loading" class="loading"><Spinner :size="22" /></div>
    <ListSection v-else-if="dirs.length" :header="t('Folders', 'Carpetas')">
      <ListRow v-for="d in dirs" :key="d.name" :title="d.name" clickable :chevron="true" @click="enter(d.name)">
        <template #leading><FileIcon :entry="d" :size="30" /></template>
      </ListRow>
    </ListSection>
    <p v-else class="footnote secondary none">{{ t('No more folders inside.', 'No hay más carpetas dentro.') }}</p>
  </Sheet>
</template>

<style scoped>
.where { padding: 0 4px 18px; }
.up { display: flex; align-items: center; gap: 2px; margin: 0 0 10px -6px; color: var(--accent); font-size: 17px; }
.here { display: flex; align-items: center; gap: 14px; }
.loading { display: grid; place-items: center; padding: 30px; }
.none { text-align: center; padding: 24px; }
.section :deep(.row) { --inset: 64px; }
</style>
