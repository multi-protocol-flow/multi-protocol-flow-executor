/**
 * App - 应用入口组件
 */
import { useEffect } from 'react';
import { MainLayout } from '@/layouts/MainLayout';
import { useKeyboardShortcuts } from '@/hooks/useKeyboardShortcuts';
import { useSettingsStore } from '@/stores/settings';

function App() {
  // 初始化键盘快捷键
  useKeyboardShortcuts();

  // 应用启动时加载全局设置
  const loadSettings = useSettingsStore((state) => state.loadSettings);
  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

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
