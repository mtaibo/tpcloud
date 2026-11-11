import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  // Served under /store/ by the gateway.
  base: '/store/',
  plugins: [vue(), tailwindcss()],
  server: {
    proxy: {
      '/auth': 'http://localhost:8000',
      '/api': 'http://localhost:8001',
    },
  },
})
