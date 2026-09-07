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
  return new Date(ts * 1000).toLocaleDateString('es-ES', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}
</script>

<template>
  <tr
    class="group transition-colors"
    style="border-bottom: 1px solid rgba(255,255,255,0.04);"
    @mouseover="(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.03)'"
    @mouseleave="(e) => e.currentTarget.style.background = ''"
  >
    <!-- Name -->
    <td class="px-4 py-2">
      <button
        class="flex items-center gap-2 text-sm text-left w-full"
        :class="entry.type === 'directory' ? 'cursor-pointer' : 'cursor-default'"
        @click="entry.type === 'directory' && emit('open', entry)"
      >
        <component :is="fileIcon" class="w-4 h-4 shrink-0" :style="{ color: iconColor }" />
        <span class="truncate" style="color: #d1d1d6;">{{ entry.name }}</span>
      </button>
    </td>

    <!-- Size -->
    <td class="px-4 py-2 text-right text-sm whitespace-nowrap" style="color: #636366;">
      {{ formatSize(entry.size) }}
    </td>

    <!-- Date -->
    <td class="px-4 py-2 text-right text-sm whitespace-nowrap" style="color: #636366;">
      {{ formatDate(entry.modified) }}
    </td>

    <!-- Actions -->
    <td class="px-4 py-2">
      <div class="flex items-center justify-end gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
        <button
          v-if="entry.type === 'file'"
          @click="emit('download', entry)"
          class="p-1 rounded transition-colors"
          title="Descargar"
          style="color: #636366;"
          @mouseover="(e) => { e.currentTarget.style.background = 'rgba(255,255,255,0.08)'; e.currentTarget.style.color = '#fff' }"
          @mouseleave="(e) => { e.currentTarget.style.background = ''; e.currentTarget.style.color = '#636366' }"
        >
          <Download class="w-4 h-4" />
        </button>
        <button
          @click="emit('delete', entry)"
          class="p-1 rounded transition-colors"
          title="Eliminar"
          style="color: #636366;"
          @mouseover="(e) => { e.currentTarget.style.background = 'rgba(255,69,58,0.15)'; e.currentTarget.style.color = '#ff453a' }"
          @mouseleave="(e) => { e.currentTarget.style.background = ''; e.currentTarget.style.color = '#636366' }"
        >
          <Trash2 class="w-4 h-4" />
        </button>
      </div>
    </td>
  </tr>
</template>
