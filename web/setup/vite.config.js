import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  // Served at the root of a host that isn't set up yet, by the `tpcloud` program itself.
  base: '/',
  plugins: [vue()],
  resolve: {
    alias: { '@ui': fileURLToPath(new URL('../ui', import.meta.url)) },
    dedupe: ['vue'],
  },
  server: {
    fs: { allow: ['..'] },
    proxy: { '/api': 'http://localhost:8090' },
  },
})
