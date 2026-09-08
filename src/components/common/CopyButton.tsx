/**
 * CopyButton - 通用数据复制按钮
 *
 * 支持文本、对象、函数懒求值；自动处理 JSON 格式化；
 * 支持防止点击冒泡、自定义尺寸与文案；
 * 点击后切换为绿色对勾图标并弹出 Toast 提示。
 */
import { useState, useCallback, memo } from 'react';
import { Copy, Check } from 'lucide-react';
import { cn } from '@/lib/utils';
import { copyToClipboard, safeStringify } from '@/lib/clipboard';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';

export interface CopyButtonProps {
  /** 欲复制的数据，可传原始值、对象或返回数据的函数 */
  value: unknown | (() => unknown);
  /** 复制格式：'raw'（转字符串）、'json'（2空格缩进）、'json-compact'（紧凑一行） */
  format?: 'raw' | 'json' | 'json-compact';
  /** 按钮文字标签（可选，传则显示在图标右侧） */
  label?: string;
  /** 复制成功后的临时文字标签 */
  successLabel?: string;
  /** 按钮悬浮提示标题 */
  title?: string;
  /** 复制成功后的自定义 toast 提示，默认使用 i18n */
  toastMessage?: string;
  /** 外层按钮 class */
  className?: string;
  /** 图标 class */
  iconClassName?: string;
  /** 按钮尺寸 */
  size?: 'xs' | 'sm' | 'default';
  /** 是否阻止事件冒泡（默认 true，避免在 Accordion / Details 中误触折叠切换） */
  stopPropagation?: boolean;
}

export const CopyButton = memo(function CopyButton({
  value,
  format = 'raw',
  label,
  successLabel,
  title,
  toastMessage,
  className,
  iconClassName,
  size = 'xs',
  stopPropagation = true,
}: CopyButtonProps) {
  const ll = getLL();
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(
    async (e: React.MouseEvent) => {
      if (stopPropagation) {
        e.stopPropagation();
      }

      try {
        const rawVal = typeof value === 'function' ? (value as () => unknown)() : value;
        let textToCopy = '';

        if (rawVal === undefined || rawVal === null) {
          textToCopy = '';
        } else if (typeof rawVal === 'string') {
          textToCopy = rawVal;
        } else if (format === 'json') {
          textToCopy = safeStringify(rawVal, 2);
        } else if (format === 'json-compact') {
          textToCopy = safeStringify(rawVal);
        } else {
          textToCopy = typeof rawVal === 'object' ? safeStringify(rawVal, 2) : String(rawVal);
        }

        const ok = await copyToClipboard(textToCopy);
        if (ok) {
          setCopied(true);
          const msg = toastMessage || ll.execution.report.copied_to_clipboard();
          toast.success(msg);
          setTimeout(() => setCopied(false), 1500);
        } else {
          toast.error(ll.execution.report.copy_failed());
        }
      } catch {
        toast.error(ll.execution.report.copy_failed());
      }
    },
    [value, format, stopPropagation, toastMessage, ll]
  );

  const sizeStyles = {
    xs: 'p-1 text-xs gap-1 h-6',
    sm: 'px-2 py-1 text-xs gap-1.5 h-7',
    default: 'px-2.5 py-1.5 text-sm gap-2 h-8',
  };

  const iconSizes = {
    xs: 'w-3.5 h-3.5',
    sm: 'w-3.5 h-3.5',
    default: 'w-4 h-4',
  };

  const buttonTitle = title || (copied ? ll.execution.report.copied() : (label || ll.execution.report.copy_all()));

  return (
    <button
      type="button"
      onClick={handleCopy}
      title={buttonTitle}
      aria-label={buttonTitle}
      className={cn(
        'inline-flex items-center justify-center rounded-md font-medium transition-all duration-150 cursor-pointer',
        'border border-border/60 hover:border-border bg-background/80 hover:bg-muted text-muted-foreground hover:text-foreground',
        'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring',
        sizeStyles[size],
        copied && 'text-emerald-600 dark:text-emerald-400 border-emerald-300 dark:border-emerald-700 bg-emerald-50/50 dark:bg-emerald-950/30',
        className
      )}
    >
      {copied ? (
        <Check className={cn(iconSizes[size], 'text-emerald-500 shrink-0 animate-in zoom-in-50 duration-150', iconClassName)} />
      ) : (
        <Copy className={cn(iconSizes[size], 'shrink-0', iconClassName)} />
      )}
      {(label || (copied && successLabel)) && (
        <span className="leading-none select-none">
          {copied ? (successLabel || label) : label}
        </span>
      )}
    </button>
  );
});
