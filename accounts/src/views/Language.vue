<script setup>
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import Icon from '@ui/Icon.vue'
import { api } from '@ui/api.js'
import { toast } from '@ui/dialog.js'
import { lang, setLang, t } from '@ui/i18n.js'
import Page from '../components/Page.vue'
import { store } from '../store.js'

// Like Settings → General → Language: each language in its own name, a check on the chosen one.
// It changes here at once and is kept on the account, so every device and module follows.
const languages = [
  { code: 'en', name: 'English', native: 'English' },
  { code: 'es', name: 'Spanish', native: 'Español' },
]

async function choose(code) {
  if (code === lang.value) return
  const before = lang.value
  setLang(code)
  try {
    await api('/auth/account/profile', { method: 'PATCH', body: { language: code } })
    await store.reload()
  } catch (err) {
    setLang(before)
    toast(err.message, 'warning')
  }
}
</script>

<template>
  <Page :title="t('Language', 'Idioma')">
    <ListSection :footer="t('TPCloud, Files, Photos and Home use this language. The sign-in and share pages are always in English.', 'TPCloud, Archivos, Fotos y Casa usan este idioma. Las páginas de inicio de sesión y de enlaces compartidos están siempre en inglés.')">
      <ListRow
        v-for="l in languages"
        :key="l.code"
        :title="l.native"
        :subtitle="t(l.name, l.code === 'es' ? 'Español' : 'Inglés') === l.native ? '' : t(l.name, l.code === 'es' ? 'Español' : 'Inglés')"
        clickable
        @click="choose(l.code)"
      >
        <template #trailing>
          <Icon v-if="lang === l.code" name="checkmark" :size="20" :weight="2.4" class="tint" />
        </template>
      </ListRow>
    </ListSection>
  </Page>
</template>
