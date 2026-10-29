import { fileURLToPath, URL } from 'node:url'

import vue from '@vitejs/plugin-vue'
import { ElementPlusResolver } from 'unplugin-vue-components/resolvers'
import Components from 'unplugin-vue-components/vite'
import { defineConfig } from 'vite'

export default defineConfig({
  root: 'web',
  plugins: [
    vue(),
    // Element Plus 组件按需引入：模板里用到哪个才打包哪个组件及其样式
    Components({ resolvers: [ElementPlusResolver()], dts: false }),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./web/src', import.meta.url)),
    },
  },
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'chrome110',
  },
})
