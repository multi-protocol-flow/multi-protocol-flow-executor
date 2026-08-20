/**
 * SandboxBanner - 浏览器沙箱模式固定提示横幅
 *
 * 在非 Tauri 环境（浏览器模式）下始终显示警告横幅，
 * 不可关闭，确保用户始终知晓这是模拟环境而非真实桌面应用。
 */
import { isTauriEnv } from '@/lib/platform';
import { Home } from 'lucide-react';

const SANDBOX_WARNINGS = [
  'Flow execution uses simulated data — results are NOT real',
  'Network requests, database queries, and protocol calls are mocked',
  'No data is persisted — all state resets on page reload',
];

export function SandboxBanner() {
  // Tauri 模式下不渲染
  if (isTauriEnv()) {
    return null;
  }

  return (
    <div
      role="alert"
      className="flex items-center justify-between gap-3 bg-amber-500/10 border-b border-amber-500/20 px-4 py-2 text-xs leading-relaxed text-amber-700 dark:text-amber-400"
    >
      <div className="flex items-center gap-3 min-w-0">
        <span className="inline-flex items-center justify-center size-6 rounded-full bg-amber-500/20 text-amber-600 dark:text-amber-400 text-sm font-bold shrink-0">
          !
        </span>
        <div className="flex flex-wrap items-baseline gap-x-1">
          <span className="text-sm font-semibold mr-1">Sandbox Mode</span>
          <span className="text-amber-600/80 dark:text-amber-500/80 mx-1 hidden sm:inline">|</span>
          {SANDBOX_WARNINGS.map((text, i) => (
            <span key={i} className="inline-flex items-center gap-1">
              {i > 0 && <span className="text-amber-500/50 mx-0.5">&bull;</span>}
              {text}
            </span>
          ))}
        </div>
      </div>
      <a
        href="../"
        className="inline-flex items-center gap-1.5 px-3 py-1 rounded-md bg-amber-500/20 hover:bg-amber-500/30 text-amber-800 dark:text-amber-300 font-medium text-xs transition-colors shrink-0 border border-amber-500/30 shadow-xs"
        title="Return to official website"
      >
        <Home className="size-3.5 shrink-0" />
        <span>Official Website</span>
      </a>
    </div>
  );
}
