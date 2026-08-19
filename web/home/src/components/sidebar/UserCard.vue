<script setup>
    import { ref, onMounted, onUnmounted } from 'vue'
    import { User, Cloud, LogOut } from 'lucide-vue-next'

    const user    = ref(null)
    const menuOpen = ref(false)
    const cardRef  = ref(null)

    onMounted(async () => {
        try {
            const res = await fetch('/auth/passkey/me')
            if (res.ok) user.value = await res.json()
        } catch {}
        document.addEventListener('click', onClickOutside)
    })

    onUnmounted(() => {
        document.removeEventListener('click', onClickOutside)
    })

    function onClickOutside(e) {
        if (cardRef.value && !cardRef.value.contains(e.target)) menuOpen.value = false
    }

    async function logout() {
        await fetch('/auth/passkey/logout', { method: 'POST' })
        window.location.href = 'https://login.migueltaibo.com'
    }
</script>

<template>
    <div ref="cardRef" class="relative">

        <Transition name="fade">
            <div v-if="menuOpen" class="absolute bottom-full left-0 right-0 mb-2 bg-[#1c1c1e] border border-white/10 rounded-xl overflow-hidden shadow-xl">
                <a
                    href="https://cloud.migueltaibo.com"
                    class="flex items-center gap-2.5 px-3 py-2.5 text-sm text-tp-text/70 hover:bg-white/5 hover:text-white transition-colors"
                >
                    <Cloud class="w-4 h-4 shrink-0" />
                    TPCloud
                </a>
                <button
                    @click="logout"
                    class="w-full flex items-center gap-2.5 px-3 py-2.5 text-sm text-tp-text/70 hover:bg-white/5 hover:text-red-400 transition-colors"
                >
                    <LogOut class="w-4 h-4 shrink-0" />
                    Cerrar sesión
                </button>
            </div>
        </Transition>

        <button
            @click="menuOpen = !menuOpen"
            class="w-full flex items-center px-4 py-2.5 rounded-lg hover:bg-white/5 transition-colors cursor-pointer"
        >
            <div class="w-9 h-9 rounded-full bg-tp-surface flex items-center justify-center shrink-0">
                <User class="w-[15px] h-[15px] text-tp-accent" />
            </div>
            <span class="ml-4 text-white text-sm font-medium whitespace-nowrap overflow-hidden text-left">
                {{ user?.display_name ?? '…' }}
            </span>
        </button>

    </div>
</template>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.15s, transform 0.15s; }
.fade-enter-from, .fade-leave-to { opacity: 0; transform: translateY(4px); }
</style>
