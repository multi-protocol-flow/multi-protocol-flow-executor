/**
 * EmptyCanvasTips - 未进入画布时的软件使用技巧与快捷键提示组件
 * 支持多语言、随机展示、点击切换、悬停暂停与定时轮播
 */
import React, { useState, useEffect, useCallback } from 'react';
import { Lightbulb, Shuffle } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import { hotkeyLabel } from '@/lib/hotkeyLabels';
import type { TranslationFunctions } from '@/i18n/i18n-types';

export type WelcomeTips = TranslationFunctions['welcome']['empty_tips'];

/** 单条提示定义 */
export interface CanvasTipItem {
  id: string;
  /** 快捷键标签列表或国际化解析函数 */
  keys: string[] | ((tips: WelcomeTips) => string[]);
  /** 描述国际化解析函数 */
  getDesc: (tips: WelcomeTips) => string;
}

/** 预置的软件使用小技巧列表 */
export const CANVAS_TIPS: CanvasTipItem[] = [
  {
    id: 'right_click',
    keys: (tips) => [tips.right_click_key()],
    getDesc: (tips) => tips.right_click_desc(),
  },
  {
    id: 'spotlight',
    keys: ['Ctrl+K'],
    getDesc: (tips) => tips.spotlight_desc(),
  },
  {
    id: 'copilot',
    keys: ['Ctrl+I'],
    getDesc: (tips) => tips.copilot_desc(),
  },
  {
    id: 'search',
    keys: ['Ctrl+Shift+F'],
    getDesc: (tips) => tips.search_desc(),
  },
  {
    id: 'execute',
    keys: ['Ctrl+Enter'],
    getDesc: (tips) => tips.execute_desc(),
  },
  {
    id: 'shortcuts',
    keys: ['Ctrl+/'],
    getDesc: (tips) => tips.shortcuts_desc(),
  },
  {
    id: 'duplicate',
    keys: ['Ctrl+D'],
    getDesc: (tips) => tips.duplicate_desc(),
  },
  {
    id: 'fit_view',
    keys: ['F'],
    getDesc: (tips) => tips.fit_view_desc(),
  },
  {
    id: 'pan',
    keys: (tips) => [tips.pan_key()],
    getDesc: (tips) => tips.pan_desc(),
  },
  {
    id: 'zoom',
    keys: (tips) => [tips.zoom_key()],
    getDesc: (tips) => tips.zoom_desc(),
  },
  {
    id: 'reset_zoom',
    keys: ['Ctrl+0'],
    getDesc: (tips) => tips.reset_zoom_desc(),
  },
  {
    id: 'import_file',
    keys: (tips) => [tips.import_file_key()],
    getDesc: (tips) => tips.import_file_desc(),
  },
  {
    id: 'box_select',
    keys: (tips) => [tips.box_select_key()],
    getDesc: (tips) => tips.box_select_desc(),
  },
  {
    id: 'drag_port',
    keys: (tips) => [tips.drag_port_key()],
    getDesc: (tips) => tips.drag_port_desc(),
  },
  {
    id: 'toggle_sidebar',
    keys: ['Ctrl+B'],
    getDesc: (tips) => tips.toggle_sidebar_desc(),
  },
  {
    id: 'undo_redo',
    keys: ['Ctrl+Z', 'Ctrl+Shift+Z'],
    getDesc: (tips) => tips.undo_redo_desc(),
  },
  {
    id: 'delete',
    keys: ['Delete', 'Backspace'],
    getDesc: (tips) => tips.delete_desc(),
  },
  {
    id: 'save',
    keys: ['Ctrl+S'],
    getDesc: (tips) => tips.save_desc(),
  },
  {
    id: 'select_all',
    keys: ['Ctrl+A'],
    getDesc: (tips) => tips.select_all_desc(),
  },
  {
    id: 'toggle_config',
    keys: ['Ctrl+J'],
    getDesc: (tips) => tips.toggle_config_desc(),
  },
];

export interface EmptyCanvasTipsProps {
  /** 初始指定提示索引（主要用于测试） */
  initialIndex?: number;
  /** 自动轮播间隔（毫秒，默认 8000，设为 0 禁用） */
  autoRotateInterval?: number;
  /** 自定义类名 */
  className?: string;
}

export function EmptyCanvasTips({
  initialIndex,
  autoRotateInterval = 8000,
  className = '',
}: EmptyCanvasTipsProps) {
  const ll = getLL();
  const tips = ll.welcome.empty_tips;

  const [currentIndex, setCurrentIndex] = useState<number>(() => {
    if (typeof initialIndex === 'number' && initialIndex >= 0 && initialIndex < CANVAS_TIPS.length) {
      return initialIndex;
    }
    return Math.floor(Math.random() * CANVAS_TIPS.length);
  });

  const [isHovered, setIsHovered] = useState(false);

  // 随机切换至下一个不同提示
  const handleNextTip = useCallback(() => {
    setCurrentIndex((prev) => {
      if (CANVAS_TIPS.length <= 1) return prev;
      let next = Math.floor(Math.random() * (CANVAS_TIPS.length - 1));
      if (next >= prev) next += 1;
      return next;
    });
  }, []);

  // 自动轮播定时器（悬停时暂停）
  useEffect(() => {
    if (autoRotateInterval <= 0 || isHovered) return;
    const timer = setInterval(handleNextTip, autoRotateInterval);
    return () => clearInterval(timer);
  }, [autoRotateInterval, isHovered, handleNextTip]);

  const currentTip = CANVAS_TIPS[currentIndex] ?? CANVAS_TIPS[0];
  const keysList = typeof currentTip.keys === 'function'
    ? currentTip.keys(tips)
    : currentTip.keys.map((k) => hotkeyLabel([k]));
  const desc = currentTip.getDesc(tips);

  return (
    <div
      data-testid="empty-canvas-tips"
      className={`inline-flex items-center justify-center ${className}`}
    >
      <button
        type="button"
        data-testid="empty-canvas-tips-btn"
        onClick={handleNextTip}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
        title={tips.next_tip()}
        aria-label={`${tips.tip_label()}: ${desc}`}
        className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs text-muted-foreground/70 hover:text-foreground hover:bg-muted/40 border border-transparent hover:border-border/50 transition-all duration-150 cursor-pointer select-none group"
      >
        <span className="flex items-center gap-1 text-muted-foreground/60 group-hover:text-amber-500 transition-colors">
          <Lightbulb className="w-3.5 h-3.5" />
          <span className="text-[11px] font-medium">{tips.tip_label()}:</span>
        </span>
        <span className="inline-flex items-center gap-1">
          {keysList.map((keyStr, idx) => (
            <React.Fragment key={idx}>
              {idx > 0 && <span className="text-[10px] text-muted-foreground/40 mx-0.5">/</span>}
              <kbd className="px-1.5 py-0.5 rounded border border-border bg-muted text-[10px] font-mono shadow-xs">
                {keyStr}
              </kbd>
            </React.Fragment>
          ))}
          <span className="ml-0.5">{desc}</span>
        </span>
        <Shuffle className="w-3 h-3 text-muted-foreground/40 opacity-0 group-hover:opacity-100 transition-opacity ml-0.5" />
      </button>
    </div>
  );
}
