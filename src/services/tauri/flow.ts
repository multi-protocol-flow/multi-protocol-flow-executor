/**
 * Tauri IPC 流程相关调用
 * 封装 invoke 调用，添加统一错误处理
 */

import { invoke, Channel } from '@tauri-apps/api/core';
import type { Flow } from '@/types/flow';
import type { Node } from '@/types/node';
import type { NodeExecutionReport } from '@/types/report';
import type {
  CreateFlowCommandInput,
  CreateFlowCommandOutput,
  ExecuteFlowCommandInput,
  ExecuteFlowCommandOutput,
  ValidationResult,
} from '@/types/ipc';
import type { LogEvent } from '@/types/events';
import type { ExecutionEvent } from '@/types/execution_event';
import { getLL } from '@/i18n/i18n-lazy';

/**
 * IPC 错误类
 * 包含命令名称、错误码和原始错误原因
 */
export class IpcError extends Error {
  constructor(
    message: string,
    public readonly command: string,
    public readonly cause?: unknown,
    /** 可选错误码（如 E_TIMEOUT），用于 i18n 本地化 */
    public readonly code?: string
  ) {
    super(message);
    this.name = 'IpcError';
  }
}

/**
 * 根据错误码解析本地化错误消息
 * 调用 LL().errors[code](args) 获取翻译后的消息
 * 若错误码不存在或查询失败，返回原始 error code 作为 fallback
 *
 * @param code - 错误码（如 E_TIMEOUT、E_HTTP_REQUEST_FAILED）
 * @param args - 模板参数（如 { url, status }），与 errors.ts 中的 RequiredParams 对齐
 * @returns 本地化错误消息
 */
export function resolveErrorCode(code: string, args?: Record<string, string>): string {
  try {
    // LL.errors exists at runtime via custom-types.ts augmentation on RootTranslation,
    // but TranslationFunctions (auto-generated type alias) does not have the `errors` key.
    // We cast through unknown to access it at runtime with minimal type assertion surface.
    const LL = getLL() as unknown as { errors: Record<string, (a?: Record<string, string>) => string> };
    const fn = LL.errors[code];
    if (typeof fn === 'function') {
      return fn(args);
    }
  } catch {
    // 忽略异常，返回 code 作为 fallback
  }
  return code;
}

/**
 * 格式化错误信息
 * 检测 IPC 返回的结构化错误（含 error_code），自动提取并传递
 * @internal 导出供其他 IPC 模块使用
 */
export function formatError(error: unknown, command: string): IpcError {
  if (typeof error === 'string') {
    return new IpcError(error, command);
  }
  if (error instanceof Error) {
    return new IpcError(error.message, command, error);
  }
  // 结构化错误：检查 error_code 字段
  if (error && typeof error === 'object') {
    const errObj = error as Record<string, unknown>;
    const errorCode = typeof errObj.error_code === 'string' ? errObj.error_code : undefined;
    const message =
      typeof errObj.message === 'string'
        ? errObj.message
        : errorCode
          ? `Error: ${errorCode}`
          : `Unknown error: ${String(error)}`;
    return new IpcError(message, command, error, errorCode);
  }
  return new IpcError(`Unknown error: ${String(error)}`, command, error);
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

/**
 * 验证流程结构
 * 调用后端 validate_flow IPC 命令
 *
 * @param flow - 要验证的流程
 * @returns 验证结果（valid/errors/warnings）
 * @throws IpcError 如果验证调用失败
 */
export async function validateFlow(flow: Flow): Promise<ValidationResult> {
  try {
    return await invoke<ValidationResult>('validate_flow', { flow });
  } catch (error) {
    throw formatError(error, 'validate_flow');
  }
}

/**
 * 执行单个节点（连接性验证）
 * 调用后端 execute_single_node IPC 命令
 *
 * 支持的节点类型: http, script, assertion, redis_connect, db_connect,
 * mqtt_connect, kafka_connect, amqp_connect, amqp10_connect,
 * smtp_connect, imap_connect, ws_connect, sse_connect, grpc_connect,
 * mcp_connect, graphql_connect, tcp_connect
 *
 * @param node - 要执行的节点
 * @param variables - 可选的变量
 * @returns 节点执行报告
 * @throws IpcError 如果执行失败
 */
export async function executeSingleNode(
  node: Node,
  variables?: Record<string, unknown>
): Promise<NodeExecutionReport> {
  try {
    return await invoke<NodeExecutionReport>('execute_single_node', {
      node,
      variables: variables ?? null,
    });
  } catch (error) {
    throw formatError(error, 'execute_single_node');
  }
}
