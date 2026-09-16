<script setup>
defineProps({
  width: { type: String, default: '320px' },
  maxHeight: { type: String, default: null },
})
const emit = defineEmits(['close'])
</script>

<template>
  <Teleport to="body">
    <div class="modal-backdrop" @click.self="emit('close')">
      <div class="modal-frame" :style="{ width, maxHeight: maxHeight || undefined }">
        <slot />
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-backdrop {
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

.modal-frame {
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

@media (max-width: 767px) {
  .modal-backdrop { align-items: flex-start; padding-top: calc(env(safe-area-inset-top) + 80px); }
  .modal-frame { width: calc(100% - 32px) !important; max-height: none !important; }
}
</style>
