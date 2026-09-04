/**
 * AI 请求异常智能诊断工具
 * 分析底层网络、流式传输、身份校验等错误，为用户提供通俗易懂的故障原因与排查建议
 */

import { getLL } from '@/i18n/i18n-lazy';

export type AIErrorKind =
  | 'stream_interrupted'
  | 'connect_failed'
  | 'timeout'
  | 'unauthorized'
  | 'rate_limit'
  | 'general';

export interface AIDiagnosisResult {
  kind: AIErrorKind;
  /** 通俗易懂的原因诊断与排查建议 */
  diagnosticText: string;
  /** 是否建议直接点击重试 (例如流式中断、网络抖动、超时可直接重试；401/403 则提示检查配置) */
  retryable: boolean;
  /** 完整的原始底层错误信息 */
  rawError: string;
}

/**
 * 诊断 AI 请求报错原因并生成友好的排查指引
 */
export function diagnoseAIError(error: unknown): AIDiagnosisResult {
  const rawError =
    error instanceof Error
      ? error.message
      : typeof error === 'string'
        ? error
        : JSON.stringify(error) || 'Unknown error';

  const lower = rawError.toLowerCase();
  const ll = getLL();

  // 1. 流式数据中途断开 / 连接重置
  if (
    lower.includes('data stream interrupted') ||
    lower.includes('stream chunk error') ||
    lower.includes('connection closed before message completed') ||
    lower.includes('connection reset') ||
    lower.includes('broken pipe') ||
    lower.includes('stream broken') ||
    lower.includes('prematurely')
  ) {
    return {
      kind: 'stream_interrupted',
      diagnosticText: ll.ai.copilot.error_diag_stream_interrupted(),
      retryable: true,
      rawError,
    };
  }

  // 2. 无法建立连接 (DNS / Proxy / 拒绝连接)
  if (
    lower.includes('failed to connect') ||
    lower.includes('connection refused') ||
    lower.includes('dns') ||
    lower.includes('lookup address') ||
    lower.includes('proxy') ||
    lower.includes('enotfound') ||
    lower.includes('econnrefused')
  ) {
    return {
      kind: 'connect_failed',
      diagnosticText: ll.ai.copilot.error_diag_connect_failed(),
      retryable: true,
      rawError,
    };
  }

  // 3. 超时 (请求超时或连接超时)
  if (
    lower.includes('timed out') ||
    lower.includes('timeout') ||
    lower.includes('etimedout')
  ) {
    return {
      kind: 'timeout',
      diagnosticText: ll.ai.copilot.error_diag_timeout(),
      retryable: true,
      rawError,
    };
  }

  // 4. 未授权 / API Key 错误
  if (
    lower.includes('401') ||
    lower.includes('unauthorized') ||
    lower.includes('invalid api key') ||
    lower.includes('incorrect api key')
  ) {
    return {
      kind: 'unauthorized',
      diagnosticText: ll.ai.copilot.error_diag_unauthorized(),
      retryable: false,
      rawError,
    };
  }

  // 5. 频率限制 / 配额耗尽
  if (
    lower.includes('429') ||
    lower.includes('rate limit') ||
    lower.includes('quota') ||
    lower.includes('insufficient_quota')
  ) {
    return {
      kind: 'rate_limit',
      diagnosticText: ll.ai.copilot.error_diag_rate_limit(),
      retryable: true,
      rawError,
    };
  }

  // 6. 通用错误兜底
  return {
    kind: 'general',
    diagnosticText: ll.ai.copilot.error_diag_general(),
    retryable: true,
    rawError,
  };
}
