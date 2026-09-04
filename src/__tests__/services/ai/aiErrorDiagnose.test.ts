import { describe, it, expect } from 'vitest';
import { diagnoseAIError } from '@/services/ai/aiErrorDiagnose';

describe('aiErrorDiagnose', () => {
  it('正确识别流式传输中断/连接重置', () => {
    const err = '[Data stream interrupted (connection broken or reset by remote server or proxy)] error sending request for url (https://api.openai.com): connection closed before message completed';
    const diag = diagnoseAIError(err);
    expect(diag.kind).toBe('stream_interrupted');
    expect(diag.retryable).toBe(true);
    expect(diag.diagnosticText).toBeTruthy();
  });

  it('正确识别 connection reset by peer', () => {
    const err = 'error sending request for url: connection reset by peer';
    const diag = diagnoseAIError(err);
    expect(diag.kind).toBe('stream_interrupted');
    expect(diag.retryable).toBe(true);
  });

  it('正确识别连接失败与代理拒绝', () => {
    const err = '[Failed to connect to host (check network, DNS, or proxy settings)] connection refused (os error 111)';
    const diag = diagnoseAIError(err);
    expect(diag.kind).toBe('connect_failed');
    expect(diag.retryable).toBe(true);
  });

  it('正确识别超时错误', () => {
    const err = '[Request timed out] operation timed out';
    const diag = diagnoseAIError(err);
    expect(diag.kind).toBe('timeout');
    expect(diag.retryable).toBe(true);
  });

  it('正确识别 401 未授权错误并标记不可直接重试', () => {
    const err = 'HTTP 401: Invalid API Key provided';
    const diag = diagnoseAIError(err);
    expect(diag.kind).toBe('unauthorized');
    expect(diag.retryable).toBe(false);
  });

  it('正确识别 429 速率超限错误', () => {
    const err = 'HTTP 429: Rate limit reached for requests';
    const diag = diagnoseAIError(err);
    expect(diag.kind).toBe('rate_limit');
    expect(diag.retryable).toBe(true);
  });

  it('其他未知错误走兜底', () => {
    const err = 'Something completely bizarre happened';
    const diag = diagnoseAIError(err);
    expect(diag.kind).toBe('general');
    expect(diag.retryable).toBe(true);
  });
});
