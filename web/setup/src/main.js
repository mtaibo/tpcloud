import { createApp } from 'vue'
import '@ui/styles.css'
import { lockLang } from '@ui/i18n.js'
import App from './App.vue'

// Before there's an account there's no language to read: English, like the sign-in page.
lockLang('en')

createApp(App).mount('#app')
