/**
 * App - 应用入口组件
 */
import { MainLayout } from '@/layouts/MainLayout';
import { useKeyboardShortcuts } from '@/hooks/useKeyboardShortcuts';

function App() {
  // 初始化键盘快捷键
  useKeyboardShortcuts();

  return <MainLayout />;
}

export default App;
