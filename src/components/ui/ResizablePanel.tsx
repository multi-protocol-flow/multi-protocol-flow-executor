import { useState, useEffect, useCallback } from "react";
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

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
  }, []);

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      const newWidth = Math.max(minWidth, Math.min(maxWidth, e.clientX));
      setLeftWidth(newWidth);
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
        className={cn(
          "w-1 bg-border hover:bg-primary/50 cursor-col-resize flex-shrink-0 transition-colors"
        )}
      />

      {/* 右侧面板 */}
      <div className="flex-1 overflow-hidden">
        {rightPanel}
      </div>
    </div>
  );
}
