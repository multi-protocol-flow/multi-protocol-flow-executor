/**
 * useTheme Hook 测试
 * 覆盖：默认 system 模式解析主题、store 主题变更后 DOM class/color-scheme 更新、
 * setTheme 持久化并更新 store、system 模式监听系统主题变化
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import { useTheme } from '@/hooks/useTheme';
import { useSettingsStore } from '@/stores/settings';

const originalMatchMedia = window.matchMedia;

/** 可控制的 matchMedia mock：捕获 change 监听器并可切换系统暗色偏好 */
function installMatchMedia(initialDark: boolean) {
  let systemDark = initialDark;
  const listeners: Array<() => void> = [];
  const mediaQueryList = {
    matches: false,
    media: '(prefers-color-scheme: dark)',
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn((_type: string, cb: () => void) => {
      listeners.push(cb);
    }),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(() => false),
  };
  const matchMediaMock = vi.fn((query: string) => {
    mediaQueryList.media = query;
    mediaQueryList.matches = systemDark;
    return mediaQueryList;
  });
  window.matchMedia = matchMediaMock as unknown as typeof window.matchMedia;

  return {
    setSystemDark: (dark: boolean) => {
      systemDark = dark;
    },
    fireChange: () => {
      listeners.forEach((cb) => cb());
    },
  };
}

describe('useTheme', () => {
  beforeEach(() => {
    useSettingsStore.setState({ theme: 'system' });
    document.documentElement.classList.remove('dark');
    document.documentElement.style.colorScheme = '';
  });

  afterEach(() => {
    window.matchMedia = originalMatchMedia;
    document.documentElement.classList.remove('dark');
    document.documentElement.style.colorScheme = '';
  });

  it('默认 system 模式解析为系统主题（浅色）', () => {
    const { result } = renderHook(() => useTheme());

    expect(result.current.theme).toBe('system');
    expect(result.current.resolvedTheme).toBe('light');
    expect(document.documentElement.classList.contains('dark')).toBe(false);
    expect(document.documentElement.style.colorScheme).toBe('light');
  });

  it('store 主题变更为 dark 时更新 DOM 与 resolvedTheme', () => {
    const { result } = renderHook(() => useTheme());

    act(() => {
      useSettingsStore.setState({ theme: 'dark' });
    });

    expect(result.current.resolvedTheme).toBe('dark');
    expect(document.documentElement.classList.contains('dark')).toBe(true);
    expect(document.documentElement.style.colorScheme).toBe('dark');
  });

  it('setTheme 更新 store 并应用主题', async () => {
    const { result } = renderHook(() => useTheme());

    await act(async () => {
      await result.current.setTheme('dark');
    });

    expect(useSettingsStore.getState().theme).toBe('dark');
    expect(result.current.theme).toBe('dark');
    expect(result.current.resolvedTheme).toBe('dark');
    expect(document.documentElement.classList.contains('dark')).toBe(true);
  });

  it('system 模式下响应系统主题变化', () => {
    const { setSystemDark, fireChange } = installMatchMedia(false);
    const { result } = renderHook(() => useTheme());
    expect(result.current.resolvedTheme).toBe('light');

    // 系统切到暗色 → resolvedTheme 变 dark，DOM 加上 dark class
    act(() => {
      setSystemDark(true);
      fireChange();
    });
    expect(result.current.resolvedTheme).toBe('dark');
    expect(document.documentElement.classList.contains('dark')).toBe(true);

    // 系统切回浅色 → 恢复 light
    act(() => {
      setSystemDark(false);
      fireChange();
    });
    expect(result.current.resolvedTheme).toBe('light');
    expect(document.documentElement.classList.contains('dark')).toBe(false);
  });
});
