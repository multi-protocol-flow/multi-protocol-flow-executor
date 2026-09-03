/**
 * 全局未捕获异常与 Promise Rejection 拦截器
 * 自动捕获运行时崩溃，记入本地日志 (app.log)，并通过 Console 系统日志和 Toast 进行反馈。
 */
import { notifyError } from '@/lib/notify';
import { logger } from '@/lib/logger';
import { getLL } from '@/i18n/i18n-lazy';
let initialized = false;

export function setupGlobalErrorHandlers(): void {
  if (initialized || typeof window === 'undefined') return;
  initialized = true;

  window.addEventListener('error', (event: ErrorEvent) => {
    const error = event.error || event.message;
    logger.error('Uncaught window error:', error);
    const title = getLL().flow?.runtime_error?.() ?? 'Runtime Error';
    notifyError(title, error, {
      category: 'Runtime',
    });
  });

  window.addEventListener('unhandledrejection', (event: PromiseRejectionEvent) => {
    const reason = event.reason;
    logger.error('Unhandled promise rejection:', reason);
    const title = getLL().flow?.unhandled_rejection?.() ?? 'Unhandled Promise Rejection';
    notifyError(title, reason, {
      category: 'Runtime',
    });
  });
}
