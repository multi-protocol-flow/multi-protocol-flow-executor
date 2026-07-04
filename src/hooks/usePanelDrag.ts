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
  /** 尺寸变化回调 */
  onResize: (size: number) => void;
}

export function usePanelDrag(options: UsePanelDragOptions) {
  const { axis, min, max, sign = 1, initialSize, onResize } = options;

  const [isDragging, setIsDragging] = useState(false);
  const isDraggingRef = useRef(false);
  const startPosRef = useRef(0);
  const startSizeRef = useRef(0);

  // 开始拖拽
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isDraggingRef.current = true;
    setIsDragging(true);
    startPosRef.current = axis === 'vertical' ? e.clientY : e.clientX;
    startSizeRef.current = initialSize;
    document.body.style.cursor = axis === 'vertical' ? 'row-resize' : 'col-resize';
    document.body.style.userSelect = 'none';
  }, [axis, initialSize]);

  // 拖拽中（不依赖 isDragging state，用 ref 避免 effect 重建）
  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDraggingRef.current) return;
    const currentPos = axis === 'vertical' ? e.clientY : e.clientX;
    const delta = startPosRef.current - currentPos;
    const newSize = startSizeRef.current + sign * delta;
    const clamped = Math.max(min, Math.min(max, newSize));
    onResize(clamped);
  }, [axis, sign, min, max, onResize]);

  // 结束拖拽
  const handleMouseUp = useCallback(() => {
    isDraggingRef.current = false;
    setIsDragging(false);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, []);

  // 绑定/解绑全局事件
  useEffect(() => {
    if (!isDragging) return;

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);

    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging, handleMouseMove, handleMouseUp]);

  return { isDragging, handleMouseDown };
}
