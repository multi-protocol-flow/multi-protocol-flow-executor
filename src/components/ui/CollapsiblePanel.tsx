/**
 * CollapsiblePanel - 可折叠面板组件
 * 支持 VSCode 风格的折叠/展开，以及拖拽调整高度
 */
import { ChevronDown, ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useState, useCallback, useRef, useEffect, type ReactNode, type KeyboardEvent, type MouseEvent } from 'react';
import { getLL } from '@/i18n/i18n-lazy';

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
  /** 是否允许拖拽调整高度（通过标题栏上边缘拖拽） */
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
  /** 额外的 style */
  style?: React.CSSProperties;
}

/** 折叠状态下的标题栏高度 */
const COLLAPSED_HEIGHT = 32;

/** 拖拽判定阈值（像素） */
const DRAG_THRESHOLD = 3;

export function CollapsiblePanel({
  title,
  headerRight,
  children,
  isCollapsed,
  onToggle,
  height,
  onHeightChange,
  resizable = false,
  minHeight = 100,
  maxHeight = 400,
  className,
  contentClassName,
  collapsedHeight = COLLAPSED_HEIGHT,
  style,
}: CollapsiblePanelProps) {
  const LL = getLL();
  const [isDragging, setIsDragging] = useState(false);
  const startYRef = useRef(0);
  const startHeightRef = useRef(0);
  const didDragRef = useRef(false);
  const headerRef = useRef<HTMLDivElement>(null);

  // 开始拖拽（在标题栏上按下鼠标）
  const handleHeaderMouseDown = useCallback((e: MouseEvent<HTMLDivElement>) => {
    if (!resizable || isCollapsed) return;
    
    // 记录起始状态
    startYRef.current = e.clientY;
    startHeightRef.current = height ?? minHeight;
    didDragRef.current = false;
  }, [resizable, isCollapsed, height, minHeight]);

  // 全局鼠标移动
  const handleMouseMove = useCallback((e: globalThis.MouseEvent) => {
    if (startYRef.current === 0 || !onHeightChange) return;
    
    const deltaY = startYRef.current - e.clientY;
    
    // 超过阈值才视为拖拽
    if (!didDragRef.current && Math.abs(deltaY) > DRAG_THRESHOLD) {
      didDragRef.current = true;
      setIsDragging(true);
      document.body.style.cursor = 'row-resize';
      document.body.style.userSelect = 'none';
    }
    
    if (didDragRef.current) {
      const newHeight = Math.max(minHeight, Math.min(maxHeight, startHeightRef.current + deltaY));
      onHeightChange(newHeight);
    }
  }, [onHeightChange, minHeight, maxHeight]);

  // 全局鼠标释放
  const handleMouseUp = useCallback(() => {
    if (startYRef.current === 0) return;
    
    startYRef.current = 0;
    
    if (didDragRef.current) {
      setIsDragging(false);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    }
  }, []);

  // 点击标题栏（仅在未拖拽时触发折叠）
  const handleHeaderClick = useCallback(() => {
    if (didDragRef.current) return;
    onToggle();
  }, [onToggle]);

  // 全局事件监听
  useEffect(() => {
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [handleMouseMove, handleMouseUp]);

  // 键盘事件处理
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onToggle();
    }
  }, [onToggle]);

  // 计算面板高度
  const panelHeight = isCollapsed ? collapsedHeight : (height ?? 'auto' as unknown as number);

  // 计算样式：确保高度固定，不会被内容撑大
  const panelStyle: React.CSSProperties = {
    ...style,
    height: typeof panelHeight === 'number' ? panelHeight : undefined,
    maxHeight: typeof panelHeight === 'number' ? panelHeight : undefined,
  };

  return (
    <div
      className={cn(
        'flex flex-col bg-background overflow-hidden',
        isCollapsed ? 'shrink-0' : 'min-h-0',
        isDragging && 'select-none',
        className
      )}
      style={panelStyle}
    >
      {/* 标题栏 - 点击折叠，上边缘拖拽调整高度 */}
      <div
        ref={headerRef}
        role="button"
        tabIndex={0}
        className={cn(
          'shrink-0 flex items-center justify-between px-3 h-8 w-full',
          'border-b border-border bg-background',
          'cursor-pointer select-none',
          'hover:bg-accent transition-colors duration-150',
          'focus:outline-none focus:ring-2 focus:ring-inset focus:ring-muted-foreground',
          resizable && !isCollapsed && 'cursor-row-resize'
        )}
        onMouseDown={handleHeaderMouseDown}
        onClick={handleHeaderClick}
        onKeyDown={handleKeyDown}
        aria-expanded={!isCollapsed}
        aria-label={isCollapsed ? LL.common.collapse_panel_aria({ title }) : LL.common.expand_panel_aria({ title })}
      >
        <div className="flex items-center gap-1.5">
          {isCollapsed ? (
            <ChevronRight className="w-4 h-4 text-muted-foreground" />
          ) : (
            <ChevronDown className="w-4 h-4 text-muted-foreground" />
          )}
          <span className="text-sm font-medium text-foreground">{title}</span>
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
            'flex-1 min-h-0 overflow-y-auto scrollbar-thin',
            contentClassName
          )}
        >
          {children}
        </div>
      )}
    </div>
  );
}
