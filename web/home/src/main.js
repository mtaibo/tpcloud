import { createApp } from 'vue'
import { createPinia } from 'pinia'

import './ui/styles.css'
import App from './App.vue'
import { socket_manager } from './config/socket'

createApp(App).use(createPinia()).mount('#app')

socket_manager.connect()
