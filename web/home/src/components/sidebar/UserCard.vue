<script setup>
    import { User } from 'lucide-vue-next'
</script>

<template>
    <div class="w-full flex items-center px-4 py-2.5 rounded-lg">
        <div class="w-9 h-9 rounded-full bg-tp-surface flex items-center justify-center shrink-0">
            <User class="w-[15px] h-[15px] text-tp-accent" />
        </div>
        <span class="ml-4 text-white text-sm font-medium whitespace-nowrap overflow-hidden">
            Miguel Taibo
        </span>
    </div>
</template>
