import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  // Served under /login/ by the gateway.
  base: '/login/',
  plugins: [vue()],
  envDir: '../',
  server: {
    proxy: {
      '/auth': 'http://localhost:8000',
    },
  },
})
