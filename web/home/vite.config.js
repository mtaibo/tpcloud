import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
  // Served under /home/ by tpcloud's gateway.
  base: '/home/',
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  server: {
    proxy: {
      '/home/api': {
        target: 'http://tp.home',
        changeOrigin: true,
        ws: true,
      },
    },
  },
})
