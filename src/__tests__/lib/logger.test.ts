/**
 * Logger 模块测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createLogger, logger } from '@/lib/logger';

describe('logger', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('在开发环境下正常调用 console.debug/info/warn/error', () => {
    const debugSpy = vi.spyOn(console, 'debug').mockImplementation(() => {});
    const infoSpy = vi.spyOn(console, 'info').mockImplementation(() => {});
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const testLogger = createLogger('TestModule');

    testLogger.debug('debug 调试信息');
    expect(debugSpy).toHaveBeenCalledWith('[TestModule] debug 调试信息');

    testLogger.info('info 提示信息', { user: 'test' });
    expect(infoSpy).toHaveBeenCalledWith('[TestModule] info 提示信息', { user: 'test' });

    testLogger.warn('warn 警告信息');
    expect(warnSpy).toHaveBeenCalledWith('[TestModule] warn 警告信息');

    testLogger.error('error 错误信息', new Error('boom'));
    expect(errorSpy).toHaveBeenCalled();
  });

  it('默认导出的 logger 实例 tag 为 App', () => {
    const infoSpy = vi.spyOn(console, 'info').mockImplementation(() => {});
    logger.info('app message');
    expect(infoSpy).toHaveBeenCalledWith('[App] app message');
  });
});
