/**
 * useFullscreen - 全屏模式 Hook
 * 提供全屏切换功能
 */
import { useCallback, useEffect, useState, type RefObject } from 'react';

interface UseFullscreenReturn {
  /** 当前是否处于全屏状态 */
  isFullscreen: boolean;
  /** 切换全屏状态 */
  toggleFullscreen: () => void;
  /** 退出全屏 */
  exitFullscreen: () => void;
}

/**
 * 全屏模式 Hook
 *
 * @param elementRef - 可选，指定要全屏的元素引用，默认使用 document.documentElement
 * @returns 全屏状态和控制方法
 *
 * @example
 * ```tsx
 * const { isFullscreen, toggleFullscreen, exitFullscreen } = useFullscreen();
 *
 * // 使用特定元素
 * const ref = useRef<HTMLDivElement>(null);
 * const { toggleFullscreen } = useFullscreen(ref);
 * ```
 */
export function useFullscreen(
  elementRef?: RefObject<HTMLElement | null>
): UseFullscreenReturn {
  const [isFullscreen, setIsFullscreen] = useState(false);

  // 退出全屏
  const exitFullscreen = useCallback(() => {
    if (document.fullscreenElement) {
      document.exitFullscreen();
    }
  }, []);

  // 切换全屏状态
  const toggleFullscreen = useCallback(() => {
    if (!document.fullscreenElement) {
      const element = elementRef?.current || document.documentElement;
      element.requestFullscreen();
    } else {
      document.exitFullscreen();
    }
  }, [elementRef]);

  // 监听 fullscreenchange 事件
  useEffect(() => {
    const handleChange = () => {
      setIsFullscreen(!!document.fullscreenElement);
    };

    document.addEventListener('fullscreenchange', handleChange);
    return () => document.removeEventListener('fullscreenchange', handleChange);
  }, []);

  // 监听 ESC 键退出全屏
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isFullscreen) {
        exitFullscreen();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isFullscreen, exitFullscreen]);

  return { isFullscreen, toggleFullscreen, exitFullscreen };
}
