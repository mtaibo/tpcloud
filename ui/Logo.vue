<script setup>
// TPCloud's mark on an app-icon plate, same artwork as the iOS app icon (tpapp AppIcon).
defineProps({ size: { type: Number, default: 72 } })
</script>

<template>
  <span class="logo" :style="{ width: `${size}px`, height: `${size}px`, borderRadius: `${size * 0.2237}px` }">
    <!-- Geometry measured from the 1024 px iOS icon, so the web and the app match exactly. -->
    <svg :width="size" :height="size" viewBox="0 0 1024 1024" aria-hidden="true">
      <rect x="224" y="272" width="576" height="128" rx="64" fill="currentColor" />
      <circle cx="512" cy="592" r="96" fill="currentColor" />
      <circle cx="512" cy="800" r="70" fill="currentColor" />
    </svg>
  </span>
</template>

<style scoped>
.logo {
  display: inline-grid;
  place-items: center;
  flex-shrink: 0;
  overflow: hidden;
  color: #000;
  background: #fff;
  box-shadow:
    0 10px 30px rgba(0, 0, 0, 0.12),
    0 1px 3px rgba(0, 0, 0, 0.08),
    inset 0 0 0 0.5px rgba(0, 0, 0, 0.12);
}
@media (prefers-color-scheme: dark) {
  .logo {
    box-shadow:
      0 14px 36px rgba(0, 0, 0, 0.6),
      inset 0 0 0 0.5px rgba(255, 255, 255, 0.2);
  }
}
</style>
