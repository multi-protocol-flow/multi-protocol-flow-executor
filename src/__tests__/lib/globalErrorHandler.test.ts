/**
 * globalErrorHandler 测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setupGlobalErrorHandlers } from '@/lib/globalErrorHandler';
import * as notifyModule from '@/lib/notify';

vi.mock('@/lib/notify', () => ({
  notifyError: vi.fn(),
  notifySuccess: vi.fn(),
  notifyWarning: vi.fn(),
  notifyInfo: vi.fn(),
}));

describe('globalErrorHandler', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('监听 window error 事件并调用 notifyError', () => {
    setupGlobalErrorHandlers();

    const err = new Error('Script crash');
    const event = new ErrorEvent('error', { error: err, message: 'Script crash' });
    window.dispatchEvent(event);

    expect(notifyModule.notifyError).toHaveBeenCalledWith(
      '前端运行时异常',
      err,
      expect.objectContaining({ category: 'Runtime' })
    );
  });

  it('监听 window unhandledrejection 事件并调用 notifyError', () => {
    setupGlobalErrorHandlers();

    const reason = new Error('Rejected promise');
    const event = new PromiseRejectionEvent('unhandledrejection', {
      promise: Promise.reject(reason).catch(() => {}),
      reason,
    });
    window.dispatchEvent(event);

    expect(notifyModule.notifyError).toHaveBeenCalledWith(
      '未处理的异步异常',
      reason,
      expect.objectContaining({ category: 'Runtime' })
    );
  });
});
