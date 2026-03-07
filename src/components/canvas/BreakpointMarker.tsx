/**
 * 断点标记组件
 * 显示在节点左上角，用于设置/取消断点
 */
import { cn } from '@/lib/utils';

export interface BreakpointMarkerProps {
  /** 是否设置断点 */
  hasBreakpoint: boolean;
  /** 点击切换断点状态的回调 */
  onToggle: () => void;
}

/**
 * 断点标记
 * - 有断点：红色实心圆点
 * - 无断点：灰色空心圆点（悬停显示红色）
 */
export function BreakpointMarker({ hasBreakpoint, onToggle }: BreakpointMarkerProps) {
  return (
    <button
      type="button"
      onClick={(e) => {
        e.stopPropagation(); // 防止触发节点选中
        onToggle();
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
        // 无断点：灰色空心，悬停显示红色
        !hasBreakpoint && [
          'bg-white',
          'border-gray-400',
          'hover:bg-red-50',
          'hover:border-red-500',
        ]
      )}
      title={hasBreakpoint ? '点击取消断点' : '点击设置断点'}
      aria-label={hasBreakpoint ? '取消断点' : '设置断点'}
    />
  );
}
