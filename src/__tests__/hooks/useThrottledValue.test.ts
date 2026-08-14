/**
 * useThrottledValue Hook 测试（fake timers）
 * 覆盖：初始值立即返回、间隔内变更延迟到 maxWait 期满后更新、
 * 间隔内多次变更只取最新值、超过间隔的变更立即生效、卸载时清理挂起定时器
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import { useThrottledValue } from '@/hooks/useThrottledValue';

const INTERVAL = 80;

describe('useThrottledValue', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  function renderValue(initial: string) {
    return renderHook(({ value }) => useThrottledValue(value, INTERVAL), {
      initialProps: { value: initial },
    });
  }

  it('初始值立即返回', () => {
    const { result } = renderValue('a');
    expect(result.current).toBe('a');
  });

  it('间隔内变更延迟到 maxWait 期满后更新', () => {
    const { result, rerender } = renderValue('a');
    // 先推进首次 effect 的挂起定时器，记录最近更新时间点
    act(() => {
      vi.advanceTimersByTime(INTERVAL);
    });

    act(() => {
      rerender({ value: 'b' });
    });
    // 尚未到 maxWait → 仍返回旧值
    expect(result.current).toBe('a');

    act(() => {
      vi.advanceTimersByTime(INTERVAL);
    });
    expect(result.current).toBe('b');
  });

  it('间隔内多次变更只取最新值', () => {
    const { result, rerender } = renderValue('a');
    act(() => {
      vi.advanceTimersByTime(INTERVAL);
    });

    act(() => {
      rerender({ value: 'b' });
    });
    act(() => {
      rerender({ value: 'c' });
    });
    act(() => {
      vi.advanceTimersByTime(INTERVAL);
    });

    // 中间值 b 被跳过，只取最新的 c
    expect(result.current).toBe('c');
  });

  it('超过间隔的变更立即生效', () => {
    const { result, rerender } = renderValue('a');
    act(() => {
      vi.advanceTimersByTime(INTERVAL * 2);
    });

    act(() => {
      rerender({ value: 'd' });
    });
    // 距上次更新已超过 interval → 无需等待定时器
    expect(result.current).toBe('d');
  });

  it('卸载时清理挂起的节流定时器', () => {
    const clearTimeoutSpy = vi.spyOn(globalThis, 'clearTimeout');
    const { rerender, unmount } = renderValue('a');
    act(() => {
      vi.advanceTimersByTime(INTERVAL);
    });

    // 产生一个挂起的节流定时器
    act(() => {
      rerender({ value: 'b' });
    });
    unmount();

    expect(clearTimeoutSpy).toHaveBeenCalled();
    clearTimeoutSpy.mockRestore();
  });
});
