/**
 * App - 应用入口组件
 */
import { useEffect, useState } from 'react';
import { MainLayout } from '@/layouts/MainLayout';
import { useKeyboardShortcuts } from '@/hooks/useKeyboardShortcuts';
import { useTheme } from '@/hooks/useTheme';
import { useSettingsStore } from '@/stores/settings';
import { useDebugStore } from '@/stores/debug';
import { useFlowStore } from '@/stores/flow';
import { useZoomStore } from '@/stores/zoom';
import { get_debug_status, force_end_debug } from '@/services/tauri';
import { TypesafeI18n, switchLocale } from '@/i18n';
import { useI18nContext } from '@/i18n/i18n-react';
import type { Locales } from '@/i18n';

// 检测是否在 Tauri 环境中
function isTauriEnv(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

/**
 * I18nBridge - 当 locale 变化时强制整个子树重渲染
 *
 * 绝大多数组件通过 getLL() 获取翻译，它是一个模块级函数，不参与 React 响应式。
 * 本组件通过接收 locale prop 来触发自身重渲染 → 子孙组件全部重渲染 → getLL() 重新计算。
 */
function I18nBridge({ children }: { children: React.ReactNode }) {
  // 使用 useI18nContext 订阅 context 变化，确保 TypesafeI18n 切换 locale 时重渲染
  useI18nContext();
  return <>{children}</>;
}

function App() {
  // 当前语言区域状态
  const [currentLocale, setCurrentLocale] = useState<Locales>('zh-CN');

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

  // 订阅 store 中的 locale 变化，同步到 TypesafeI18n context
  const storeLocale = useSettingsStore((state) => state.locale);
  useEffect(() => {
    switchLocale(storeLocale as Locales);
    setCurrentLocale(storeLocale as Locales);
  }, [storeLocale]);

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

  return (
    <TypesafeI18n locale={currentLocale}>
      <I18nBridge>
        <MainLayout />
      </I18nBridge>
    </TypesafeI18n>
  );
}

export default App;
