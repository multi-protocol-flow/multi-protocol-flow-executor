/**
 * SSE 服务层 — SSE 协议相关的 Tauri IPC 调用
 */

import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// 类型定义
// ============================================================================

/** SSE 测试连接请求参数 */
export interface SseTestConnectionInput {
  /** SSE 端点 URL（如 https://example.com/events） */
  url: string;
}

// ============================================================================
// IPC 调用
// ============================================================================

/**
 * 测试 SSE 端点连接
 *
 * 验证 SSE 端点 URL 格式有效性。
 *
 * @param input - 测试连接请求参数
 * @returns true 如果 URL 格式有效
 */
export async function testSseConnection(
  input: SseTestConnectionInput
): Promise<boolean> {
  return invoke<boolean>('sse_test_connection', { url: input.url });
}
