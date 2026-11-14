<script setup>
import { computed, ref } from 'vue'
import { Search, X } from 'lucide-vue-next'
import Sheet from '../ui/Sheet.vue'
import { FOLDER_ICONS_MAP, FOLDER_ICON_CATEGORIES } from '../folderIcons.js'
import FileIcon from './FileIcon.vue'
import { t } from '../ui/i18n.js'

// The symbol pressed into a folder's front. tpapp shows the same names as SF Symbols.
const props = defineProps({ entry: Object })
const emit = defineEmits(['close', 'done'])

const open = ref(true)
const search = ref('')
const selected = ref(props.entry.icon_name || null)

const categories = computed(() => {
  const q = search.value.trim().toLowerCase()
  if (!q) return FOLDER_ICON_CATEGORIES
  const matched = FOLDER_ICON_CATEGORIES.flatMap(c => c.icons).filter(n => n.toLowerCase().includes(q))
  return matched.length ? [{ label: null, icons: matched }] : []
})

// folderIcons.js names its categories in English.
const CATEGORY_ES = {
  General: 'General', Education: 'Educación', 'Calendar & Time': 'Calendario y hora', Media: 'Multimedia', Documents: 'Documentos',
  Tech: 'Tecnología', Work: 'Trabajo', Personal: 'Personal', 'Nature & Travel': 'Naturaleza y viajes', Misc: 'Otros',
}
const changed = computed(() => selected.value !== (props.entry.icon_name || null))
</script>

<template>
  <Sheet v-model:open="open" :title="t('Folder icon', 'Icono de la carpeta')" confirmable :can-confirm="changed" @confirm="emit('done', selected)" @close="emit('close')">
    <div class="preview">
      <FileIcon :entry="{ ...entry, icon_name: selected }" :size="96" />
      <p class="headline">{{ entry.name }}</p>
      <button v-if="selected" type="button" class="btn btn-bordered small" @click="selected = null">{{ t('Remove icon', 'Quitar icono') }}</button>
    </div>

    <label class="search">
      <Search :size="16" />
      <input v-model="search" :placeholder="t('Search symbols', 'Buscar símbolos')" autocomplete="off" spellcheck="false" />
      <button v-if="search" type="button" class="clear" :aria-label="t('Clear', 'Borrar')" @click="search = ''"><X :size="11" :stroke-width="3" /></button>
    </label>

    <p v-if="!categories.length" class="footnote secondary none">{{ t(`No symbol matches “${search}”.`, `Ningún símbolo coincide con «${search}».`) }}</p>
    <section v-for="cat in categories" :key="cat.label || 'results'" class="cat">
      <p v-if="cat.label" class="section-header">{{ t(cat.label, CATEGORY_ES[cat.label] || cat.label) }}</p>
      <div class="grid card">
        <button
          v-for="name in cat.icons"
          :key="name"
          type="button"
          class="cell"
          :class="{ on: selected === name }"
          :title="name"
          @click="selected = selected === name ? null : name"
        >
          <component :is="FOLDER_ICONS_MAP[name]" :size="22" :stroke-width="1.8" />
        </button>
      </div>
    </section>
  </Sheet>
</template>

<style scoped>
.preview { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 0 0 20px; }
.search { display: flex; align-items: center; gap: 8px; height: 38px; padding: 0 12px; margin-bottom: 18px; border-radius: 12px; color: var(--label-2); background: var(--fill-3); }
.search input { flex: 1; min-width: 0; border: none; outline: none; background: none; font-size: 16px; color: var(--label); }
.clear { width: 17px; height: 17px; display: grid; place-items: center; border-radius: 50%; color: var(--bg-elevated); background: var(--label-3); }
.cat { margin-bottom: 18px; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(48px, 1fr)); gap: 4px; padding: 8px; }
.cell { aspect-ratio: 1; display: grid; place-items: center; border-radius: 12px; color: var(--label-2); transition: background 0.15s, color 0.15s; }
.cell:hover { background: var(--fill-3); color: var(--label); }
.cell.on { background: var(--accent); color: #fff; }
.none { text-align: center; padding: 20px; }
</style>
