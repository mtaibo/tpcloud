<script setup>

    import { ChevronRight } from 'lucide-vue-next'

    defineProps({ label: { type: String, required: true }, open: { type: Boolean, default: false } })
    defineEmits(['toggle'])

</script>

<template>

    <div class="group flex items-center justify-between px-2 pt-4 pb-1">
        <p class="text-xs font-semibold text-tp-muted select-none">{{ label }}</p>
        <ChevronRight
            class="w-4 h-4 text-tp-muted transition-all duration-200 opacity-0 group-hover:opacity-100"
            :class="open ? 'rotate-90' : ''"
            @click="$emit('toggle')"
        />
    </div>

</template>
