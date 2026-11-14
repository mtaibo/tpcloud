import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  // Served under /photos/ by the gateway. Its data comes from tpstore's API at /store/api.
  base: '/photos/',
  plugins: [vue()],
  server: {
    proxy: {
      '/auth': 'http://localhost:8000',
      '/store/api': { target: 'http://localhost:8001', rewrite: p => p.replace(/^\/store/, '') },
    },
  },
})
