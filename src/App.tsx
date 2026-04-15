/**
 * App - 应用入口组件
 */
import { useEffect } from 'react';
import { MainLayout } from '@/layouts/MainLayout';
import { useKeyboardShortcuts } from '@/hooks/useKeyboardShortcuts';
import { useTheme } from '@/hooks/useTheme';
import { useSettingsStore } from '@/stores/settings';
import { useDebugStore } from '@/stores/debug';
import { useFlowStore } from '@/stores/flow';
import { useZoomStore } from '@/stores/zoom';
import { get_debug_status, force_end_debug } from '@/services/tauri';

// 检测是否在 Tauri 环境中
function isTauriEnv(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

function App() {
  // 初始化键盘快捷键
  useKeyboardShortcuts();

  // 主题管理
  useTheme();

  // 应用启动时加载全局设置
  const loadSettings = useSettingsStore((state) => state.loadSettings);
  useEffect(() => {
    const initApp = async () => {
      await loadSettings();

      // 在 Tauri 环境中，设置加载完成后显示窗口
      if (isTauriEnv()) {
        try {
          const { getCurrentWindow } = await import('@tauri-apps/api/window');
          await getCurrentWindow().show();
        } catch (error) {
          console.debug('[启动] 显示窗口失败:', error);
        }
      }

      // 检查并清理可能残留的调试会话（刷新页面时后端状态未清理）
      try {
        const hasActiveSession = await get_debug_status();

        if (hasActiveSession) {
          console.warn('[调试] 检测到残留的调试会话，自动清理');
          await force_end_debug();
          useDebugStore.getState().reset();
        }
      } catch (error) {
        // 非 Tauri 环境或 API 不存在，忽略
        console.debug('[调试] 跳过调试状态检查:', error);
      }

      // 恢复上次保存的缩放级别
      useZoomStore.getState().applyZoom();
    };

    initApp();
  }, [loadSettings]);

  // 关闭窗口前提示保存（有未保存修改时阻止关闭）
  useEffect(() => {
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      const isDirty = useFlowStore.getState().isDirty;
      if (isDirty) {
        e.preventDefault();
      }
    };
    window.addEventListener('beforeunload', handleBeforeUnload);
    return () => window.removeEventListener('beforeunload', handleBeforeUnload);
  }, []);

  // 禁用非输入框的右键菜单
  useEffect(() => {
    const handleContextMenu = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      const isEditable =
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable;

      if (!isEditable) {
        e.preventDefault();
      }
    };

    document.addEventListener('contextmenu', handleContextMenu);
    return () => document.removeEventListener('contextmenu', handleContextMenu);
  }, []);

  return <MainLayout />;
}

export default App;
