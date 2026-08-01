import { createApp } from 'vue'
import { createPinia } from 'pinia'

import { socket_manager } from './config/socket'

import App from './App.vue'
import './main.css'

const app = createApp(App)
app.use(createPinia())
app.mount('#app')

socket_manager.connect()
