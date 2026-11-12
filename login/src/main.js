import { createApp } from 'vue'
import '@ui/styles.css'
import { lockLang } from '@ui/i18n.js'
import App from './App.vue'

// The sign-in page is English only, whatever the account chose for the rest.
lockLang('en')

createApp(App).mount('#app')
