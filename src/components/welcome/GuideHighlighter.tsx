/**
 * GuideHighlighter - 新手引导高亮组件
 *
 * 在半透明遮罩上为指定元素开一个"洞"，配合 tooltip 实现引导效果。
 * 使用 createPortal 渲染到 document.body，确保覆盖在所有组件之上。
 */
import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';

interface GuideHighlighterProps {
  /** CSS selector for the target element (e.g. '[data-guide="new-flow-btn"]') */
  targetSelector: string;
  /** Padding around the target element in pixels */
  padding?: number;
  /** Border radius of the highlight hole */
  borderRadius?: number;
  /** Content to show in the tooltip/callout near the highlighted element */
  children?: React.ReactNode;
  /** Position of the tooltip relative to target: 'top' | 'bottom' | 'left' | 'right' */
  tooltipPosition?: 'top' | 'bottom' | 'left' | 'right';
  /** Callback when overlay is clicked */
  onClick?: () => void;
}

interface TargetRect {
  top: number;
  left: number;
  width: number;
  height: number;
}

export function GuideHighlighter({
  targetSelector,
  padding = 8,
  borderRadius = 12,
  children,
  tooltipPosition = 'bottom',
  onClick,
}: GuideHighlighterProps) {
  const [targetRect, setTargetRect] = useState<TargetRect | null>(null);

  useEffect(() => {
    const updatePosition = () => {
      const el = document.querySelector(targetSelector);
      if (el) {
        const rect = el.getBoundingClientRect();
        setTargetRect({
          top: rect.top - padding,
          left: rect.left - padding,
          width: rect.width + padding * 2,
          height: rect.height + padding * 2,
        });
      }
    };

    updatePosition();
    // 窗口尺寸/滚动变化时重新计算
    window.addEventListener('resize', updatePosition);
    window.addEventListener('scroll', updatePosition);
    // 使用 MutationObserver 处理动态内容
    const observer = new MutationObserver(updatePosition);
    observer.observe(document.body, { childList: true, subtree: true, attributes: true });

    return () => {
      window.removeEventListener('resize', updatePosition);
      window.removeEventListener('scroll', updatePosition);
      observer.disconnect();
    };
  }, [targetSelector, padding]);

  // 目标元素未找到时，渲染居中卡片但不显示遮罩层
  const showOverlay = targetRect !== null;

  return createPortal(
    <div
      className="fixed inset-0 z-[60]"
      onClick={onClick}
      style={{ cursor: onClick ? 'pointer' : 'default' }}
    >
      {/* 半透明遮罩 + 高亮开孔（仅有目标时才渲染） */}
      {showOverlay && (
        <svg className="absolute inset-0 w-full h-full" style={{ pointerEvents: 'none' }}>
          <defs>
            <mask id="guide-highlight-mask">
              <rect width="100%" height="100%" fill="white" />
              <rect
                x={targetRect.left}
                y={targetRect.top}
                width={targetRect.width}
                height={targetRect.height}
                fill="black"
                rx={borderRadius}
              />
            </mask>
          </defs>
          <rect
            width="100%"
            height="100%"
            fill="rgba(0, 0, 0, 0.5)"
            mask="url(#guide-highlight-mask)"
          />
        </svg>
      )}

      {/* Tooltip/callout：有目标时定位到目标附近，否则居中显示 */}
      {children && (
        <div
          className="animate-in fade-in zoom-in-95 duration-200"
          style={
            showOverlay
              ? { position: 'absolute', ...getTooltipStyle(targetRect, tooltipPosition) }
              : {
                  position: 'fixed',
                  top: '50%',
                  left: '50%',
                  transform: 'translate(-50%, -50%)',
                }
          }
        >
          {children}
        </div>
      )}
    </div>,
    document.body
  );
}

function clampX(center: number, cardWidth: number, padding: number): number {
  return Math.max(
    cardWidth / 2 + padding,
    Math.min(center, window.innerWidth - cardWidth / 2 - padding),
  );
}

function clampY(center: number, cardHeight: number, padding: number): number {
  return Math.max(
    cardHeight / 2 + padding,
    Math.min(center, window.innerHeight - cardHeight / 2 - padding),
  );
}

function getTooltipStyle(rect: TargetRect, position: string): React.CSSProperties {
  // 卡片估算尺寸，用于边界裁剪
  const CARD_WIDTH = 320;
  const CARD_HEIGHT = 280; // 引导卡片最大高度估算
  const PAD = 16;

  switch (position) {
    case 'top': {
      return {
        left: clampX(rect.left + rect.width / 2, CARD_WIDTH, PAD),
        bottom: window.innerHeight - rect.top + 16,
        transform: 'translateX(-50%)',
        maxWidth: Math.min(CARD_WIDTH, window.innerWidth - PAD * 2),
      };
    }
    case 'bottom': {
      // bottom 位置的 top 值可能使卡片超出屏幕底部
      const topValue = Math.min(
        rect.top + rect.height + 16,
        window.innerHeight - 100, // 至少留 100px 底部空间
      );
      return {
        left: clampX(rect.left + rect.width / 2, CARD_WIDTH, PAD),
        top: topValue,
        transform: 'translateX(-50%)',
        maxWidth: Math.min(CARD_WIDTH, window.innerWidth - PAD * 2),
      };
    }
    case 'left': {
      return {
        right: Math.max(16, window.innerWidth - rect.left + 16),
        top: clampY(rect.top + rect.height / 2, CARD_HEIGHT, PAD),
        transform: 'translateY(-50%)',
        maxWidth: Math.min(CARD_WIDTH, window.innerWidth - PAD * 2),
      };
    }
    case 'right': {
      // right 位置：left 要保证卡片不超出右边界，top 要保证不超出上下边界
      const leftValue = Math.max(
        PAD,
        Math.min(rect.left + rect.width + 16, window.innerWidth - CARD_WIDTH - PAD),
      );
      return {
        left: leftValue,
        top: clampY(rect.top + rect.height / 2, CARD_HEIGHT, PAD),
        transform: 'translateY(-50%)',
        maxWidth: Math.min(CARD_WIDTH, window.innerWidth - PAD * 2),
      };
    }
    default: {
      return {
        left: clampX(rect.left + rect.width / 2, CARD_WIDTH, PAD),
        top: rect.top + rect.height + 16,
        transform: 'translateX(-50%)',
        maxWidth: Math.min(CARD_WIDTH, window.innerWidth - PAD * 2),
      };
    }
  }
}
