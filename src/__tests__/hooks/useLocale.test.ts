/**
 * useCurrentLocale Hook 测试（对应 src/hooks/useLocale.ts）
 * 覆盖：无存储默认 zh-CN、读取已存储 locale、localStorage 抛错时静默降级
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useCurrentLocale } from '@/hooks/useLocale';

describe('useCurrentLocale', () => {
  afterEach(() => {
    localStorage.removeItem('locale');
    vi.restoreAllMocks();
  });

  it('无存储设置时默认返回 zh-CN', () => {
    localStorage.removeItem('locale');
    const { result } = renderHook(() => useCurrentLocale());
    expect(result.current).toBe('zh-CN');
  });

  it('读取 localStorage 中已存储的 locale', () => {
    localStorage.setItem('locale', 'en-US');
    const { result } = renderHook(() => useCurrentLocale());
    expect(result.current).toBe('en-US');
  });

  it('localStorage 抛错时静默降级为 zh-CN', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('localStorage 不可用');
    });
    const { result } = renderHook(() => useCurrentLocale());
    expect(result.current).toBe('zh-CN');
  });
});
