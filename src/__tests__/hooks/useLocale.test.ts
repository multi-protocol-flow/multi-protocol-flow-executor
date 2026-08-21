/**
 * useCurrentLocale Hook 测试（对应 src/hooks/useLocale.ts）
 * 覆盖：无存储默认 en-US、读取已存储 locale、localStorage 抛错时静默降级
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useCurrentLocale } from '@/hooks/useLocale';

describe('useCurrentLocale', () => {
  afterEach(() => {
    localStorage.removeItem('locale');
    vi.restoreAllMocks();
  });

  it('无存储设置时默认返回 en-US', () => {
    localStorage.removeItem('locale');
    const { result } = renderHook(() => useCurrentLocale());
    expect(result.current).toBe('en-US');
  });

  it('读取 localStorage 中已存储的 locale', () => {
    localStorage.setItem('locale', 'zh-CN');
    const { result } = renderHook(() => useCurrentLocale());
    expect(result.current).toBe('zh-CN');
  });

  it('localStorage 抛错时静默降级为 en-US', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('localStorage 不可用');
    });
    const { result } = renderHook(() => useCurrentLocale());
    expect(result.current).toBe('en-US');
  });
});
