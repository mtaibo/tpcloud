<script setup>
defineProps({ size: { type: Number, default: 20 } })
</script>

<template>
  <span class="spinner" :style="{ width: `${size}px`, height: `${size}px` }" role="progressbar" aria-label="Cargando">
    <i v-for="n in 8" :key="n" :style="{ transform: `rotate(${(n - 1) * 45}deg)`, animationDelay: `${(n - 9) * 0.1}s` }" />
  </span>
</template>
