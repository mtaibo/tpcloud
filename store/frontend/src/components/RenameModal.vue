<script setup>
import { ref, onMounted, computed } from 'vue'
import { Folder, File, FileText, Image, Film, Music, Archive, Code } from 'lucide-vue-next'

const props = defineProps({
  entry: Object,
})

const emit = defineEmits(['close', 'renamed'])

const inputRef = ref(null)
const newName = ref(props.entry.name)

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

onMounted(() => {
  if (!inputRef.value) return
  inputRef.value.focus()
  const name = props.entry.name
  const dotIndex = props.entry.type === 'file' ? name.lastIndexOf('.') : -1
  if (dotIndex > 0) {
    inputRef.value.setSelectionRange(0, dotIndex)
  } else {
    inputRef.value.select()
  }
})

const isValid = computed(() => {
  const trimmed = newName.value.trim()
  return trimmed.length > 0 && trimmed !== props.entry.name
})

function submit() {
  if (!isValid.value) { emit('close'); return }
  emit('renamed', newName.value.trim())
}

function onKeydown(e) {
  if (e.key === 'Enter') submit()
  if (e.key === 'Escape') emit('close')
}
</script>

<template>
  <Teleport to="body">
    <div class="backdrop" @click.self="emit('close')">
      <div class="modal">

        <div class="modal-header">
          <component :is="fileIcon" class="entry-icon" :style="{ color: iconColor }" />
          <div class="header-text">
            <span class="modal-title">Rename</span>
            <span class="modal-subtitle">{{ entry.name }}</span>
          </div>
        </div>

        <div class="modal-body">
          <input
            ref="inputRef"
            v-model="newName"
            class="rename-input"
            type="text"
            spellcheck="false"
            autocomplete="off"
            @keydown="onKeydown"
          />
        </div>

        <div class="modal-footer">
          <button class="btn-cancel" @click="emit('close')">Cancel</button>
          <button class="btn-rename" :disabled="!isValid" @click="submit">Rename</button>
        </div>

      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 10000;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
}

.modal {
  width: 320px;
  display: flex;
  flex-direction: column;
  background: rgba(28, 28, 30, 0.97);
  backdrop-filter: blur(40px) saturate(180%);
  -webkit-backdrop-filter: blur(40px) saturate(180%);
  border: 0.5px solid rgba(255, 255, 255, 0.14);
  border-radius: 14px;
  box-shadow:
    0 24px 60px rgba(0, 0, 0, 0.7),
    0 4px 16px rgba(0, 0, 0, 0.4),
    inset 0 0.5px 0 rgba(255, 255, 255, 0.1);
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 18px 20px 14px;
  border-bottom: 0.5px solid rgba(255, 255, 255, 0.08);
}

.entry-icon {
  width: 28px;
  height: 28px;
  flex-shrink: 0;
}

.header-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.modal-title {
  font-size: 13px;
  font-weight: 600;
  color: #fff;
  letter-spacing: -0.01em;
}

.modal-subtitle {
  font-size: 12px;
  color: #636366;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.modal-body {
  padding: 16px 20px;
}

.rename-input {
  width: 100%;
  padding: 8px 12px;
  background: rgba(255, 255, 255, 0.06);
  border: 0.5px solid rgba(255, 255, 255, 0.12);
  border-radius: 8px;
  color: #fff;
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.rename-input:focus {
  border-color: #007AFF;
  box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.25);
}

.modal-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  padding: 0 16px 16px;
}

.btn-cancel {
  padding: 6px 14px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  color: #adadad;
  background: rgba(255, 255, 255, 0.06);
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
  font-family: inherit;
}

.btn-cancel:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
}

.btn-rename {
  padding: 6px 14px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  color: #fff;
  background: #007AFF;
  border: none;
  cursor: pointer;
  transition: background 0.12s, opacity 0.12s;
  font-family: inherit;
}

.btn-rename:hover:not(:disabled) {
  background: #0066d6;
}

.btn-rename:disabled {
  opacity: 0.35;
  cursor: default;
}

@media (max-width: 767px) {
  .backdrop { align-items: flex-start; padding-top: calc(env(safe-area-inset-top) + 80px); }
  .modal { width: calc(100% - 32px); }
}
</style>
