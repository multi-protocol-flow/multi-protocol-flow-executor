import { useState, useEffect, useCallback, useRef } from "react";
import { cn } from "@/lib/utils";

interface ResizablePanelProps {
  leftPanel: React.ReactNode;
  rightPanel: React.ReactNode;
  defaultWidth?: number;
  minWidth?: number;
  maxWidth?: number;
  className?: string;
}

export function ResizablePanel({
  leftPanel,
  rightPanel,
  defaultWidth = 280,
  minWidth = 180,
  maxWidth = 400,
  className,
}: ResizablePanelProps) {
  const [leftWidth, setLeftWidth] = useState(defaultWidth);
  const [isDragging, setIsDragging] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
  }, []);

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      // 使用容器相对位置计算宽度，而不是页面绝对位置
      if (containerRef.current) {
        const containerRect = containerRef.current.getBoundingClientRect();
        const newWidth = Math.max(
          minWidth,
          Math.min(maxWidth, e.clientX - containerRect.left)
        );
        setLeftWidth(newWidth);
      }
    };

    const handleMouseUp = () => {
      setIsDragging(false);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isDragging, minWidth, maxWidth]);

  return (
    <div
      ref={containerRef}
      className={cn(
        "flex h-full w-full",
        isDragging && "select-none",
        className
      )}
    >
      {/* 左侧面板 */}
      <div style={{ width: leftWidth }} className="flex-shrink-0 overflow-hidden">
        {leftPanel}
      </div>

      {/* 分隔栏 */}
      <div
        onMouseDown={handleMouseDown}
        className="w-2 flex items-center justify-center flex-shrink-0 relative group cursor-col-resize"
      >
        {/* 背景分隔线 */}
        <div className="absolute inset-y-0 left-1/2 -translate-x-1/2 w-px bg-border group-hover:bg-blue-400 group-active:bg-blue-500 transition-colors" />
        {/* 拖拽手柄 */}
        <div className="relative z-10 w-4 h-8 rounded-sm bg-muted-foreground/20 group-hover:bg-blue-400 group-active:bg-blue-500 transition-colors opacity-0 group-hover:opacity-100 group-active:opacity-100" />
      </div>

      {/* 右侧面板 */}
      <div className="flex-1 overflow-hidden">
        {rightPanel}
      </div>
    </div>
  );
}
