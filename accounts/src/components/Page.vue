<script setup>
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import Icon from '@ui/Icon.vue'
import NavBar from '@ui/NavBar.vue'
import { t } from '@ui/i18n.js'
import { wide } from '../store.js'

// A pushed screen: the bar with its back button, then the large title.
const props = defineProps({
  title: { type: String, required: true },
  subtitle: { type: String, default: '' },
  root: { type: Boolean, default: false },
})

const route = useRoute()
const router = useRouter()
const parent = computed(() => route.meta.parent || '/')
const showBack = computed(() => !props.root && (route.meta.parent || !wide.value))

function back() {
  if (window.history.state?.back) router.back()
  else router.push(parent.value)
}
</script>

<template>
  <div>
    <NavBar :title="title">
      <template #leading>
        <button v-if="showBack" class="icon-btn" type="button" :aria-label="t('Back', 'Atrás')" @click="back">
          <Icon name="chevron-left" :size="22" :weight="2.4" />
        </button>
      </template>
      <template #trailing><slot name="toolbar" /></template>
    </NavBar>
    <div class="page">
      <header class="page-header">
        <slot name="header">
          <h1 class="large-title">{{ title }}</h1>
          <p v-if="subtitle" class="subheadline secondary">{{ subtitle }}</p>
        </slot>
      </header>
      <slot />
    </div>
  </div>
</template>
