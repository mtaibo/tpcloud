<script setup>
const model = defineModel({ type: Boolean, default: false })
defineProps({ label: { type: String, default: '' }, disabled: { type: Boolean, default: false } })
</script>

<template>
  <!-- A label, so a click anywhere on the switch flips it (Safari keeps a bare checkbox at its own small size). -->
  <label class="toggle" :class="{ on: model, disabled }">
    <input v-model="model" type="checkbox" role="switch" :aria-label="label" :disabled="disabled" />
    <span class="knob" />
  </label>
</template>
