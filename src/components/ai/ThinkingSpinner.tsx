import { cn } from '@/lib/utils';

export interface ThinkingDotsProps {
  className?: string;
  dotClassName?: string;
}

/**
 * ThinkingDots (ThinkingSpinner) - AI 深度思考点点堆叠下降动效指示器
 *
 * 替代传统旋转圆圈，采用 3 颗点点依序堆叠下落（Cascading Fall）的微动效：
 * 1. 彻底解决旋转图形在小尺寸（10~14px）下因视觉神经质心追踪与子像素渲染导致的“圆心不固定”视错觉；
 * 2. 3 颗点点呈平滑波浪式依序从上方向下堆叠下落（-2.5px -> +2px -> 归位），视觉轻盈且具有思考流动感；
 * 3. 采用纯 CSS GPU 合成，不依赖 SVG 旋转变换，各平台与 WebView 下渲染稳定丝滑。
 */
export function ThinkingDots({ className, dotClassName }: ThinkingDotsProps) {
  return (
    <span
      data-testid="thinking-dots"
      className={cn('inline-flex items-center gap-1 shrink-0 px-0.5 select-none', className)}
      aria-hidden="true"
    >
      <style>{`
        @keyframes thinking-dot-fall {
          0%, 100% {
            transform: translateY(-2.5px) scale(0.8);
            opacity: 0.3;
          }
          45% {
            transform: translateY(2px) scale(1.15);
            opacity: 1;
          }
          70% {
            transform: translateY(0px) scale(1);
            opacity: 0.65;
          }
        }
        .animate-thinking-dot-1 {
          animation: thinking-dot-fall 1.1s cubic-bezier(0.4, 0, 0.2, 1) infinite;
          animation-delay: 0s;
        }
        .animate-thinking-dot-2 {
          animation: thinking-dot-fall 1.1s cubic-bezier(0.4, 0, 0.2, 1) infinite;
          animation-delay: 0.18s;
        }
        .animate-thinking-dot-3 {
          animation: thinking-dot-fall 1.1s cubic-bezier(0.4, 0, 0.2, 1) infinite;
          animation-delay: 0.36s;
        }
      `}</style>
      <span
        data-testid="thinking-dot"
        className={cn(
          'w-1.5 h-1.5 rounded-full bg-primary animate-thinking-dot-1 shrink-0',
          dotClassName
        )}
      />
      <span
        data-testid="thinking-dot"
        className={cn(
          'w-1.5 h-1.5 rounded-full bg-primary animate-thinking-dot-2 shrink-0',
          dotClassName
        )}
      />
      <span
        data-testid="thinking-dot"
        className={cn(
          'w-1.5 h-1.5 rounded-full bg-primary animate-thinking-dot-3 shrink-0',
          dotClassName
        )}
      />
    </span>
  );
}

/** 兼容 ThinkingSpinner 别名导出 */
export const ThinkingSpinner = ThinkingDots;
export type ThinkingSpinnerProps = ThinkingDotsProps;
