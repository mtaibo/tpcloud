import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  // Served under /share/ by the gateway.
  base: '/share/',
  plugins: [vue()],
  server: {
    proxy: {
      '/share/api': { target: 'http://localhost:8001', rewrite: p => p.replace(/^\/share/, '') },
    },
  },
})
