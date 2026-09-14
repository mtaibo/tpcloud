<script setup>
import { ref, computed, onMounted } from 'vue'
import { FolderPlus, FilePlus } from 'lucide-vue-next'

const props = defineProps({
  type: String, // 'folder' | 'file'
})

const emit = defineEmits(['close', 'created'])

const inputRef = ref(null)
const name = ref('')

const isFolder = computed(() => props.type === 'folder')
const title = computed(() => isFolder.value ? 'New Folder' : 'New File')
const placeholder = computed(() => isFolder.value ? 'Folder name' : 'File name')

const isValid = computed(() => name.value.trim().length > 0)

onMounted(() => {
  inputRef.value?.focus()
})

function submit() {
  if (!isValid.value) return
  emit('created', name.value.trim())
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
          <component :is="isFolder ? FolderPlus : FilePlus" class="entry-icon" />
          <span class="modal-title">{{ title }}</span>
        </div>

        <div class="modal-body">
          <input
            ref="inputRef"
            v-model="name"
            class="create-input"
            type="text"
            :placeholder="placeholder"
            spellcheck="false"
            autocomplete="off"
            @keydown="onKeydown"
          />
        </div>

        <div class="modal-footer">
          <button class="btn-cancel" @click="emit('close')">Cancel</button>
          <button class="btn-create" :disabled="!isValid" @click="submit">Create</button>
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
  width: 22px;
  height: 22px;
  flex-shrink: 0;
  color: #007AFF;
}

.modal-title {
  font-size: 13px;
  font-weight: 600;
  color: #fff;
  letter-spacing: -0.01em;
}

.modal-body {
  padding: 16px 20px;
}

.create-input {
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

.create-input::placeholder {
  color: #3a3a3c;
}

.create-input:focus {
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

.btn-create {
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

.btn-create:hover:not(:disabled) {
  background: #0066d6;
}

.btn-create:disabled {
  opacity: 0.35;
  cursor: default;
}

@media (max-width: 767px) {
  .backdrop { align-items: flex-start; padding-top: calc(env(safe-area-inset-top) + 80px); }
  .modal { width: calc(100% - 32px); }
}
</style>
