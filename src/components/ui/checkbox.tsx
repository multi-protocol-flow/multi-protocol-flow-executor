import * as React from 'react';
import { Check, Minus } from 'lucide-react';
import { cn } from '@/lib/utils';

export interface CheckboxProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'type' | 'onChange'> {
  checked?: boolean;
  indeterminate?: boolean;
  onCheckedChange?: (checked: boolean) => void;
  onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void;
}

const Checkbox = React.forwardRef<HTMLInputElement, CheckboxProps>(
  (
    {
      className,
      checked = false,
      indeterminate = false,
      disabled = false,
      onCheckedChange,
      onChange,
      ...props
    },
    ref
  ) => {
    const internalRef = React.useRef<HTMLInputElement | null>(null);

    // 同步 indeterminate 属性到原生 DOM
    React.useEffect(() => {
      if (internalRef.current) {
        internalRef.current.indeterminate = indeterminate;
      }
    }, [indeterminate]);

    const handleRef = (node: HTMLInputElement | null) => {
      internalRef.current = node;
      if (typeof ref === 'function') {
        ref(node);
      } else if (ref) {
        ref.current = node;
      }
    };

    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      if (disabled) return;
      onCheckedChange?.(e.target.checked);
      onChange?.(e);
    };

    const isChecked = checked && !indeterminate;

    return (
      <label
        className={cn(
          'relative inline-flex items-center justify-center select-none',
          disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'
        )}
      >
        <input
          type="checkbox"
          className="sr-only peer"
          checked={checked}
          disabled={disabled}
          onChange={handleChange}
          ref={handleRef}
          {...props}
        />
        <div
          className={cn(
            'flex items-center justify-center h-4 w-4 rounded border transition-colors shrink-0',
            // 焦点环无障碍规范
            'peer-focus-visible:outline-none peer-focus-visible:ring-2 peer-focus-visible:ring-ring peer-focus-visible:ring-offset-2 peer-focus-visible:ring-offset-background',
            // 未选中状态：在浅色与深色背景下均保持清晰边框与底色
            'border-zinc-300 bg-white hover:border-zinc-400 dark:border-zinc-600 dark:bg-zinc-800/80 dark:hover:border-zinc-500',
            // 选中或半选状态：主题主色背景与前景色对勾（暗黑模式下为白底黑勾，对比度极高）
            (isChecked || indeterminate) && [
              'bg-primary border-primary text-primary-foreground',
              'hover:bg-primary/90 hover:border-primary/90',
            ],
            className
          )}
        >
          {isChecked && <Check className="h-3 w-3 stroke-[3]" />}
          {indeterminate && <Minus className="h-3 w-3 stroke-[3]" />}
        </div>
      </label>
    );
  }
);

Checkbox.displayName = 'Checkbox';

export { Checkbox };
