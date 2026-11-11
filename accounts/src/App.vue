<script setup>
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import AlertHost from '@ui/AlertHost.vue'
import Logo from '@ui/Logo.vue'
import Spinner from '@ui/Spinner.vue'
import Menu from './components/Menu.vue'
import { store, wide } from './store.js'

const route = useRoute()
const router = useRouter()
const failed = ref('')

const bare = computed(() => route.meta.bare)
const ready = computed(() => store.profile || (bare.value && store.me))

// On a wide screen the menu is always on the left, so the root shows the profile next to it.
watch([wide, () => route.path], ([isWide, path]) => {
  if (isWide && path === '/') router.replace('/profile')
}, { immediate: true })

watch([() => route.meta.admin, () => store.me], ([admin, me]) => {
  if (admin && me && !me.is_admin) router.replace('/')
})

onMounted(async () => {
  if (window.location.pathname === '/accounts/' || window.location.pathname === '/accounts') {
    window.history.replaceState(window.history.state, '', '/accounts')
  }
  try {
    await store.load()
  } catch (e) {
    if (e.status !== 401) failed.value = e.message
  }
})
</script>

<template>
  <div v-if="!ready" class="splash">
    <Spinner v-if="!failed" :size="26" />
    <p v-else class="secondary">{{ failed }}</p>
  </div>

  <router-view v-else-if="bare" />

  <div v-else class="split" :class="{ wide }">
    <aside v-if="wide" class="sidebar">
      <a href="/" class="brand">
        <Logo :size="30" />
        <span class="headline">TPCloud</span>
      </a>
      <Menu />
    </aside>
    <main class="detail">
      <router-view v-slot="{ Component, route: r }">
        <Transition name="push" mode="out-in">
          <component :is="Component" :key="r.path" />
        </Transition>
      </router-view>
    </main>
  </div>

  <AlertHost />
</template>

<style scoped>
.splash { min-height: 100dvh; display: grid; place-items: center; }
.split.wide { display: grid; grid-template-columns: 340px 1fr; min-height: 100dvh; }
.sidebar {
  position: sticky;
  top: 0;
  height: 100dvh;
  overflow-y: auto;
  padding: 18px 16px 24px;
  border-right: 0.5px solid var(--separator);
}
.brand { display: flex; align-items: center; gap: 10px; padding: 6px 8px 22px; }
.brand :deep(.logo) { box-shadow: none; }
.detail { min-width: 0; }
.split.wide .detail :deep(.page) { padding-top: 4px; }
</style>
