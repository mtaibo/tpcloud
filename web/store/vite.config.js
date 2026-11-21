import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import pdfAssets from '../ui/pdfAssets.js'

export default defineConfig({
  // Served under /store/ by the gateway.
  base: '/store/',
  plugins: [vue(), pdfAssets()],
  resolve: {
    // The shared look (ui/) lives outside this app; its imports of vue and pdfjs-dist resolve from here.
    alias: { '@ui': fileURLToPath(new URL('../ui', import.meta.url)) },
    dedupe: ['vue', 'pdfjs-dist'],
  },
  server: {
    fs: { allow: ['..'] },
    proxy: {
      '/auth': 'http://localhost:8000',
      '/store/api': { target: 'http://localhost:8001', rewrite: p => p.replace(/^\/store/, '') },
    },
  },
})
