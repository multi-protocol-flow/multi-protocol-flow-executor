/**
 * Tauri IPC 流程相关调用
 * 封装 invoke 调用，添加统一错误处理
 */

import { invoke, Channel } from '@tauri-apps/api/core';
import type { Flow } from '@/types/flow';
import type {
  CreateFlowCommandInput,
  CreateFlowCommandOutput,
  ExecuteFlowCommandInput,
  ExecuteFlowCommandOutput,
} from '@/types/ipc';
import type { LogEvent } from '@/types/events';
import type { ExecutionEvent } from '@/types/execution_event';

/**
 * IPC 错误类
 */
export class IpcError extends Error {
  constructor(
    message: string,
    public readonly command: string,
    public readonly cause?: unknown
  ) {
    super(message);
    this.name = 'IpcError';
  }
}

/**
 * 格式化错误信息
 * @internal 导出供其他 IPC 模块使用
 */
export function formatError(error: unknown, command: string): IpcError {
  if (typeof error === 'string') {
    return new IpcError(error, command);
  }
  if (error instanceof Error) {
    return new IpcError(error.message, command, error);
  }
  return new IpcError(`未知错误: ${String(error)}`, command, error);
}

/**
 * 创建流程
 * 调用后端 create_flow IPC 命令
 *
 * @param name - 流程名称
 * @returns 创建的流程对象
 * @throws IpcError 如果创建失败
 */
export async function createFlow(name: string): Promise<Flow> {
  try {
    const result = await invoke<CreateFlowCommandOutput>('create_flow', {
      input: { name } as CreateFlowCommandInput,
    });
    return result.flow;
  } catch (error) {
    throw formatError(error, 'create_flow');
  }
}

/**
 * 执行流程并订阅日志和执行事件（Tauri 环境）
 * 使用三 Channel API 接收后端日志事件、实时执行事件和 WS 流事件
 *
 * @param flow - 要执行的流程对象
 * @param onLog - 日志回调函数
 * @param onExecutionEvent - 执行事件回调函数（实时节点报告）
 * @param onWsEvent - WS 流事件回调函数（DebugEvent 中的 WS 变体）
 * @param variables - 可选的执行变量
 * @returns 执行结果
 * @throws IpcError 如果执行失败
 */
export async function executeFlowWithChannel(
  flow: Flow,
  onLog: (event: LogEvent) => void,
  onExecutionEvent: (event: ExecutionEvent) => void = () => {},
  onWsEvent: (event: import('@/types/debug').DebugEvent) => void = () => {},
  variables?: Record<string, unknown>
): Promise<ExecuteFlowCommandOutput> {
  try {
    // Channel 1: Log events
    const logChannel = new Channel<LogEvent>();
    logChannel.onmessage = onLog;

    // Channel 2: Execution events (real-time node reports)
    const executionEventChannel = new Channel<ExecutionEvent>();
    executionEventChannel.onmessage = onExecutionEvent;

    // Channel 3: WS stream events (DebugEvent variants for WS streams)
    const wsEventChannel = new Channel<import('@/types/debug').DebugEvent>();
    wsEventChannel.onmessage = onWsEvent;

    // 调用后端命令，传递三 Channel
    return await invoke<ExecuteFlowCommandOutput>('execute_flow', {
      input: {
        flow_id: flow.uuid,
        variables: variables || null,
      } as ExecuteFlowCommandInput,
      flow,
      onEvent: logChannel,
      onExecutionEvent: executionEventChannel,
      onWsEvent: wsEventChannel,
    });
  } catch (error) {
    throw formatError(error, 'execute_flow');
  }
}

/**
 * 取消正在执行的流程
 * 调用后端 cancel_flow IPC 命令
 *
 * @throws IpcError 如果取消失败
 */
export async function cancelFlow(): Promise<void> {
  try {
    await invoke<void>('cancel_flow');
  } catch (error) {
    throw formatError(error, 'cancel_flow');
  }
}
