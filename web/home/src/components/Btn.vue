<script setup>
defineProps({
    muted:    { type: Boolean, default: false },
    pressing: { type: Boolean, default: false },
})
</script>

<template>
    <button class="btn" :class="{ 'btn-muted': muted, pressing }">
        <slot />
    </button>
</template>

<style scoped>
@keyframes btn-press {
    0%   { transform: scale(1); }
    28%  { transform: scale(0.86); }
    66%  { transform: scale(1.08); }
    100% { transform: scale(1); }
}

.btn {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background: linear-gradient(
        145deg,
        rgba(255, 255, 255, 0.14) 0%,
        rgba(255, 255, 255, 0.07) 50%,
        rgba(255, 255, 255, 0.10) 100%
    );
    backdrop-filter: blur(24px) saturate(200%);
    -webkit-backdrop-filter: blur(24px) saturate(200%);
    border: 0.5px solid rgba(255, 255, 255, 0.22);
    box-shadow:
        inset 0 1px 0 rgba(255, 255, 255, 0.36),
        inset 0 -0.5px 0 rgba(0, 0, 0, 0.18),
        0 4px 12px rgba(0, 0, 0, 0.24),
        0 1px 3px rgba(0, 0, 0, 0.14);
    transition: box-shadow 0.18s ease, background 0.18s ease;
}

.btn:hover {
    background: linear-gradient(
        145deg,
        rgba(255, 255, 255, 0.20) 0%,
        rgba(255, 255, 255, 0.11) 50%,
        rgba(255, 255, 255, 0.16) 100%
    );
    box-shadow:
        inset 0 1px 0 rgba(255, 255, 255, 0.42),
        inset 0 -0.5px 0 rgba(0, 0, 0, 0.18),
        0 6px 18px rgba(0, 0, 0, 0.30),
        0 1px 3px rgba(0, 0, 0, 0.14);
}

.btn-muted {
    background: linear-gradient(
        145deg,
        rgba(255, 255, 255, 0.07) 0%,
        rgba(255, 255, 255, 0.03) 100%
    );
    border-color: rgba(255, 255, 255, 0.11);
    box-shadow:
        inset 0 1px 0 rgba(255, 255, 255, 0.18),
        inset 0 -0.5px 0 rgba(0, 0, 0, 0.12),
        0 2px 8px rgba(0, 0, 0, 0.16);
}

.btn.pressing {
    animation: btn-press 0.42s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
}
</style>
