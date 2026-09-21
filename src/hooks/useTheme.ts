/**
 * useTheme - 主题切换 Hook
 * 支持三态切换（浅色/暗黑/跟随系统），默认跟随系统
 */
import { useEffect, useState } from 'react';
import { useSettingsStore } from '@/stores/settings';
import { isTauriEnv } from '@/lib/platform';
import type { ThemeMode } from '@/types/settings';

/**
 * 获取系统主题偏好
 */
export function getSystemTheme(): 'light' | 'dark' {
  if (typeof window === 'undefined') return 'light';
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

/**
 * 应用主题到 DOM
 * @param mode - 主题模式
 * @returns 实际应用的 resolved theme ('light' | 'dark')
 */
export function applyTheme(mode: ThemeMode): 'light' | 'dark' {
  const resolvedTheme = mode === 'system' ? getSystemTheme() : mode;

  const root = document.documentElement;

  if (resolvedTheme === 'dark') {
    root.classList.add('dark');
  } else {
    root.classList.remove('dark');
  }

  // 同步 color-scheme 样式（影响原生 UI 如滚动条）
  root.style.colorScheme = resolvedTheme;

  return resolvedTheme;
}

/**
 * 已同步到 Tauri 窗口的**模式**（非 resolved 值）。
 *
 * 模块级去重：`useTheme()` 在 App / FlowSurface / CodeMirrorViewer / NodeConfigPanel
 * 多处调用，每次主题变更若每个实例都发一次 IPC 就是无谓的重复。记录最后一次
 * 下发的模式，相同则跳过。
 */
let syncedWindowMode: ThemeMode | null = null;

/**
 * 把主题同步到 Tauri 窗口，使 webview 的 `prefers-color-scheme` 等于宿主主题。
 *
 * 这是**兼容杠杆**，不是权威路径：自带前端的插件（agent/grpc/llm）里，存量
 * `dist` 仍以 `@media (prefers-color-scheme)` 取色，宿主显式下发 theme 之前，
 * 该调用能让它们与 webview 原生控件自动跟随宿主，无需等插件重新发版。
 * 权威路径是 iframe 桥的 `init.theme` / `themeChanged`，见
 * docs/plugin-frontend-theme.md。
 *
 * `system` 模式传 `null`（Tauri 语义：恢复跟随系统），否则会把窗口钉死在
 * 某个明暗值上、使系统主题变化不再被感知。
 *
 * 注意：目标值由 **mode 自身**推导，不接收 `resolvedTheme` 状态 —— 后者在
 * `theme` 变更的同一 commit 内仍是上一轮的值，会把过期的明暗值发给窗口，
 * 而模块级去重又会让紧随其后的正确调用被跳过。
 */
function syncWindowTheme(mode: ThemeMode): void {
  if (!isTauriEnv()) return;
  if (syncedWindowMode === mode) return;
  syncedWindowMode = mode;
  void import('@tauri-apps/api/window')
    .then(({ getCurrentWindow }) => getCurrentWindow().setTheme(mode === 'system' ? null : mode))
    .catch(() => {
      // 兼容杠杆：平台不支持或调用失败时不阻塞主题应用（显式通道仍生效）
    });
}

/**
 * 主题 Hook
 *
 * @returns theme: 当前设置的主题模式
 * @returns setTheme: 设置主题（持久化）
 * @returns resolvedTheme: 实际应用的主题（'light' | 'dark'）
 */
export function useTheme() {
  const theme = useSettingsStore((state) => state.theme);
  const setTheme = useSettingsStore((state) => state.setTheme);
  const [resolvedTheme, setResolvedTheme] = useState<'light' | 'dark'>(() =>
    applyTheme(theme)
  );

  // 监听 theme 变化，应用主题
  useEffect(() => {
    const resolved = applyTheme(theme);
    setResolvedTheme(resolved);
  }, [theme]);

  // 同步 Tauri 窗口主题（兼容杠杆，见 syncWindowTheme 注释）
  useEffect(() => {
    syncWindowTheme(theme);
  }, [theme]);

  // 监听系统主题变化（仅在 system 模式下生效）
  useEffect(() => {
    if (theme !== 'system') return;

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    const handleChange = () => {
      const resolved = applyTheme('system');
      setResolvedTheme(resolved);
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, [theme]);

  return {
    theme,
    setTheme,
    resolvedTheme,
  };
}
