import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  // Served under /store/ by the gateway.
  base: '/store/',
  plugins: [vue()],
  resolve: {
    // The shared look (ui/) lives outside this app; its imports of vue resolve from here.
    alias: { '@ui': fileURLToPath(new URL('../ui', import.meta.url)) },
    dedupe: ['vue'],
  },
  server: {
    fs: { allow: ['..'] },
    proxy: {
      '/auth': 'http://localhost:8000',
      '/store/api': { target: 'http://localhost:8001', rewrite: p => p.replace(/^\/store/, '') },
    },
  },
})
