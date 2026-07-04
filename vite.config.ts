import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const isTauri = mode === 'tauri'

  return {
    plugins: [react(), tailwindcss()],
    optimizeDeps: {
      // 限制 dep-scan 范围，避免扫到 src-tauri/target/ 下数万 Rust 产物触发 EMFILE
      entries: ['src/**/*.{ts,tsx,mts,cts}'],
    },
    server: {
      host: '0.0.0.0',
      port: 5173,
      strictPort: true,
      watch: {
        ignored: [
          '**/src-tauri/target/**',
          '**/.git/**',
          '**/node_modules/**',
        ],
      },
      fs: {
        // 只允许提供 src/ 和 public/ 下的文件
        allow: ['.', 'src', 'public'],
      },
    },
    resolve: {
      alias: {
        '@': path.resolve(__dirname, './src'),
      },
    },
    // 不使用 define: { __TAURI__ }，因为 @tauri-apps/api/core 包内部也引用 __TAURI__
    // 全局变量，define 替换会破坏包内部逻辑。运行时靠 isTauriEnv() 检测即可。
    base: isTauri ? '/' : './',
    build: {
      outDir: isTauri ? 'dist' : 'website/playground',
    },
  }
})
