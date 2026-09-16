<script setup>
import { ref, computed, onMounted } from 'vue'
import { FolderPlus, FilePlus } from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const props = defineProps({ type: String })
const emit = defineEmits(['close', 'created'])

const inputRef = ref(null)
const name = ref('')

const isFolder = computed(() => props.type === 'folder')
const title = computed(() => isFolder.value ? 'New Folder' : 'New File')
const placeholder = computed(() => isFolder.value ? 'Folder name' : 'File name')
const isValid = computed(() => name.value.trim().length > 0)

onMounted(() => inputRef.value?.focus())

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
  <BaseModal @close="emit('close')">
    <div class="modal-header">
      <component :is="isFolder ? FolderPlus : FilePlus" class="entry-icon" style="color: #007AFF" />
      <span class="modal-title">{{ title }}</span>
    </div>

    <div class="modal-body">
      <input
        ref="inputRef"
        v-model="name"
        class="modal-input"
        type="text"
        :placeholder="placeholder"
        spellcheck="false"
        autocomplete="off"
        @keydown="onKeydown"
      />
    </div>

    <div class="modal-footer">
      <button class="btn-cancel" @click="emit('close')">Cancel</button>
      <button class="btn-primary" :disabled="!isValid" @click="submit">Create</button>
    </div>
  </BaseModal>
</template>
