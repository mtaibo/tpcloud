import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
  // Served under /home/ by the gateway.
  base: '/home/',
  plugins: [vue()],
  resolve: {
    // The shared look (ui/) lives outside this app; its imports of vue resolve from here.
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      '@ui': fileURLToPath(new URL('../ui', import.meta.url)),
    },
    dedupe: ['vue'],
  },
  server: {
    fs: { allow: ['..'] },
    proxy: {
      '/home/api': {
        target: 'http://tp.home',
        changeOrigin: true,
        ws: true,
      },
    },
  },
})
