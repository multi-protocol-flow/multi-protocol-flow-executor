/**
 * Tauri 2 Channel API 事件订阅服务
 * 用于接收后端执行器的日志事件
 */

import { invoke, Channel } from '@tauri-apps/api/core';
import type { LogEvent } from '@/types/events';

/**
 * 创建日志订阅 Channel
 *
 * @param onLog - 日志回调函数
 * @returns channel 对象（传递给 invoke）和 cleanup 函数
 */
export function subscribeToFlowLogs(
  onLog: (event: LogEvent) => void
): { channel: Channel<LogEvent>; cleanup: () => void } {
  const channel = new Channel<LogEvent>();
  channel.onmessage = onLog;

  return {
    channel,
    cleanup: () => {
      // Channel 会随命令结束自动清理，无需手动操作
    },
  };
}

/**
 * 执行流程并订阅日志（完整示例）
 *
 * @param input - 执行输入参数
 * @param flow - 流程对象
 * @param onLog - 日志回调函数
 */
export async function executeFlowWithLogs(
  input: { flowId: string; variables?: Record<string, unknown> },
  flow: unknown,
  onLog: (event: LogEvent) => void
): Promise<void> {
  const { channel } = subscribeToFlowLogs(onLog);

  // 注意：参数名必须是 camelCase (onEvent)，对应 Rust 的 snake_case (on_event)
  await invoke('execute_flow', {
    input,
    flow,
    onEvent: channel, // ⚠️ camelCase，不是 on_event
  });
}
