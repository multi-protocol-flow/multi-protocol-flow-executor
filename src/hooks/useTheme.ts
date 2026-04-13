/**
 * useTheme - 主题切换 Hook
 * 支持三态切换（浅色/暗黑/跟随系统），默认跟随系统
 */
import { useEffect, useState } from 'react';
import { useSettingsStore } from '@/stores/settings';
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
