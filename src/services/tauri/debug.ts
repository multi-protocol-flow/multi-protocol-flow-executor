/**
 * Tauri IPC 调试相关调用
 * 封装调试命令的 invoke 调用
 */

import { invoke, Channel } from '@tauri-apps/api/core';
import type { Flow } from '@/types/flow';
import type { DebugEvent, DebugCommand } from '@/types/debug';
import { IpcError } from './flow';

/**
 * 格式化错误信息
 */
function formatError(error: unknown, command: string): IpcError {
  if (typeof error === 'string') {
    return new IpcError(error, command);
  }
  if (error instanceof Error) {
    return new IpcError(error.message, command, error);
  }
  return new IpcError(`未知错误: ${String(error)}`, command, error);
}

/**
 * 启动调试会话
 * 使用 Channel API 接收调试事件
 *
 * @param flow - 要调试的流程对象
 * @param breakpoints - 断点列表（节点 UUID）
 * @param onEvent - 调试事件回调函数
 * @throws IpcError 如果启动失败
 */
export async function startDebug(
  flow: Flow,
  breakpoints: string[],
  onEvent: (event: DebugEvent) => void
): Promise<void> {
  try {
    // 创建 Channel 用于接收调试事件
    const eventChannel = new Channel<DebugEvent>();
    eventChannel.onmessage = (event) => {
      onEvent(event);
    };

    await invoke<void>('start_debug', {
      flow,
      breakpoints,
      onEvent: eventChannel,
    });
  } catch (error) {
    throw formatError(error, 'start_debug');
  }
}

/**
 * 发送调试命令
 *
 * @param command - 调试命令
 * @throws IpcError 如果发送失败
 */
export async function sendDebugCommand(command: DebugCommand): Promise<void> {
  try {
    await invoke<void>('send_debug_command', { command });
  } catch (error) {
    throw formatError(error, 'send_debug_command');
  }
}

/**
 * 结束调试会话
 *
 * @throws IpcError 如果结束失败
 */
export async function endDebug(): Promise<void> {
  try {
    await invoke<void>('end_debug');
  } catch (error) {
    throw formatError(error, 'end_debug');
  }
}
