<script setup>
import { ref, computed, onMounted } from 'vue'
import { getFileIcon, getIconColor } from '../fileTypes.js'
import BaseModal from './BaseModal.vue'

const props = defineProps({ entry: Object })
const emit = defineEmits(['close', 'renamed'])

const inputRef = ref(null)
const newName = ref(props.entry.name)

const fileIcon = computed(() => getFileIcon(props.entry))
const iconColor = computed(() => getIconColor(props.entry))

onMounted(() => {
  if (!inputRef.value) return
  inputRef.value.focus()
  const name = props.entry.name
  const dotIndex = props.entry.type === 'file' ? name.lastIndexOf('.') : -1
  if (dotIndex > 0) inputRef.value.setSelectionRange(0, dotIndex)
  else inputRef.value.select()
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
  <BaseModal @close="emit('close')">
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
        class="modal-input"
        type="text"
        spellcheck="false"
        autocomplete="off"
        @keydown="onKeydown"
      />
    </div>

    <div class="modal-footer">
      <button class="btn-cancel" @click="emit('close')">Cancel</button>
      <button class="btn-primary" :disabled="!isValid" @click="submit">Rename</button>
    </div>
  </BaseModal>
</template>
