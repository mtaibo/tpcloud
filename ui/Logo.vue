<script setup>
// TPCloud's mark on an app-icon plate.
defineProps({ size: { type: Number, default: 72 } })
</script>

<template>
  <span class="logo" :style="{ width: `${size}px`, height: `${size}px`, borderRadius: `${size * 0.2237}px` }">
    <svg :width="size * 0.62" :height="size * 0.62" viewBox="0 0 32 32" aria-hidden="true">
      <rect x="7" y="7" width="18" height="4" rx="2" fill="currentColor" />
      <circle cx="16" cy="17.5" r="3" fill="currentColor" />
      <circle cx="16" cy="24.5" r="2.2" fill="currentColor" />
    </svg>
  </span>
</template>

<style scoped>
.logo {
  display: inline-grid;
  place-items: center;
  flex-shrink: 0;
  color: #fff;
  background: linear-gradient(180deg, #2c2c2e, #000);
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.18), inset 0 0.5px 0 rgba(255, 255, 255, 0.25);
}
@media (prefers-color-scheme: dark) {
  .logo { color: #000; background: linear-gradient(180deg, #fff, #d8d8dc); box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5); }
}
</style>
