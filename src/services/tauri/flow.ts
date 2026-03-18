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
import { useConnectionsStore } from '@/stores/connections';

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
 * 执行流程并订阅日志（Tauri 环境）
 * 使用 Channel API 接收后端日志事件
 *
 * @param flow - 要执行的流程对象
 * @param onLog - 日志回调函数
 * @param variables - 可选的执行变量
 * @returns 执行结果
 * @throws IpcError 如果执行失败
 */
export async function executeFlowWithChannel(
  flow: Flow,
  onLog: (event: LogEvent) => void,
  variables?: Record<string, unknown>
): Promise<ExecuteFlowCommandOutput> {
  try {
    // 创建 Channel 订阅日志
    const channel = new Channel<LogEvent>();
    channel.onmessage = onLog;

    // 从连接配置中心获取所有数据库连接配置
    // 执行器根据 connection_uuid 从此列表获取完整的连接配置
    const dbConnections = useConnectionsStore.getState().connections;

    // 调用后端命令，传递 Channel
    return await invoke<ExecuteFlowCommandOutput>('execute_flow', {
      input: {
        flow_id: flow.uuid,
        variables: variables || null,
        db_connections: dbConnections,
      } as ExecuteFlowCommandInput,
      flow,
      onEvent: channel, // 关键：传递 Channel（camelCase 对应 Rust 的 snake_case）
    });
  } catch (error) {
    throw formatError(error, 'execute_flow');
  }
}
