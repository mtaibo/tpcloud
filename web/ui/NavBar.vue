<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'

// A navigation bar like a large-title NavigationStack: the title sits big in the page and moves
// into the bar, over a soft blur, once it scrolls away. `inline` keeps it small from the start.
const props = defineProps({
  title: { type: String, default: '' },
  inline: { type: Boolean, default: false },
})

const scrolled = ref(false)
function onScroll() { scrolled.value = window.scrollY > (props.inline ? 2 : 38) }
onMounted(() => { onScroll(); window.addEventListener('scroll', onScroll, { passive: true }) })
onBeforeUnmount(() => window.removeEventListener('scroll', onScroll))
</script>

<template>
  <header class="navbar" :class="{ scrolled, inline }">
    <div class="navbar-leading"><slot name="leading" /></div>
    <div class="navbar-title">{{ title }}</div>
    <div class="navbar-trailing"><slot name="trailing" /></div>
  </header>
</template>
