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
  /** Position of the tooltip relative to target: 'top' | 'bottom' | 'left' | 'right' | 'center' */
  tooltipPosition?: 'top' | 'bottom' | 'left' | 'right' | 'center';
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
          onClick={(e) => e.stopPropagation()}
          style={
            showOverlay
              ? {
                  position: 'absolute',
                  maxHeight: 'calc(100vh - 32px)',
                  overflowY: 'auto',
                  ...getTooltipStyle(targetRect, tooltipPosition),
                }
              : {
                  position: 'fixed',
                  top: '50%',
                  left: '50%',
                  transform: 'translate(-50%, -50%)',
                  maxHeight: 'calc(100vh - 32px)',
                  overflowY: 'auto',
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
  // 卡片估算尺寸，用于边界裁剪与自适应定位
  const CARD_WIDTH = 320;
  const CARD_HEIGHT = 220; // 引导卡片实际高度估算（包含 header, desc, footer）
  const GAP = 16;
  const PAD = 16;

  const viewportWidth = window.innerWidth;
  const viewportHeight = window.innerHeight;

  // 计算目标元素各方向的外部剩余空间
  const spaceBottom = viewportHeight - (rect.top + rect.height + GAP);
  const spaceTop = rect.top - GAP;
  const spaceRight = viewportWidth - (rect.left + rect.width + GAP);
  const spaceLeft = rect.left - GAP;

  // 判断各方向是否有足够空间容纳卡片
  const canFitBottom = spaceBottom >= CARD_HEIGHT + PAD;
  const canFitTop = spaceTop >= CARD_HEIGHT + PAD;
  const canFitRight = spaceRight >= CARD_WIDTH + PAD;
  const canFitLeft = spaceLeft >= CARD_WIDTH + PAD;

  // 自适应定位判断
  let resolvedPosition = position;

  if (position === 'center') {
    resolvedPosition = 'center';
  } else if (position === 'bottom') {
    if (!canFitBottom) {
      if (canFitTop) {
        resolvedPosition = 'top';
      } else {
        // 上下外部均不足以容纳（例如目标元素占满大部分屏幕高度），回退至内部居中
        resolvedPosition = 'center';
      }
    }
  } else if (position === 'top') {
    if (!canFitTop) {
      if (canFitBottom) {
        resolvedPosition = 'bottom';
      } else {
        resolvedPosition = 'center';
      }
    }
  } else if (position === 'right') {
    if (!canFitRight) {
      if (canFitLeft) {
        resolvedPosition = 'left';
      } else {
        resolvedPosition = 'center';
      }
    }
  } else if (position === 'left') {
    if (!canFitLeft) {
      if (canFitRight) {
        resolvedPosition = 'right';
      } else {
        resolvedPosition = 'center';
      }
    }
  }

  switch (resolvedPosition) {
    case 'center': {
      return {
        left: clampX(rect.left + rect.width / 2, CARD_WIDTH, PAD),
        top: clampY(rect.top + rect.height / 2, CARD_HEIGHT, PAD),
        transform: 'translate(-50%, -50%)',
        maxWidth: Math.min(CARD_WIDTH, viewportWidth - PAD * 2),
      };
    }
    case 'top': {
      const rawBottom = viewportHeight - rect.top + GAP;
      const maxBottom = viewportHeight - CARD_HEIGHT - PAD;
      const bottomValue = Math.min(rawBottom, maxBottom);
      return {
        left: clampX(rect.left + rect.width / 2, CARD_WIDTH, PAD),
        bottom: Math.max(PAD, bottomValue),
        transform: 'translateX(-50%)',
        maxWidth: Math.min(CARD_WIDTH, viewportWidth - PAD * 2),
      };
    }
    case 'bottom': {
      const rawTop = rect.top + rect.height + GAP;
      const maxTop = viewportHeight - CARD_HEIGHT - PAD;
      const topValue = Math.min(rawTop, maxTop);
      return {
        left: clampX(rect.left + rect.width / 2, CARD_WIDTH, PAD),
        top: Math.max(PAD, topValue),
        transform: 'translateX(-50%)',
        maxWidth: Math.min(CARD_WIDTH, viewportWidth - PAD * 2),
      };
    }
    case 'left': {
      return {
        right: Math.max(PAD, viewportWidth - rect.left + GAP),
        top: clampY(rect.top + rect.height / 2, CARD_HEIGHT, PAD),
        transform: 'translateY(-50%)',
        maxWidth: Math.min(CARD_WIDTH, viewportWidth - PAD * 2),
      };
    }
    case 'right': {
      const leftValue = Math.max(
        PAD,
        Math.min(rect.left + rect.width + GAP, viewportWidth - CARD_WIDTH - PAD),
      );
      return {
        left: leftValue,
        top: clampY(rect.top + rect.height / 2, CARD_HEIGHT, PAD),
        transform: 'translateY(-50%)',
        maxWidth: Math.min(CARD_WIDTH, viewportWidth - PAD * 2),
      };
    }
    default: {
      return {
        left: clampX(rect.left + rect.width / 2, CARD_WIDTH, PAD),
        top: clampY(rect.top + rect.height / 2, CARD_HEIGHT, PAD),
        transform: 'translate(-50%, -50%)',
        maxWidth: Math.min(CARD_WIDTH, viewportWidth - PAD * 2),
      };
    }
  }
}
