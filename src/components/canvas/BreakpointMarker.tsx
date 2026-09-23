/**
 * 断点标记组件
 * 显示在节点左上角，用于设置/取消断点
 */
import { memo } from 'react';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';

export interface BreakpointMarkerProps {
  /** 是否设置断点 */
  hasBreakpoint: boolean;
  /** 点击切换断点状态的回调（locked 模式下不需要） */
  onToggle?: () => void;
  /** 是否为锁定状态（入口节点固定断点，不可取消） */
  locked?: boolean;
}

/**
 * 断点标记
 * - 有断点：红色实心圆点
 * - 无断点：灰色空心圆点（悬停显示红色）
 * - 锁定：红色实心圆点，不可点击（入口节点专用）
 */
export const BreakpointMarker = memo(function BreakpointMarker({ hasBreakpoint, onToggle, locked }: BreakpointMarkerProps) {
  // 锁定状态：始终显示红色实心，不可点击
  if (locked) {
    return (
      <div
        className={cn(
          'absolute -top-1.5 -left-1.5',
          'w-4 h-4',
          'rounded-full',
          'border-2',
          'bg-red-500',
          'border-red-600',
          'shadow-sm',
          'z-10',
          'pointer-events-none',
        )}
        title={getLL().flow.breakpoint_locked()}
      />
    );
  }

  return (
    <button
      type="button"
      onClick={(e) => {
        e.stopPropagation(); // 防止触发节点选中
        onToggle?.();
      }}
      className={cn(
        // 位置：节点左上角
        'absolute -top-1.5 -left-1.5',
        // 尺寸
        'w-4 h-4',
        // 形状
        'rounded-full',
        // 边框
        'border-2',
        // 过渡动画
        'transition-all duration-150',
        // 鼠标指针
        'cursor-pointer',
        // 层级（确保在最上层）
        'z-10',
        // 防止被其他元素遮挡
        'pointer-events-auto',
        // 有断点：红色实心
        hasBreakpoint && [
          'bg-red-500',
          'border-red-600',
          'hover:bg-red-600',
          'hover:border-red-700',
          'shadow-sm',
        ],
        // 无断点：灰色空心，悬停显示红色（适配暗黑模式背景，避免高光泛白）
        !hasBreakpoint && [
          'bg-background',
          'border-border dark:border-zinc-600',
          'hover:bg-red-50 dark:hover:bg-red-950/60',
          'hover:border-red-500 dark:hover:border-red-500',
        ]
      )}
      title={hasBreakpoint ? getLL().flow.breakpoint_set() : getLL().flow.breakpoint_unset()}
      aria-label={hasBreakpoint ? getLL().flow.breakpoint_set_aria() : getLL().flow.breakpoint_unset_aria()}
    />
  );
});
