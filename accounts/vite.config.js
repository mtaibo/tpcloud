import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  // Served under /accounts/ by the gateway.
  base: '/accounts/',
  plugins: [vue()],
  server: {
    proxy: {
      '/auth': 'http://localhost:8000',
    },
  },
})
