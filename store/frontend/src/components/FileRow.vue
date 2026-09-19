<script setup>
import { computed } from 'vue'
import { getFileIcon, getIconColor } from '../fileTypes.js'
import { formatSize } from '../utils/formatSize.js'

const props = defineProps({ entry: Object })
const emit = defineEmits(['open', 'view', 'contextmenu'])

let longPressTimer = null
let longPressActivated = false

function onTouchStart(e) {
  longPressActivated = false
  const touch = e.touches[0]
  longPressTimer = setTimeout(() => {
    longPressActivated = true
    emit('contextmenu', props.entry, {
      clientX: touch.clientX,
      clientY: touch.clientY,
      preventDefault: () => {},
      stopPropagation: () => {},
    })
  }, 300)
}

function onTouchEnd() { clearTimeout(longPressTimer) }
function onTouchMove() { clearTimeout(longPressTimer) }

function onRowClick() {
  if (longPressActivated) return
  if (props.entry.type === 'directory' || props.entry.type === 'share-link') emit('open', props.entry)
  // upload-pending: no action on click
}

function onRowDblClick() {
  if (props.entry.type === 'file' || props.entry.type === 'share-link') emit('view', props.entry)
}

const fileIcon = computed(() => getFileIcon(props.entry))
const iconColor = computed(() => getIconColor(props.entry))

function formatDate(ts) {
  return new Date(ts * 1000).toLocaleDateString('en-US', {
    year: 'numeric', month: 'short', day: 'numeric',
  })
}
</script>

<template>
  <tr
    class="file-row"
    @click="onRowClick"
    @dblclick="onRowDblClick"
    @contextmenu.stop="emit('contextmenu', entry, $event)"
    @touchstart.passive="onTouchStart"
    @touchend="onTouchEnd"
    @touchmove="onTouchMove"
    @touchcancel="onTouchEnd"
  >
    <td class="cell-name">
      <div class="name-btn">
        <component :is="fileIcon" class="file-icon" :style="{ color: iconColor }" />
        <span class="file-name">{{ entry.name }}</span>
      </div>
    </td>
    <td class="cell-meta">
      <template v-if="entry.type === 'upload-pending'">
        <span class="pending-pct">{{ Math.round((entry.bytes_received / entry.size) * 100) }}%</span>
      </template>
      <template v-else>{{ formatSize(entry.size) }}</template>
    </td>
    <td class="cell-meta">{{ formatDate(entry.modified) }}</td>
  </tr>
</template>

<style scoped>
.file-row {
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.05);
  transition: background 0.1s;
}

.file-row:hover { background: rgba(255, 255, 255, 0.04); }

.cell-name { padding: 0.9rem 1.25rem; }

.cell-meta {
  padding: 0.9rem 1.25rem;
  text-align: right;
  font-size: 0.875rem;
  color: #525252;
  white-space: nowrap;
}

.name-btn {
  display: flex;
  align-items: center;
  gap: 0.7rem;
  font-size: 1rem;
  width: 100%;
}

.file-icon { width: 20px; height: 20px; flex-shrink: 0; }

.file-name {
  color: #d1d1d6;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pending-pct {
  color: #ff9f0a;
  font-size: 0.8rem;
  font-weight: 500;
}

@media (max-width: 767px) {
  .cell-meta { display: none; }
}
</style>
