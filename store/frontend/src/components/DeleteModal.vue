<script setup>
import { computed } from 'vue'
import { getFileIcon, getIconColor } from '../fileTypes.js'
import BaseModal from './BaseModal.vue'

const props = defineProps({ entry: Object })
const emit = defineEmits(['close', 'confirmed'])

const fileIcon = computed(() => getFileIcon(props.entry))
const iconColor = computed(() => getIconColor(props.entry))
</script>

<template>
  <BaseModal @close="emit('close')">
    <div class="modal-header">
      <component :is="fileIcon" class="entry-icon" :style="{ color: iconColor }" />
      <div class="header-text">
        <span class="modal-title">Delete</span>
        <span class="modal-subtitle">{{ entry.name }}</span>
      </div>
    </div>

    <div class="modal-body">
      <p class="warning-text">This action cannot be undone.</p>
    </div>

    <div class="modal-footer">
      <button class="btn-cancel" @click="emit('close')">Cancel</button>
      <button class="btn-danger" @click="emit('confirmed')">Delete</button>
    </div>
  </BaseModal>
</template>

<style scoped>
.warning-text {
  font-size: 13px;
  color: #636366;
}
</style>
