<script setup>
import { computed } from 'vue'
import {
  Folder,
  File,
  FileText,
  Image,
  Film,
  Music,
  Archive,
  Code,
  Download,
  Trash2,
} from 'lucide-vue-next'

const props = defineProps({
  entry: Object,
})

const emit = defineEmits(['open', 'delete', 'download'])

const IMAGE_EXTS = new Set(['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg', 'bmp', 'tiff', 'heic'])
const VIDEO_EXTS = new Set(['mp4', 'mkv', 'avi', 'mov', 'webm', 'flv', 'm4v'])
const AUDIO_EXTS = new Set(['mp3', 'wav', 'flac', 'ogg', 'aac', 'm4a', 'opus'])
const ARCHIVE_EXTS = new Set(['zip', 'tar', 'gz', 'bz2', '7z', 'rar', 'xz', 'zst'])
const CODE_EXTS = new Set(['js', 'ts', 'py', 'go', 'rs', 'java', 'c', 'cpp', 'h', 'css', 'html', 'json', 'yaml', 'yml', 'sh', 'bash', 'zsh', 'toml', 'xml', 'vue', 'jsx', 'tsx'])
const DOC_EXTS = new Set(['pdf', 'txt', 'md', 'doc', 'docx', 'odt', 'rtf', 'csv', 'xls', 'xlsx'])

const fileIcon = computed(() => {
  if (props.entry.type === 'directory') return Folder
  const ext = (props.entry.name.split('.').pop() || '').toLowerCase()
  if (IMAGE_EXTS.has(ext)) return Image
  if (VIDEO_EXTS.has(ext)) return Film
  if (AUDIO_EXTS.has(ext)) return Music
  if (ARCHIVE_EXTS.has(ext)) return Archive
  if (CODE_EXTS.has(ext)) return Code
  if (DOC_EXTS.has(ext)) return FileText
  return File
})

const iconColor = computed(() =>
  props.entry.type === 'directory' ? '#007AFF' : '#636366'
)

function formatSize(bytes) {
  if (bytes === null || bytes === undefined) return '—'
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`
}

function formatDate(ts) {
  return new Date(ts * 1000).toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}
</script>

<template>
  <tr class="file-row">
    <td class="cell-name">
      <button
        class="name-btn"
        :class="{ 'is-dir': entry.type === 'directory' }"
        @click="entry.type === 'directory' && emit('open', entry)"
      >
        <component :is="fileIcon" class="file-icon" :style="{ color: iconColor }" />
        <span class="file-name">{{ entry.name }}</span>
      </button>
    </td>

    <td class="cell-meta">{{ formatSize(entry.size) }}</td>
    <td class="cell-meta">{{ formatDate(entry.modified) }}</td>

    <td class="cell-actions">
      <div class="row-actions">
        <button
          v-if="entry.type === 'file'"
          class="action-btn"
          @click="emit('download', entry)"
          title="Download"
        >
          <Download class="action-icon" />
        </button>
        <button
          class="action-btn delete-btn"
          @click="emit('delete', entry)"
          title="Delete"
        >
          <Trash2 class="action-icon" />
        </button>
      </div>
    </td>
  </tr>
</template>

<style scoped>
.file-row {
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.05);
  transition: background 0.1s;
}

.file-row:hover {
  background: rgba(255, 255, 255, 0.04);
}

.file-row:hover .row-actions {
  opacity: 1;
}

.cell-name {
  padding: 0.65rem 1.25rem;
}

.cell-meta {
  padding: 0.65rem 1.25rem;
  text-align: right;
  font-size: 0.8rem;
  color: #525252;
  white-space: nowrap;
}

.cell-actions {
  padding: 0.65rem 1.25rem;
}

.name-btn {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  font-size: 0.875rem;
  text-align: left;
  width: 100%;
  background: none;
  border: none;
  cursor: default;
}

.name-btn.is-dir { cursor: pointer; }

.file-icon { width: 16px; height: 16px; flex-shrink: 0; }

.file-name {
  color: #d1d1d6;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.25rem;
  opacity: 0;
  transition: opacity 0.15s;
}

.action-btn {
  padding: 0.3rem;
  border-radius: 6px;
  background: none;
  border: none;
  color: #636366;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.action-btn:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #fff;
}

.delete-btn:hover {
  background: rgba(255, 69, 58, 0.15);
  color: #ff453a;
}

.action-icon { width: 14px; height: 14px; }
</style>
