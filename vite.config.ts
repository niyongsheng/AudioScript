import { resolve } from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [react(), tailwindcss()],

  resolve: {
    alias: { '@': resolve(import.meta.dirname, './src') },
  },

  clearScreen: false,
  server: {
    port: 5183,
    // ⚠️ 端口被占用时直接失败，不要静默换端口：tauri.conf.json 的 devUrl 写死 5183，
    // 换端口会连到别的进程上，表现为白屏且无报错。
    strictPort: true,
  },

  build: {
    // ⚠️ 两个 HTML 入口都必须注册。漏注册只在生产构建暴露且**不报错**：
    // dist/ 里只有 index.html，悬浮窗加载 overlay.html 时回退显示主界面；
    // 而 `pnpm tauri dev` 下一切正常，所以必须有一次 `cargo tauri build` 才算验过。
    rollupOptions: {
      input: {
        main: resolve(import.meta.dirname, 'index.html'),
        overlay: resolve(import.meta.dirname, 'overlay.html'),
      },
    },
  },
})
