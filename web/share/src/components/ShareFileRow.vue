<script setup>
import { computed } from 'vue'
import { Download } from 'lucide-vue-next'
import { getFileIcon, getIconColor } from '../fileTypes.js'
import { formatSize } from '../utils/formatSize.js'

const props = defineProps({
  entry: Object,
})
const emit = defineEmits(['open', 'open-file', 'open-tab', 'download'])

const fileIcon = computed(() => getFileIcon(props.entry))
const iconColor = computed(() => getIconColor(props.entry))

function onRowClick() {
  if (props.entry.type === 'directory') emit('open', props.entry)
}

function onRowDblClick() {
  if (props.entry.type === 'file') emit('open-file', props.entry)
}

function onRowAuxClick(e) {
  if (e.button === 1 && props.entry.type === 'file') emit('open-tab', props.entry)
}
</script>

<template>
  <tr class="file-row" @click="onRowClick" @dblclick="onRowDblClick" @auxclick.prevent="onRowAuxClick">
    <td class="cell-name">
      <div class="name-btn">
        <component :is="fileIcon" class="file-icon" :style="{ color: iconColor }" />
        <span class="file-name">{{ entry.name }}</span>
      </div>
    </td>
    <td class="cell-size">{{ formatSize(entry.size) }}</td>
    <td class="cell-action">
      <button v-if="entry.type === 'file'" class="dl-btn" @click.stop="emit('download', entry)">
        <Download class="dl-icon" />
      </button>
    </td>
  </tr>
</template>

<style scoped>
.file-row {
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.05);
  transition: background 0.1s;
}
.file-row:hover { background: rgba(255, 255, 255, 0.04); }

.cell-name { padding: 0.9rem 1.25rem; width: 100%; }

.cell-size {
  padding: 0.9rem 1rem;
  text-align: right;
  font-size: 0.875rem;
  color: #525252;
  white-space: nowrap;
}

.cell-action { padding: 0 1rem 0 0.5rem; }

.name-btn {
  display: flex;
  align-items: center;
  gap: 0.7rem;
  font-size: 1rem;
}

.file-icon { width: 20px; height: 20px; flex-shrink: 0; }

.file-name {
  color: #d1d1d6;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dl-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: transparent;
  border: none;
  color: #636366;
  opacity: 0;
  transition: opacity 0.15s, background 0.15s, color 0.15s;
}

.file-row:hover .dl-btn { opacity: 1; }
.dl-btn:hover { background: rgba(255,255,255,0.07); color: #fff; }

.dl-icon { width: 15px; height: 15px; }

@media (max-width: 767px) {
  .cell-size { display: none; }
  .dl-btn { opacity: 1; }
}
</style>
