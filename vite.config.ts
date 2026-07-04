import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const isTauri = mode === 'tauri'

  return {
    plugins: [react(), tailwindcss()],
    server: {
      host: '0.0.0.0',
      port: 5173,
      strictPort: true,
      watch: {
        ignored: ['**/src-tauri/target/**'],
      },
    },
    resolve: {
      alias: {
        '@': path.resolve(__dirname, './src'),
        ...(isTauri
          ? {}
          : {
              '@tauri-apps/api/core': path.resolve(__dirname, 'src/browser-shims/api-core.ts'),
              '@tauri-apps/api/event': path.resolve(__dirname, 'src/browser-shims/api-event.ts'),
              '@tauri-apps/plugin-dialog': path.resolve(__dirname, 'src/browser-shims/plugin-dialog.ts'),
              '@tauri-apps/plugin-opener': path.resolve(__dirname, 'src/browser-shims/plugin-opener.ts'),
              '@tauri-apps/plugin-store': path.resolve(__dirname, 'src/browser-shims/plugin-store.ts'),
              '@tauri-apps/plugin-updater': path.resolve(__dirname, 'src/browser-shims/plugin-updater.ts'),
              '@tauri-apps/plugin-process': path.resolve(__dirname, 'src/browser-shims/plugin-process.ts'),
              '@tauri-apps/api/webview': path.resolve(__dirname, 'src/browser-shims/api-webview.ts'),
              '@tauri-apps/api/window': path.resolve(__dirname, 'src/browser-shims/api-window.ts'),
            }),
      },
    },
    define: {
      __TAURI__: isTauri ? JSON.stringify(true) : JSON.stringify(false),
    },
    base: isTauri ? '/' : './',
    build: {
      outDir: isTauri ? 'dist' : 'website/playground',
    },
  }
})
