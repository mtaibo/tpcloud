<script setup>
import { t } from '@ui/i18n.js'
import Page from '../components/Page.vue'
import Menu from '../components/Menu.vue'
</script>

<template>
  <Page :title="t('Account', 'Cuenta')" root>
    <Menu />
  </Page>
</template>
