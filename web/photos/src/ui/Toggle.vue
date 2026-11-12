<script setup>
const model = defineModel({ type: Boolean, default: false })
defineProps({ label: { type: String, default: '' }, disabled: { type: Boolean, default: false } })
</script>

<template>
  <span class="toggle" :class="{ on: model, disabled }">
    <input v-model="model" type="checkbox" role="switch" :aria-label="label" :disabled="disabled" />
    <span class="knob" />
  </span>
</template>
