<script setup>

    import { computed } from 'vue'

    const props = defineProps({
        icon:      { type: Object,  required: true },
        label:     { type: String,  required: true },
        active:    { type: Boolean, default: false },
        collapsed: { type: Boolean, default: false },
        navItem:   { type: Boolean, default: false }
    })

    defineEmits(['click'])

    const buttonClass = computed(() => [
        'w-full flex items-center py-2.5 rounded-lg transition-[background-color,padding] duration-300',
        props.collapsed ? (props.navItem ? 'px-[10px]' : 'px-[6px]') : 'px-4',
        props.navItem && props.active ? 'bg-tp-surface' : 'bg-transparent',
    ].join(' '))

</script>

<template>

    <button
        :class="buttonClass"
        :title="collapsed ? label : ''"
        @click="$emit('click')"
    >
        <component :is="icon" class="w-5 h-5 shrink-0 text-tp-accent" />

        <span
            class="text-white text-sm whitespace-nowrap overflow-hidden transition-[width,opacity] duration-300"
            :class="[collapsed ? 'w-0 opacity-0' : 'opacity-100 ml-4', navItem && active ? 'font-bold' : 'font-medium']"
        >
            {{ label }}
        </span>
    </button>

</template>
