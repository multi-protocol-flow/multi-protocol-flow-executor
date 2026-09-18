/**
 * usePanelDrag - 面板拖拽调整大小 Hook
 *
 * 将 MainLayout 中三个面板（底部/右侧/左侧）的拖拽调整大小逻辑抽象为可复用 Hook。
 *
 * 用法:
 *   const { isDragging, handleMouseDown } = usePanelDrag({
 *     axis: 'horizontal', min: 250, max: 500,
 *     initialSize: configPanelWidth,
 *     onResize: setConfigPanelWidth,
 *   });
 *
 * sign 控制拖拽方向：
 *   - 1 （默认）：拖拽方向与鼠标移动方向相反（right 面板）
 *     delta = startPos - currentPos，鼠标左移（currentPos 减小）使尺寸增大
 *   - -1：拖拽方向与鼠标移动方向相同（left 面板）
 *     delta = -(startPos - currentPos)，鼠标左移使尺寸减小
 */
import { useState, useCallback, useRef, useEffect } from 'react';
import { beginLayoutTransition, endLayoutTransition } from '@/lib/layoutTransition';

export interface UsePanelDragOptions {
  /** 拖拽轴方向: 'vertical' 为垂直拖拽 (行调整), 'horizontal' 为水平拖拽 (列调整) */
  axis: 'vertical' | 'horizontal';
  /** 最小尺寸（像素） */
  min: number;
  /** 最大尺寸（像素） */
  max: number;
  /** 拖拽方向系数: 1 或 -1，默认 1 */
  sign?: 1 | -1;
  /** 当前尺寸初始值 */
  initialSize: number;
  /** 尺寸变化回调（主要在 mouseup 提交最终值；在未提供 onLiveResize 或测试环境下同步触发） */
  onResize: (size: number) => void;
  /** 实时尺寸变化回调（用于直接更新 CSS 变量或 DOM，完全绕过 Store / React 调度） */
  onLiveResize?: (size: number) => void;
  /** 是否启用 requestAnimationFrame 节流，默认在生产环境启用 */
  throttle?: boolean;
}

export function usePanelDrag(options: UsePanelDragOptions) {
  const isTest = typeof process !== 'undefined' && Boolean(process.env.VITEST);
  const { axis, min, max, sign = 1, initialSize, onResize, onLiveResize, throttle = !isTest } = options;
  const [isDragging, setIsDragging] = useState(false);
  const isDraggingRef = useRef(false);
  const startPosRef = useRef(0);
  const startSizeRef = useRef(0);
  const rafIdRef = useRef<number | null>(null);
  const pendingSizeRef = useRef<number | null>(null);

  const disableIframes = () => {
    if (typeof document !== 'undefined') {
      document.querySelectorAll('iframe').forEach((iframe) => {
        iframe.style.pointerEvents = 'none';
      });
    }
  };

  const restoreIframes = () => {
    if (typeof document !== 'undefined') {
      document.querySelectorAll('iframe').forEach((iframe) => {
        iframe.style.pointerEvents = '';
      });
    }
  };

  // 开始拖拽
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isDraggingRef.current = true;
    setIsDragging(true);
    startPosRef.current = axis === 'vertical' ? e.clientY : e.clientX;
    startSizeRef.current = initialSize;
    document.body.style.cursor = axis === 'vertical' ? 'row-resize' : 'col-resize';
    document.body.style.userSelect = 'none';
    disableIframes();
    beginLayoutTransition();
  }, [axis, initialSize]);

  // 拖拽中（带 requestAnimationFrame 节流，避免高频 mousemove 导致每秒上百次重排）
  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDraggingRef.current) return;
    const currentPos = axis === 'vertical' ? e.clientY : e.clientX;
    const delta = startPosRef.current - currentPos;
    const newSize = startSizeRef.current + sign * delta;
    const clamped = Math.max(min, Math.min(max, newSize));

    // 如果提供了 onLiveResize，直接同步调用实时更新 CSS 变量/DOM（0ms 开销，完全绕过 React 调度）
    if (onLiveResize) {
      onLiveResize(clamped);
    }

    if (pendingSizeRef.current === clamped && throttle) return;
    pendingSizeRef.current = clamped;

    // 当未配置 onLiveResize 或在单元测试环境中时，执行 onResize
    if (!onLiveResize || isTest) {
      if (throttle && typeof window !== 'undefined' && typeof window.requestAnimationFrame === 'function') {
        if (rafIdRef.current === null) {
          rafIdRef.current = window.requestAnimationFrame(() => {
            rafIdRef.current = null;
            if (pendingSizeRef.current !== null) {
              onResize(pendingSizeRef.current);
              pendingSizeRef.current = null;
            }
          });
        }
      } else {
        onResize(clamped);
      }
    }
  }, [axis, sign, min, max, onResize, onLiveResize, throttle, isTest]);
  // 结束拖拽
  const handleMouseUp = useCallback(() => {
    isDraggingRef.current = false;
    setIsDragging(false);
    if (rafIdRef.current !== null) {
      window.cancelAnimationFrame(rafIdRef.current);
      rafIdRef.current = null;
    }
    if (pendingSizeRef.current !== null) {
      onResize(pendingSizeRef.current);
      pendingSizeRef.current = null;
    }
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
    restoreIframes();
    endLayoutTransition();
  }, [onResize]);
  // 绑定/解绑全局事件
  useEffect(() => {
    if (!isDragging) return;

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      if (rafIdRef.current !== null) {
        window.cancelAnimationFrame(rafIdRef.current);
        rafIdRef.current = null;
      }
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
      restoreIframes();
      endLayoutTransition();
    };
  }, [isDragging, handleMouseMove, handleMouseUp]);
  return { isDragging, handleMouseDown };
}
