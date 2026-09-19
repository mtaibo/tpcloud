<script setup>
import { ref, computed } from 'vue'
import { Search, X } from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'
import { FOLDER_ICONS_MAP, FOLDER_ICON_CATEGORIES } from '../folderIcons.js'

const props = defineProps({
  entry: Object,
})
const emit = defineEmits(['close', 'done'])

const search = ref('')
const selected = ref(props.entry.icon_name || null)

function iconLabel(name) {
  return name.replace(/([A-Z0-9])/g, ' $1').trim()
}

const allIcons = FOLDER_ICON_CATEGORIES.flatMap(c => c.icons)

const filteredCategories = computed(() => {
  const q = search.value.trim().toLowerCase()
  if (!q) return FOLDER_ICON_CATEGORIES
  const matched = allIcons.filter(name => name.toLowerCase().includes(q))
  return matched.length ? [{ label: null, icons: matched }] : []
})

function select(name) {
  selected.value = selected.value === name ? null : name
}

function done() {
  if (selected.value) {
    emit('done', selected.value)
  } else {
    emit('done', null)
  }
}
</script>

<template>
  <BaseModal width="520px" @close="emit('close')">
    <div class="picker-header">
      <span class="picker-title">Change Icon</span>
      <span class="picker-subtitle">{{ entry.name }}</span>
    </div>

    <div class="picker-search-row">
      <Search class="search-icon" />
      <input
        v-model="search"
        class="search-input"
        placeholder="Search icons…"
        autocomplete="off"
        spellcheck="false"
      />
      <button v-if="search" class="search-clear" @click="search = ''">
        <X class="search-clear-icon" />
      </button>
    </div>

    <div class="picker-body">
      <template v-if="filteredCategories.length === 0">
        <p class="no-results">No icons match "{{ search }}"</p>
      </template>
      <template v-for="cat in filteredCategories" :key="cat.label">
        <p v-if="cat.label" class="cat-label">{{ cat.label }}</p>
        <div class="icon-grid">
          <button
            v-for="name in cat.icons"
            :key="name"
            class="icon-cell"
            :class="{ selected: selected === name }"
            @click="select(name)"
            :title="iconLabel(name)"
          >
            <component :is="FOLDER_ICONS_MAP[name]" class="icon-img" />
            <span class="icon-label">{{ name }}</span>
          </button>
        </div>
      </template>
    </div>

    <div class="picker-footer">
      <button
        v-if="entry.icon_name"
        class="btn-reset"
        @click="emit('done', null)"
      >
        Reset to default
      </button>
      <div class="footer-spacer" />
      <button class="btn-cancel" @click="emit('close')">Cancel</button>
      <button class="btn-primary" :disabled="selected === (entry.icon_name || null)" @click="done">
        Apply
      </button>
    </div>
  </BaseModal>
</template>

<style scoped>
.picker-header {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 1.25rem 1.25rem 0.75rem;
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.08);
}

.picker-title {
  font-size: 0.9375rem;
  font-weight: 600;
  color: #fff;
}

.picker-subtitle {
  font-size: 0.8125rem;
  color: #636366;
}

.picker-search-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.06);
}

.search-icon {
  width: 15px;
  height: 15px;
  color: #636366;
  flex-shrink: 0;
}

.search-input {
  flex: 1;
  background: none;
  border: none;
  outline: none;
  font-size: 0.875rem;
  color: #fff;
  caret-color: #8E8E93;
}

.search-input::placeholder { color: #3a3a3c; }

.search-clear {
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255,255,255,0.06);
  border: none;
  border-radius: 50%;
  width: 18px;
  height: 18px;
  cursor: default;
  flex-shrink: 0;
}

.search-clear-icon { width: 10px; height: 10px; color: #8E8E93; }

.picker-body {
  overflow-y: auto;
  max-height: 340px;
  padding: 0.5rem 0.75rem 0.75rem;
}

.no-results {
  padding: 2rem 0.5rem;
  text-align: center;
  font-size: 0.875rem;
  color: #3a3a3c;
}

.cat-label {
  font-size: 0.65rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #3a3a3c;
  padding: 0.75rem 0.25rem 0.25rem;
}

.icon-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 2px;
}

.icon-cell {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  width: 64px;
  height: 60px;
  border-radius: 8px;
  background: none;
  border: none;
  cursor: default;
  transition: background 0.1s;
  color: #636366;
}

.icon-cell:hover { background: rgba(255, 255, 255, 0.05); color: #8E8E93; }
.icon-cell.selected { background: rgba(255, 255, 255, 0.1); color: #fff; }

.icon-img { width: 22px; height: 22px; flex-shrink: 0; }

.icon-label {
  font-size: 0.55rem;
  line-height: 1;
  color: inherit;
  opacity: 0.7;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 58px;
  text-align: center;
}

.picker-footer {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.875rem 1rem;
  border-top: 0.5px solid rgba(255, 255, 255, 0.08);
}

.footer-spacer { flex: 1; }

.btn-reset {
  font-size: 0.8125rem;
  color: #636366;
  background: none;
  border: none;
  cursor: default;
  padding: 0.375rem 0.5rem;
  border-radius: 6px;
  transition: color 0.15s;
}

.btn-reset:hover { color: #8E8E93; }

.btn-cancel {
  font-size: 0.875rem;
  color: rgba(255, 255, 255, 0.5);
  background: rgba(255, 255, 255, 0.06);
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  padding: 0.4rem 1rem;
  cursor: default;
  transition: background 0.15s;
}

.btn-cancel:hover { background: rgba(255, 255, 255, 0.09); }

.btn-primary {
  font-size: 0.875rem;
  font-weight: 600;
  color: #fff;
  background: #8E8E93;
  border: none;
  border-radius: 8px;
  padding: 0.4rem 1.125rem;
  cursor: default;
  transition: opacity 0.15s;
}

.btn-primary:disabled { opacity: 0.35; }
.btn-primary:not(:disabled):hover { opacity: 0.85; }
</style>
