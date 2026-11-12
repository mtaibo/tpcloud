<script setup>
import { computed } from 'vue'
import { ChevronRight, Cloud, House, LogOut, User } from 'lucide-vue-next'

import { useSections } from '@/config/sections'
import { apiOnline } from '@/config/socket'
import { session, signOut } from '@/config/user'
import { openMenu } from '@/menu.js'
import { t } from '@/ui/i18n.js'

// The Mac-style sidebar, the same as Archivos': the home's screens, its settings, and who's signed in.
const { activeSection, homeSections, settingsSections } = useSections()

const initials = computed(() => (session.user?.display_name || '?').split(/\s+/).slice(0, 2).map(w => w[0]?.toUpperCase()).join(''))

function userMenu(e) {
    openMenu(e.currentTarget, [
        { label: t('Your account', 'Tu cuenta'), icon: User, action: () => { window.location.href = '/accounts' } },
        { label: t('Back to TPCloud', 'Volver a TPCloud'), icon: Cloud, action: () => { window.location.href = '/' } },
        { separator: true },
        { label: t('Log out', 'Cerrar sesión'), icon: LogOut, danger: true, action: signOut },
    ])
}
</script>

<template>
    <aside class="sidebar">
        <a href="/" class="brand" :title="t('Back to TPCloud', 'Volver a TPCloud')">
            <span class="brand-icon"><House :size="15" :stroke-width="2.4" /></span>
            <span>{{ t('Home', 'Casa') }}</span>
        </a>

        <nav class="nav">
            <div class="group-head"><span>{{ t('Home', 'Casa') }}</span></div>
            <button
                v-for="s in homeSections"
                :key="s.id"
                type="button"
                class="item"
                :class="{ active: activeSection === s.id }"
                @click="activeSection = s.id"
            >
                <component :is="s.icon" :size="18" :stroke-width="1.9" class="item-icon" />
                <span class="item-label">{{ s.name }}</span>
            </button>

            <div class="group-head"><span>{{ t('Settings', 'Configuración') }}</span></div>
            <button
                v-for="s in settingsSections"
                :key="s.id"
                type="button"
                class="item"
                :class="{ active: activeSection === s.id }"
                @click="activeSection = s.id"
            >
                <component :is="s.icon" :size="18" :stroke-width="1.9" class="item-icon" />
                <span class="item-label">{{ s.name }}</span>
            </button>
        </nav>

        <p class="status" :class="{ off: !apiOnline }">
            <i />{{ apiOnline ? t('Connected to the house', 'Conectado a la casa') : t('No connection to the house', 'Sin conexión con la casa') }}
        </p>

        <button type="button" class="user" @click="userMenu">
            <span class="avatar" style="width: 30px; height: 30px; font-size: 12px">{{ initials }}</span>
            <span class="user-name">{{ session.user?.display_name }}</span>
            <ChevronRight :size="14" :stroke-width="2.4" class="user-chevron" />
        </button>
    </aside>
</template>

<style scoped>
.sidebar {
    width: 248px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    margin: 8px 0 8px 8px;
    padding: 10px 10px 8px;
    border-radius: 20px;
    background: color-mix(in srgb, var(--cell) 70%, transparent);
    backdrop-filter: blur(30px) saturate(180%);
    -webkit-backdrop-filter: blur(30px) saturate(180%);
    box-shadow: 0 0 0 0.5px var(--separator), 0 8px 30px rgba(0, 0, 0, 0.06);
}
@media (prefers-color-scheme: dark) {
    .sidebar { background: rgba(44, 44, 46, 0.55); box-shadow: 0 0 0 0.5px rgba(255, 255, 255, 0.08); }
}
.brand { display: flex; align-items: center; gap: 8px; padding: 6px 10px 10px; font-size: 14px; font-weight: 700; letter-spacing: -0.2px; color: var(--label); text-decoration: none; }
.brand-icon { width: 24px; height: 24px; display: grid; place-items: center; border-radius: 7px; color: #fff; background: linear-gradient(180deg, #ffb340, #ff9500); }
.nav { flex: 1; overflow-y: auto; }
.group-head { padding: 14px 10px 4px; font-size: 11px; font-weight: 700; color: var(--label-3); }
.item {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    height: 32px;
    padding: 0 10px;
    border-radius: 9px;
    font-size: 13.5px;
    letter-spacing: -0.1px;
    text-align: left;
    color: var(--label);
    cursor: default;
}
.item:hover { background: var(--fill-4); }
.item.active { background: var(--fill-2); font-weight: 600; }
.item-icon { flex-shrink: 0; color: var(--accent); }
.item-label { flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.status { display: flex; align-items: center; gap: 8px; padding: 10px 10px 8px; border-top: 0.5px solid var(--separator); font-size: 11.5px; color: var(--label-2); }
.status i { width: 7px; height: 7px; border-radius: 50%; background: var(--green); box-shadow: 0 0 6px var(--green); }
.status.off i { background: var(--red); box-shadow: none; }
.user { display: flex; align-items: center; gap: 9px; padding: 6px 8px; border-radius: 12px; text-align: left; color: var(--label); }
.user:hover { background: var(--fill-4); }
.user-name { flex: 1; min-width: 0; font-size: 13.5px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.user-chevron { color: var(--label-3); transform: rotate(-90deg); }
</style>
