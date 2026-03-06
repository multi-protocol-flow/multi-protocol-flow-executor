/**
 * CollapsiblePanel - 可折叠面板组件
 * 支持 VSCode 风格的折叠/展开，以及拖拽调整高度
 */
import { ChevronDown, ChevronRight, GripHorizontal } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useState, useCallback, useRef, useEffect, type ReactNode, type KeyboardEvent } from 'react';

interface CollapsiblePanelProps {
  /** 面板标题 */
  title: string;
  /** 标题栏右侧的额外内容（如按钮） */
  headerRight?: ReactNode;
  /** 面板内容 */
  children: ReactNode;
  /** 是否折叠 */
  isCollapsed: boolean;
  /** 切换折叠状态的回调 */
  onToggle: () => void;
  /** 面板高度（像素），折叠时忽略 */
  height?: number;
  /** 高度变化回调 */
  onHeightChange?: (height: number) => void;
  /** 是否显示上方的拖拽手柄 */
  showDragHandle?: boolean;
  /** 是否允许拖拽调整高度 */
  resizable?: boolean;
  /** 最小高度 */
  minHeight?: number;
  /** 最大高度 */
  maxHeight?: number;
  /** 额外的 className */
  className?: string;
  /** 内容区域的额外 className */
  contentClassName?: string;
  /** 折叠时的固定高度（仅标题栏） */
  collapsedHeight?: number;
}

/** 折叠状态下的标题栏高度 */
const COLLAPSED_HEIGHT = 32;

export function CollapsiblePanel({
  title,
  headerRight,
  children,
  isCollapsed,
  onToggle,
  height,
  onHeightChange,
  showDragHandle = false,
  resizable = false,
  minHeight = 100,
  maxHeight = 400,
  className,
  contentClassName,
  collapsedHeight = COLLAPSED_HEIGHT,
}: CollapsiblePanelProps) {
  const [isDragging, setIsDragging] = useState(false);
  const startYRef = useRef(0);
  const startHeightRef = useRef(0);

  // 开始拖拽调整高度
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (!resizable || isCollapsed) return;
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
    startYRef.current = e.clientY;
    startHeightRef.current = height ?? minHeight;
    document.body.style.cursor = 'row-resize';
    document.body.style.userSelect = 'none';
  }, [resizable, isCollapsed, height, minHeight]);

  // 拖拽中
  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDragging || !onHeightChange) return;
    const deltaY = startYRef.current - e.clientY;
    const newHeight = Math.max(minHeight, Math.min(maxHeight, startHeightRef.current + deltaY));
    onHeightChange(newHeight);
  }, [isDragging, onHeightChange, minHeight, maxHeight]);

  // 结束拖拽
  const handleMouseUp = useCallback(() => {
    if (!isDragging) return;
    setIsDragging(false);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, [isDragging]);

  // 事件监听
  useEffect(() => {
    if (!isDragging) return;
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging, handleMouseMove, handleMouseUp]);

  // 键盘事件处理
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onToggle();
    }
  }, [onToggle]);

  // 计算面板高度
  const panelHeight = isCollapsed ? collapsedHeight : (height ?? 'auto' as unknown as number);

  return (
    <div
      className={cn(
        'flex flex-col bg-white overflow-hidden',
        isCollapsed ? 'shrink-0' : 'min-h-0',
        className
      )}
      style={{
        height: typeof panelHeight === 'number' ? panelHeight : undefined,
        flex: isCollapsed ? 'none' : undefined,
      }}
    >
      {/* 上方拖拽手柄（用于面板之间的分隔） */}
      {showDragHandle && (
        <div
          className={cn(
            'shrink-0 h-1.5 bg-transparent hover:bg-slate-200 active:bg-slate-300',
            'cursor-row-resize flex items-center justify-center',
            'transition-colors duration-150',
            isDragging && 'bg-slate-300'
          )}
          onMouseDown={handleMouseDown}
          title="拖拽调整面板高度"
        >
          <GripHorizontal className="w-6 h-3 text-slate-400" />
        </div>
      )}

      {/* 标题栏 - 使用 div 避免嵌套 button 问题 */}
      <div
        role="button"
        tabIndex={0}
        className={cn(
          'shrink-0 flex items-center justify-between px-3 h-8 w-full',
          'border-b border-border bg-white',
          'cursor-pointer select-none',
          'hover:bg-slate-50 transition-colors duration-150',
          'focus:outline-none focus:ring-2 focus:ring-inset focus:ring-slate-300'
        )}
        onClick={onToggle}
        onKeyDown={handleKeyDown}
        aria-expanded={!isCollapsed}
        aria-label={isCollapsed ? `展开 ${title}` : `折叠 ${title}`}
      >
        <div className="flex items-center gap-1.5">
          {isCollapsed ? (
            <ChevronRight className="w-4 h-4 text-slate-400" />
          ) : (
            <ChevronDown className="w-4 h-4 text-slate-400" />
          )}
          <span className="text-sm font-medium text-slate-700">{title}</span>
        </div>
        {headerRight && (
          <div
            role="presentation"
            className="flex items-center gap-1"
            onClick={(e) => e.stopPropagation()}
          >
            {headerRight}
          </div>
        )}
      </div>

      {/* 内容区域 */}
      {!isCollapsed && (
        <div
          className={cn(
            'flex-1 min-h-0 overflow-hidden',
            contentClassName
          )}
        >
          {children}
        </div>
      )}
    </div>
  );
}
