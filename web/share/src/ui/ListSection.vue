<script setup>
// An inset grouped section: an optional header, the rounded card of rows, and a footer.
defineProps({
  header: { type: String, default: '' },
  footer: { type: String, default: '' },
  icons: { type: Boolean, default: false },
})
</script>

<template>
  <section class="section">
    <div v-if="header || $slots.header || $slots.action" class="section-header">
      <span><slot name="header">{{ header }}</slot></span>
      <slot name="action" />
    </div>
    <div class="card" :class="{ icons }"><slot /></div>
    <p v-if="footer || $slots.footer" class="section-footer"><slot name="footer">{{ footer }}</slot></p>
  </section>
</template>
