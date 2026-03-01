/**
 * App - 应用入口组件
 */
import { useEffect } from 'react';
import { MainLayout } from '@/layouts/MainLayout';
import { useKeyboardShortcuts } from '@/hooks/useKeyboardShortcuts';

function App() {
  // 初始化键盘快捷键
  useKeyboardShortcuts();

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
