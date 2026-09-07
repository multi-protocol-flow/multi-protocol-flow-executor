/**
 * 全局未捕获异常与 Promise Rejection 拦截器
 * 自动捕获运行时崩溃，记入本地日志 (app.log)，并通过 Console 系统日志和 Toast 进行反馈。
 */
import { notifyError } from '@/lib/notify';
import { logger } from '@/lib/logger';
import { getLL } from '@/i18n/i18n-lazy';
let initialized = false;

/**
 * 判断是否为良性的 ResizeObserver 循环通知错误
 * 该错误由浏览器渲染循环防止死循环机制触发，不会影响应用功能与渲染，需静默忽略。
 */
export function isResizeObserverLoopError(error: unknown, message?: string): boolean {
  let text = '';
  if (typeof message === 'string' && message) {
    text = message;
  } else if (typeof error === 'string') {
    text = error;
  } else if (error instanceof Error) {
    text = error.message;
  } else if (error && typeof error === 'object' && 'message' in error && typeof error.message === 'string') {
    text = error.message;
  }

  return (
    text.includes('ResizeObserver loop completed with undelivered notifications') ||
    text.includes('ResizeObserver loop limit exceeded')
  );
}

export function setupGlobalErrorHandlers(): void {
  if (initialized || typeof window === 'undefined') return;
  initialized = true;

  window.addEventListener('error', (event: ErrorEvent) => {
    if (isResizeObserverLoopError(event.error, event.message)) {
      event.stopImmediatePropagation?.();
      event.preventDefault?.();
      return;
    }

    const error = event.error || event.message;
    logger.error('Uncaught window error:', error);
    const title = getLL().flow?.runtime_error?.() ?? 'Runtime Error';
    notifyError(title, error, {
      category: 'Runtime',
    });
  });
  window.addEventListener('unhandledrejection', (event: PromiseRejectionEvent) => {
    if (isResizeObserverLoopError(event.reason)) {
      event.stopImmediatePropagation?.();
      event.preventDefault?.();
      return;
    }

    const reason = event.reason;
    logger.error('Unhandled promise rejection:', reason);
    const title = getLL().flow?.unhandled_rejection?.() ?? 'Unhandled Promise Rejection';
    notifyError(title, reason, {
      category: 'Runtime',
    });
  });
}
