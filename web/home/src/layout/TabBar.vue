<script setup>
import { useSections } from '@/config/sections'

// The phone's floating tab bar (iOS 26), the same as Archivos'.
const { activeSection, allSections } = useSections()
</script>

<template>
    <div class="dock">
        <nav class="tabbar glass">
            <button
                v-for="s in allSections"
                :key="s.id"
                type="button"
                class="tab"
                :class="{ on: activeSection === s.id }"
                @click="activeSection = s.id"
            >
                <component :is="s.icon" :size="23" :stroke-width="activeSection === s.id ? 2.2 : 1.8" />
                <span>{{ s.name }}</span>
            </button>
        </nav>
    </div>
</template>

<style scoped>
.dock {
    position: fixed;
    left: 0;
    right: 0;
    bottom: calc(10px + env(safe-area-inset-bottom));
    z-index: 50;
    display: flex;
    justify-content: center;
    pointer-events: none;
}
@media (min-width: 768px) { .dock { display: none; } }
.tabbar { display: flex; padding: 4px; border-radius: 999px; pointer-events: auto; }
.tab {
    width: 84px;
    height: 56px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    border-radius: 999px;
    font-size: 10px;
    font-weight: 600;
    color: var(--label);
    transition: background 0.25s, color 0.25s;
}
.tab.on { color: var(--accent); background: var(--fill-3); }
.tab:active { transform: scale(0.96); }
@media (max-width: 360px) { .tab { width: 76px; } }
</style>
