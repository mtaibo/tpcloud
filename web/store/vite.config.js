import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  // Served under /store/ by the gateway.
  base: '/store/',
  plugins: [vue()],
  server: {
    proxy: {
      '/auth': 'http://localhost:8000',
      '/store/api': { target: 'http://localhost:8001', rewrite: p => p.replace(/^\/store/, '') },
    },
  },
})
