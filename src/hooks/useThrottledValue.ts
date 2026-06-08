/**
 * useThrottledValue - 带最大等待时间的节流 hook
 *
 * 与 useDeferredValue 的区别：
 * - useDeferredValue：永远延迟到主线程空闲才更新，密集场景下可能永久不更新
 * - useThrottledValue：在 intervalMs 内至少更新一次，保证面板响应性
 *
 * @param value - 原始值
 * @param intervalMs - 节流间隔（默认 80ms，约 12fps 更新速率）
 * @returns 节流后的值
 */
import { useState, useEffect, useRef } from 'react';

export function useThrottledValue<T>(value: T, intervalMs: number = 80): T {
  const [throttled, setThrottled] = useState(value);
  const lastRef = useRef(0);
  const valueRef = useRef(value);

  // 始终持有最新值
  valueRef.current = value;

  useEffect(() => {
    const now = Date.now();
    const elapsed = now - lastRef.current;

    if (elapsed >= intervalMs) {
      // 距上次更新已超过间隔 → 立即更新
      lastRef.current = now;
      setThrottled(value);
    } else {
      // 距上次更新不足间隔 → 延时到间隔期满后更新（保证 maxWait）
      const remaining = intervalMs - elapsed;
      const timer = setTimeout(() => {
        lastRef.current = Date.now();
        setThrottled(valueRef.current);
      }, remaining);
      return () => clearTimeout(timer);
    }
  }, [value, intervalMs]);

  return throttled;
}
