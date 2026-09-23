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
  ({ className, checked, onCheckedChange, onChange, disabled, variant = 'default', ...props }, ref) => {
    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      if (disabled) return;
      onCheckedChange?.(e.target.checked);
      onChange?.(e);
    };

    return (
      <label
        className={cn(
          'relative inline-flex items-center select-none',
          disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'
        )}
      >
        <input
          type="checkbox"
          className="sr-only peer"
          checked={checked}
          disabled={disabled}
          onChange={handleChange}
          ref={ref}
          {...props}
        />
        <div
          className={cn(
            // 基础结构：36px × 20px，1px 边框
            'w-9 h-5 rounded-full relative transition-colors border box-border shrink-0',
            // 焦点环（仅键盘 Tab 聚焦可见，背景自适应暗黑/浅色）
            'peer-focus-visible:outline-none peer-focus-visible:ring-2 peer-focus-visible:ring-ring peer-focus-visible:ring-offset-2 peer-focus-visible:ring-offset-background',
            // 关闭状态：浅色与暗黑均有清晰轮廓与底轨对比
            'bg-zinc-200 border-zinc-300/80 dark:bg-zinc-800 dark:border-zinc-700/80',
            // 激活状态轨道底色与边框
            variant === 'default' && [
              'peer-checked:bg-primary peer-checked:border-primary',
            ],
            variant === 'emerald' && [
              'peer-checked:bg-emerald-600 peer-checked:border-emerald-600',
              'dark:peer-checked:bg-emerald-500 dark:peer-checked:border-emerald-500',
            ],
            variant === 'amber' && [
              'peer-checked:bg-amber-500 peer-checked:border-amber-500',
              'dark:peer-checked:bg-amber-500 dark:peer-checked:border-amber-500',
            ],
            // 滑块 (Thumb)：16px × 16px，左上定位 1px，带柔和投影
            'after:content-[""] after:absolute after:top-[1px] after:left-[1px]',
            'after:h-4 after:w-4 after:rounded-full after:shadow-xs after:transition-all',
            // 未激活时滑块底色：浅色白、暗黑色浅灰（保持立体凸起）
            'after:bg-white dark:after:bg-zinc-200',
            // 激活平移（16px 即平移至右端对齐）
            'peer-checked:after:translate-x-full',
            // 关键：default 在暗黑模式下轨道为浅亮色 primary (oklch 0.922)，滑块反色为深黑，保证强反差；其他情况为白滑块
            variant === 'default'
              ? 'peer-checked:after:bg-primary-foreground dark:peer-checked:after:bg-zinc-950'
              : 'peer-checked:after:bg-white',
            className
          )}
        />
      </label>
    );
  }
);
Switch.displayName = 'Switch';

export { Switch };
