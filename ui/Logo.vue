<script setup>
// TPCloud's mark on an app-icon plate.
defineProps({ size: { type: Number, default: 72 } })
</script>

<template>
  <span class="logo" :style="{ width: `${size}px`, height: `${size}px`, borderRadius: `${size * 0.2237}px` }">
    <!-- Bar, dot, smaller dot: even gaps, the group optically centred in the plate. -->
    <svg :width="size * 0.66" :height="size * 0.66" viewBox="0 0 32 32" aria-hidden="true">
      <rect x="8" y="5.6" width="16" height="4.4" rx="2.2" fill="currentColor" />
      <circle cx="16" cy="16.2" r="3.2" fill="currentColor" />
      <circle cx="16" cy="24.4" r="2.4" fill="currentColor" />
    </svg>
  </span>
</template>

<style scoped>
.logo {
  position: relative;
  display: inline-grid;
  place-items: center;
  flex-shrink: 0;
  color: #fff;
  background: linear-gradient(180deg, #3a3a3c 0%, #1c1c1e 55%, #000 100%);
  box-shadow:
    0 12px 32px rgba(0, 0, 0, 0.2),
    0 2px 6px rgba(0, 0, 0, 0.12),
    inset 0 1px 0 rgba(255, 255, 255, 0.18),
    inset 0 0 0 0.5px rgba(255, 255, 255, 0.08);
}
@media (prefers-color-scheme: dark) {
  .logo {
    color: #000;
    background: linear-gradient(180deg, #ffffff 0%, #f2f2f7 60%, #e3e3e8 100%);
    box-shadow:
      0 14px 36px rgba(0, 0, 0, 0.6),
      inset 0 1px 0 rgba(255, 255, 255, 0.9),
      inset 0 -1px 0 rgba(0, 0, 0, 0.08);
  }
}
</style>
