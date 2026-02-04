import React from 'react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { cn } from '@/lib/utils';

export interface ContextMenuProps {
  children: React.ReactNode;
  onRename?: () => void;
  onCopy?: () => void;
  onViewInfo?: () => void;
  className?: string;
}

/**
 * 右键菜单组件
 *
 * 功能：
 * - 使用 shadcn/ui Dropdown Menu 实现
 * - 提供菜单项：重命名、复制、查看信息
 * - 通过回调函数处理菜单项点击
 */
export function ContextMenu({
  children,
  onRename,
  onCopy,
  onViewInfo,
  className,
}: ContextMenuProps) {
  const handleRename = (e: Event) => {
    e.preventDefault();
    e.stopPropagation();
    onRename?.();
  };

  const handleCopy = (e: Event) => {
    e.preventDefault();
    e.stopPropagation();
    onCopy?.();
  };

  const handleViewInfo = (e: Event) => {
    e.preventDefault();
    e.stopPropagation();
    onViewInfo?.();
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild className={cn(className)}>
        {children}
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        {onRename && (
          <DropdownMenuItem onSelect={handleRename}>
            重命名
          </DropdownMenuItem>
        )}
        {onCopy && (
          <DropdownMenuItem onSelect={handleCopy}>
            复制
          </DropdownMenuItem>
        )}
        {onViewInfo && (
          <DropdownMenuItem onSelect={handleViewInfo}>
            查看信息
          </DropdownMenuItem>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
