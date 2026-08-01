
<script setup>

    defineProps({ title: { type: String, default: '' } })

</script>

<template>

    <header class="fixed top-0 left-0 right-0 z-20 flex items-center px-6 h-24 bg-tp-bg md:hidden">
        <h1 class="text-section-title font-bold tracking-tight leading-none">{{ title }}</h1>
    </header>

</template>
