/**
 * AMQP 服务层 — AMQP 协议相关的 Tauri IPC 调用
 */

import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// 类型定义
// ============================================================================

/** AMQP 测试连接请求参数 */
export interface AmqpTestConnectionInput {
  /** AMQP Broker 地址（如 amqp://localhost:5672） */
  url: string;
}

// ============================================================================
// IPC 调用
// ============================================================================

/**
 * 测试 AMQP Broker 连接
 *
 * 验证 AMQP Broker URL 格式有效性。
 *
 * @param input - 测试连接请求参数
 * @returns true 如果 URL 格式有效
 */
export async function testAmqpConnection(
  input: AmqpTestConnectionInput
): Promise<boolean> {
  return invoke<boolean>('amqp_test_connection', { url: input.url });
}
