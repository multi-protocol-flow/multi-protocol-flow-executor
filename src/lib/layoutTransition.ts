/**
 * layoutTransition.ts - 布局动画期间的 ResizeObserver 节流保护层
 *
 * 背景：左右抽屉与底部面板通过 CSS width/height 过渡展开收缩（200ms）。
 * 过渡期间画布容器逐帧变形，@xyflow/react 的 ResizeObserver 会在每一帧
 * 更新 store 尺寸，进而驱动 MiniMap 全量遍历节点重算包围盒并重写 SVG 视口蒙版，
 * 与浏览器 Flexbox 重排在同一帧撞车导致掉帧。
 *
 * 方案：过渡期间挂起 ResizeObserver 回调，动画结束后以最新一次尺寸单次刷新，
 * 将动画期内的 N 次重算压缩为 1 次。
 */

let isLayoutTransitioning = false;
let transitionTimer: ReturnType<typeof setTimeout> | null = null;
let observerInstalled = false;
/** 被挂起的回调，键为对应的 PatchedResizeObserver 实例 */
const pendingCallbacks = new Map<object, () => void>();

/** 释放所有挂起回调（先取出再清空，避免回调内部重新注册时被误清） */
function finishLayoutTransition() {
  isLayoutTransitioning = false;
  const callbacks = Array.from(pendingCallbacks.values());
  pendingCallbacks.clear();
  for (const cb of callbacks) {
    try {
      cb();
    } catch (error) {
      console.error('[layoutTransition] resize callback failed:', error);
    }
  }
}

/** 进入布局过渡保护状态：挂起 ResizeObserver 回调，等待显式结束 */
export function beginLayoutTransition() {
  if (typeof window === 'undefined') return;
  isLayoutTransitioning = true;
  if (transitionTimer) {
    clearTimeout(transitionTimer);
    transitionTimer = null;
  }
}

/** 进入布局过渡保护，并在 duration 毫秒后自动结束（用于 CSS 过渡驱动的面板展开/收起） */
export function startLayoutTransition(duration = 220) {
  if (typeof window === 'undefined') return;
  beginLayoutTransition();
  transitionTimer = setTimeout(() => {
    transitionTimer = null;
    finishLayoutTransition();
  }, duration);
}

/** 结束布局过渡保护，立即以最终尺寸刷新所有挂起回调 */
export function endLayoutTransition() {
  if (typeof window === 'undefined') return;
  if (transitionTimer) {
    clearTimeout(transitionTimer);
    transitionTimer = null;
  }
  finishLayoutTransition();
}

/** 初始化全局 ResizeObserver 保护拦截（仅在 main.tsx 调用一次） */
export function initLayoutTransitionObserver() {
  if (typeof window === 'undefined' || typeof window.ResizeObserver === 'undefined') {
    return;
  }
  if (observerInstalled) return;
  observerInstalled = true;

  const NativeResizeObserver = window.ResizeObserver;

  class PatchedResizeObserver {
    private readonly _observer: ResizeObserver;
    private _lastEntries: ResizeObserverEntry[] = [];

    constructor(callback: ResizeObserverCallback) {
      this._observer = new NativeResizeObserver((entries, obs) => {
        this._lastEntries = entries;

        if (isLayoutTransitioning) {
          // 过渡期间只保留最新一次回调，结束时统一执行
          pendingCallbacks.set(this, () => callback(this._lastEntries, obs));
          return;
        }

        callback(entries, obs);
      });
    }

    observe(target: Element, options?: ResizeObserverOptions) {
      this._observer.observe(target, options);
    }

    unobserve(target: Element) {
      this._observer.unobserve(target);
    }

    disconnect() {
      pendingCallbacks.delete(this);
      this._observer.disconnect();
    }
  }

  window.ResizeObserver = PatchedResizeObserver as unknown as typeof ResizeObserver;
}
