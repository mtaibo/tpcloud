<script setup>

    import { ref } from 'vue'

    import { PanelLeftClose, PanelLeftOpen } from 'lucide-vue-next'

    import Btn          from '@/components/Btn.vue'
    import UserCard     from '@/components/sidebar/UserCard.vue'
    import NavButton    from '@/components/sidebar/NavButton.vue'
    import NavSeparator from '@/components/sidebar/NavSeparator.vue'

    import { useSections } from '@/config/sections.js'
    const { activeSection, dashboardSections, settingsSections } = useSections()

    const collapsed = ref(true)
    const settingsOpen = ref(false)
    const setActive = (id) => { activeSection.value = id }

</script>

<template>

    <aside
        class="relative z-20 hidden md:flex md:flex-col shrink-0 transition-[width] duration-500 ease-[cubic-bezier(0.4,0,0.2,1)]"
        :class="collapsed ? 'w-18' : 'w-60'"
    >

        <div class="absolute top-4 right-3 z-20">
            <Btn :pressing="false" @click="collapsed = !collapsed">
                <component :is="collapsed ? PanelLeftOpen : PanelLeftClose" class="w-[18px] h-[18px] text-tp-text/80" />
            </Btn>
        </div>

        <div class="absolute inset-0 overflow-hidden z-10">

            <div
                class="w-60 h-full flex flex-col bg-[#111113] shadow-xl transition-transform duration-500 ease-[cubic-bezier(0.4,0,0.2,1)]"
                :class="collapsed ? '-translate-x-60' : 'translate-x-0'"
            >
                <div class="h-[72px] shrink-0" />

                <nav class="flex-1 p-3 space-y-2 overflow-y-auto">

                    <NavButton
                        v-for="item in dashboardSections"
                        :key="item.id"
                        :icon="item.icon"
                        :label="item.name"
                        :active="activeSection === item.id"
                        :collapsed="false"
                        :nav-item="true"
                        @click="setActive(item.id)"
                    />

                    <NavSeparator label="Configuración" :open="settingsOpen" @toggle="settingsOpen = !settingsOpen" />

                    <template v-if="settingsOpen">
                        <NavButton
                            v-for="item in settingsSections"
                            :key="item.id"
                            :icon="item.icon"
                            :label="item.name"
                            :active="activeSection === item.id"
                            :collapsed="false"
                            :nav-item="true"
                            @click="setActive(item.id)"
                        />
                    </template>

                </nav>

                <footer class="px-4 pt-2 pb-6">
                    <UserCard :collapsed="false" />
                </footer>
            </div>

        </div>

    </aside>

</template>
