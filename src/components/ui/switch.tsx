/**
 * Switch 组件
 * 简单的开关组件
 */
import * as React from 'react';
import { cn } from '@/lib/utils';

export interface SwitchProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'type'> {
  checked?: boolean;
  onCheckedChange?: (checked: boolean) => void;
  variant?: 'default' | 'emerald' | 'amber';
}

const Switch = React.forwardRef<HTMLInputElement, SwitchProps>(
  ({ className, checked, onCheckedChange, onChange, variant = 'default', ...props }, ref) => {
    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      onCheckedChange?.(e.target.checked);
      onChange?.(e);
    };

    return (
      <label className="relative inline-flex items-center cursor-pointer">
        <input
          type="checkbox"
          className="sr-only peer"
          checked={checked}
          onChange={handleChange}
          ref={ref}
          {...props}
        />
        <div
          className={cn(
            'w-9 h-5 bg-zinc-300 dark:bg-zinc-700 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-ring peer-focus:ring-offset-2 rounded-full peer transition-colors',
            'peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[""] after:absolute after:top-[2px] after:left-[2px]',
            'after:bg-white dark:after:bg-zinc-100 after:shadow-sm after:rounded-full after:h-4 after:w-4 after:transition-all',
            variant === 'emerald' && 'peer-checked:bg-emerald-600 dark:peer-checked:bg-emerald-500',
            variant === 'amber' && 'peer-checked:bg-amber-500 dark:peer-checked:bg-amber-500',
            variant === 'default' && 'peer-checked:bg-primary',
            className
          )}
        />
      </label>
    );
  }
);
Switch.displayName = 'Switch';

export { Switch };
