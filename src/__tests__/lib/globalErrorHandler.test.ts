/**
 * globalErrorHandler 测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setupGlobalErrorHandlers, isResizeObserverLoopError } from '@/lib/globalErrorHandler';
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

  it('静默忽略 ResizeObserver loop completed with undelivered notifications 错误', () => {
    setupGlobalErrorHandlers();

    const event = new ErrorEvent('error', {
      message: 'ResizeObserver loop completed with undelivered notifications.',
    });
    window.dispatchEvent(event);

    expect(notifyModule.notifyError).not.toHaveBeenCalled();
  });

  it('静默忽略 ResizeObserver loop limit exceeded 错误', () => {
    setupGlobalErrorHandlers();

    const event = new ErrorEvent('error', {
      message: 'ResizeObserver loop limit exceeded',
    });
    window.dispatchEvent(event);

    expect(notifyModule.notifyError).not.toHaveBeenCalled();
  });

  it('isResizeObserverLoopError 正确识别各种输入形态', () => {
    expect(isResizeObserverLoopError(null, 'ResizeObserver loop completed with undelivered notifications.')).toBe(true);
    expect(isResizeObserverLoopError('ResizeObserver loop limit exceeded')).toBe(true);
    expect(isResizeObserverLoopError(new Error('ResizeObserver loop completed with undelivered notifications.'))).toBe(true);
    expect(isResizeObserverLoopError({ message: 'ResizeObserver loop limit exceeded' })).toBe(true);
    expect(isResizeObserverLoopError(new Error('Normal error'))).toBe(false);
    expect(isResizeObserverLoopError(null, 'Normal error message')).toBe(false);
  });
});
