/**
 * ConfigSection - 配置面板折叠区域组件
 * 在配置面板的表单中按逻辑分组折叠，默认收起"高级"区域
 */
import { useState, useCallback, type ReactNode } from 'react';
import { ChevronDown } from 'lucide-react';
import { cn } from '@/lib/utils';

interface ConfigSectionProps {
  /** 区域标题 */
  title: string;
  /** 是否默认展开 */
  defaultOpen?: boolean;
  /** 区域内容 */
  children: ReactNode;
  /** 可选徽标，如"必填""高级""可选" */
  badge?: string;
  /** localStorage 持久化 key，格式 config-section-${nodeType}-${sectionKey} */
  storageKey?: string;
  /** 外部控制展开状态（优先级高于 internal） */
  isOpen?: boolean;
  /** 展开状态变化回调 */
  onOpenChange?: (open: boolean) => void;
  /** 额外的 className */
  className?: string;
}

/**
 * 从 localStorage 读取折叠状态
 */
function loadCollapsed(storageKey: string): boolean | null {
  try {
    const stored = localStorage.getItem(storageKey);
    if (stored !== null) return stored === 'false'; // false = 未折叠
  } catch {
    // 忽略 localStorage 读取错误
  }
  return null;
}

/**
 * 保存折叠状态到 localStorage
 */
function saveCollapsed(storageKey: string, collapsed: boolean) {
  try {
    localStorage.setItem(storageKey, String(!collapsed));
  } catch {
    // 忽略 localStorage 写入错误
  }
}

export function ConfigSection({
  title,
  defaultOpen = true,
  children,
  badge,
  storageKey,
  isOpen: controlledOpen,
  onOpenChange,
  className,
}: ConfigSectionProps) {
  // 内部展开状态（受控模式下不使用）
  const initialCollapsed = storageKey
    ? loadCollapsed(storageKey) ?? !defaultOpen
    : !defaultOpen;
  const [internalOpen, setInternalOpen] = useState(!initialCollapsed);

  const isOpen = controlledOpen ?? internalOpen;

  const toggle = useCallback(() => {
    const next = !isOpen;
    if (controlledOpen === undefined) {
      setInternalOpen(next);
    }
    onOpenChange?.(next);
    if (storageKey) {
      saveCollapsed(storageKey, !next);
    }
  }, [isOpen, controlledOpen, onOpenChange, storageKey]);

  return (
    <div className={cn('border border-border rounded-lg overflow-hidden', className)}>
      {/* 标题栏 */}
      <button
        type="button"
        onClick={toggle}
        className={cn(
          'w-full flex items-center justify-between px-3 py-2',
          'text-sm font-medium text-foreground',
          'bg-muted/30 hover:bg-muted/50',
          'transition-colors duration-150 cursor-pointer',
          'focus:outline-none focus-visible:ring-2 focus-visible:ring-ring',
        )}
      >
        <div className="flex items-center gap-2">
          <ChevronDown
            className={cn(
              'w-3.5 h-3.5 text-muted-foreground transition-transform duration-200',
              !isOpen && '-rotate-90',
            )}
          />
          <span>{title}</span>
        </div>
        {badge && (
          <span
            className={cn(
              'text-[10px] px-1.5 py-0.5 rounded-full font-medium',
              badge === '必填' && 'bg-red-100 text-red-600 dark:bg-red-950 dark:text-red-400',
              badge === '高级' && 'bg-amber-100 text-amber-600 dark:bg-amber-950 dark:text-amber-400',
              badge === '可选' && 'bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400',
            )}
          >
            {badge}
          </span>
        )}
      </button>

      {/* 内容区域 */}
      <div
        className={cn(
          'overflow-hidden transition-all duration-200',
          isOpen ? 'max-h-[9999px] opacity-100' : 'max-h-0 opacity-0',
        )}
      >
        <div className="px-3 py-2 space-y-3">
          {children}
        </div>
      </div>
    </div>
  );
}

export default ConfigSection;
