<script setup>
import { onBeforeUnmount, watch } from 'vue'
import Icon from './Icon.vue'
import Spinner from './Spinner.vue'

// A sheet with the iOS 26 bar: ✕ on the left, the title, and a ✓ that confirms (when the sheet
// has something to confirm). From the bottom on a phone, centred on a wider screen.
const open = defineModel('open', { type: Boolean, default: false })
const props = defineProps({
  title: { type: String, default: '' },
  confirmable: { type: Boolean, default: false },
  canConfirm: { type: Boolean, default: true },
  busy: { type: Boolean, default: false },
})
const emit = defineEmits(['confirm', 'close'])

function close() {
  if (props.busy) return
  open.value = false
  emit('close')
}
function onKey(e) { if (e.key === 'Escape') close() }

watch(open, value => {
  document.documentElement.style.overflow = value ? 'hidden' : ''
  if (value) window.addEventListener('keydown', onKey)
  else window.removeEventListener('keydown', onKey)
}, { immediate: true })
onBeforeUnmount(() => {
  document.documentElement.style.overflow = ''
  window.removeEventListener('keydown', onKey)
})
</script>

<template>
  <Teleport to="body">
    <Transition name="sheet" appear>
      <div v-if="open" class="overlay" @click.self="close">
        <div class="sheet" role="dialog" aria-modal="true" :aria-label="title">
          <div class="sheet-bar">
            <button class="icon-btn" type="button" aria-label="Cerrar" @click="close">
              <Icon name="xmark" :size="20" :weight="2.2" />
            </button>
            <div class="navbar-title">{{ title }}</div>
            <button
              v-if="confirmable"
              class="icon-btn prominent"
              type="button"
              aria-label="Aceptar"
              :disabled="!canConfirm || busy"
              @click="emit('confirm')"
            >
              <Spinner v-if="busy" :size="18" style="color: #fff" />
              <Icon v-else name="checkmark" :size="20" :weight="2.4" />
            </button>
            <span v-else />
          </div>
          <div class="sheet-body"><slot /></div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
