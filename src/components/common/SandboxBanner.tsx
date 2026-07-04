/**
 * SandboxBanner - 浏览器沙箱模式提示横幅
 *
 * 在非 Tauri 环境（浏览器模式）下显示警告横幅，
 * 提示用户后端执行不可用，部分功能将使用模拟数据。
 * 支持 localStorage 持久化关闭状态。
 */
import { useState, useEffect } from 'react';
import { X } from 'lucide-react';
import { isTauriEnv } from '@/lib/platform';

const STORAGE_KEY = 'sandbox-banner-dismissed';

export function SandboxBanner() {
  const [dismissed, setDismissed] = useState(false);

  // 初始化：从 localStorage 读取关闭状态
  useEffect(() => {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored === 'true') {
        setDismissed(true);
      }
    } catch {
      // localStorage 不可用时静默失败
    }
  }, []);

  // Tauri 模式下不渲染
  if (isTauriEnv()) {
    return null;
  }

  // 已关闭则不渲染
  if (dismissed) {
    return null;
  }

  const handleDismiss = () => {
    try {
      localStorage.setItem(STORAGE_KEY, 'true');
    } catch {
      // localStorage 不可用时静默失败
    }
    setDismissed(true);
  };

  return (
    <div
      role="alert"
      className="relative flex items-center justify-between gap-4 bg-amber-500/10 border-b border-amber-500/20 px-4 py-2.5 text-sm text-amber-700 dark:text-amber-400"
    >
      <div className="flex items-center gap-2">
        <span className="inline-flex items-center justify-center size-5 rounded-full bg-amber-500/20 text-amber-600 dark:text-amber-400 text-xs font-bold shrink-0">
          !
        </span>
        <span>
          <strong className="font-semibold">Sandbox Mode</strong>
          {' — '}Backend execution not available. Some features will use mock data.
        </span>
      </div>
      <button
        type="button"
        onClick={handleDismiss}
        className="shrink-0 rounded p-1 text-amber-500/60 hover:text-amber-700 dark:hover:text-amber-300 hover:bg-amber-500/10 transition-colors"
        aria-label="Dismiss"
      >
        <X className="size-4" />
      </button>
    </div>
  );
}
