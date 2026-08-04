import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  envDir: '../',
  server: {
    proxy: {
      '/auth': 'http://localhost:8000',
    },
  },
})
