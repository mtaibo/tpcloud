<script setup>
import { computed } from 'vue'

const model = defineModel({ type: [String, Number], required: true })
const props = defineProps({ options: { type: Array, required: true } }) // [{ value, label }]

const index = computed(() => Math.max(0, props.options.findIndex(o => o.value === model.value)))
const thumb = computed(() => ({
  width: `calc((100% - 6px) / ${props.options.length})`,
  transform: `translateX(${index.value * 100}%)`,
}))
</script>

<template>
  <div class="segmented" role="tablist">
    <span class="thumb" :style="thumb" />
    <button
      v-for="o in options"
      :key="o.value"
      type="button"
      role="tab"
      :aria-selected="o.value === model"
      @click="model = o.value"
    >{{ o.label }}</button>
  </div>
</template>
