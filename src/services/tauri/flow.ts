/**
 * Tauri IPC 流程相关调用
 * 封装 invoke 调用，添加统一错误处理
 */

import { invoke, Channel } from '@tauri-apps/api/core';
import type { Flow } from '@/types/flow';
import type { NodeExecutionReport } from '@/types/report';
import type {
  CreateFlowCommandInput,
  CreateFlowCommandOutput,
  ExecuteFlowCommandInput,
  ExecuteFlowCommandOutput,
  ValidationResult,
} from '@/types/ipc';
import type { LogEvent } from '@/types/events';
import type { DebugEvent } from '@/types/debug';
import type { ExecutionEvent } from '@/types/execution_event';
import { toast } from 'sonner';
import { isTauriEnv } from '@/lib/platform';
import { notifyError } from '@/lib/notify';
import { getActiveVariables } from '@/types/flow';
import {
  useExecutionStore,
  useDebugStore,
  useUIStore,
} from '@/stores';
import { useConsoleStore } from '@/stores/console';
import { useWsStreamStore } from '@/stores/wsStream';
import { getLL } from '@/i18n/i18n-lazy';
import { useEnvironmentStore } from '@/stores/environment';
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
  variables?: Record<string, unknown>,
  reportsDir?: string
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

    // 当前激活的全局环境：由后端按「流程内置变量 → 环境文件 → 运行时变量」顺序叠加
    const environmentName = useEnvironmentStore.getState().activeEnvironmentName;

    // 调用后端命令，传递三 Channel
    return await invoke<ExecuteFlowCommandOutput>('execute_flow', {
      input: {
        flow_id: flow.uuid,
        variables: variables || null,
        ...(reportsDir ? { reports_dir: reportsDir } : {}),
        ...(environmentName ? { environment_name: environmentName } : {}),
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
 * 获取指定节点的完整执行详情（冷通道按需拉取）
 *
 * @param flowUuid - 流程 UUID
 * @param nodeUuid - 节点 UUID
 * @returns 节点执行报告详情，若无记录则返回 null
 */
export async function getNodeExecutionDetail(
  flowUuid: string,
  nodeUuid: string
): Promise<NodeExecutionReport | null> {
  try {
    return await invoke<NodeExecutionReport | null>('get_node_execution_detail', {
      flowUuid,
      nodeUuid,
    });
  } catch (error) {
    throw formatError(error, 'get_node_execution_detail');
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
 * 构建流程切片子流程
 *
 * 提取目标节点及其内部相互连线，生成独立的轻量临时子流程用于局部交互测试。
 * 跳过全局前置 Setup 与后置 Teardown，避免非预期的预热与清理开销。
 *
 * @param flow - 原始完整流程
 * @param targetNodeUuids - 待测试的目标节点 UUID 列表
 * @param sliceName - 可选的切片名称覆盖
 * @returns 构造的子流程
 */
export function buildFlowSlice(flow: Flow, targetNodeUuids: string[], sliceName?: string): Flow {
  const targetSet = new Set(targetNodeUuids);
  const allNodes = [
    ...(flow.setup?.nodes ?? []),
    ...flow.nodes,
    ...(flow.teardown?.nodes ?? []),
  ];
  const allConnections = [
    ...(flow.setup?.connections ?? []),
    ...flow.connections,
    ...(flow.teardown?.connections ?? []),
  ];
  const nodes = allNodes.filter((n) => targetSet.has(n.uuid));
  const connections = allConnections.filter(
    (c) => targetSet.has(c.source_node_uuid) && targetSet.has(c.target_node_uuid)
  );
  return {
    ...flow,
    uuid: `slice-${Date.now()}`,
    name: sliceName || (nodes.length === 1 ? `Test: ${nodes[0]?.name || ''}` : `Slice (${nodes.length} nodes)`),
    nodes,
    connections,
    setup: undefined,
    teardown: undefined,
  };
}

/**
 * 启动交互式切片/单节点执行服务
 *
 * 复用统一流程执行通道 execute_flow 与底部 running 运行面板，
 * 支持思维链流式输出与人机协同卡片（ask_human）。
 * 执行报告仅驻留内存供即时查阅，不落盘持久化。
 *
 * @param flow - 当前完整流程
 * @param targetNodeUuids - 待执行的目标节点 UUID 数组
 */
export async function startInteractiveSliceExecution(
  flow: Flow,
  targetNodeUuids: string[]
): Promise<void> {
  if (useExecutionStore.getState().isExecuting) {
    toast.warning(getLL().flow.debug_unavailable_executing());
    return;
  }
  const sliceFlow = buildFlowSlice(flow, targetNodeUuids);
  const flowVariables = getActiveVariables(flow);
  useExecutionStore.getState().clearExecutionState();
  useDebugStore.getState().clearDebugData();
  useConsoleStore.getState().clearLogs();
  useUIStore.setState({ isBottomPanelExpanded: true, bottomPanelTab: 'running' });
  useExecutionStore.getState().startExecution({
    flow_uuid: sliceFlow.uuid,
    total_nodes: sliceFlow.nodes.length,
  });

  const handleLog = (event: LogEvent) => {
    useConsoleStore.getState().addLog(event);
  };
  const handleExecutionEvent = (event: ExecutionEvent) => {
    useExecutionStore.getState().handleExecutionEvent(event);
  };
  const handleWsEvent = (event: DebugEvent) => {
    useWsStreamStore.getState().handleWsEvent(event);
  };

  try {
    const executeFn = isTauriEnv()
      ? executeFlowWithChannel
      : (await import('./mock')).executeFlowWithChannel;

    const result = await executeFn(
      sliceFlow,
      handleLog,
      handleExecutionEvent,
      handleWsEvent,
      flowVariables,
      undefined
    );

    if (result.flow_report) {
      useExecutionStore.getState().setReport(result.flow_report);
    }

    if (result.success) {
      toast.success(getLL().flow.slice_test_completed());
    } else {
      const errMsg = result.error || getLL().flow.slice_test_failed();
      useExecutionStore.getState().setError(errMsg);
      toast.error(getLL().flow.slice_test_failed(), {
        description: errMsg,
      });
    }
  } catch (error) {
    const errorMsg = (error as Error).message;
    useExecutionStore.getState().setError(errorMsg);
    notifyError(getLL().flow.slice_test_failed(), error, {
      category: 'Execution',
      flowUuid: sliceFlow.uuid,
    });
  } finally {
    useConsoleStore.getState().flushLogs();
    useExecutionStore.setState({ isExecuting: false });
  }
}
/**
 * 启动 Ingress Mock 监听服务
 *
 * `debug` 为 true 时，外部请求经由可调试执行器运行：流程内断点生效，
 * HTTP 连接保持挂起，直到开发者单步执行到 `http:respond` 才返回响应；
 * 调试事件与日志经同一组 Channel 推送到现有调试面板。
 */
export async function startIngressListener(
  port: number,
  flow: Flow,
  debug: boolean,
  onEvent: (event: DebugEvent) => void,
  onLog: (event: LogEvent) => void
): Promise<string | null> {
  try {
    const eventChannel = new Channel<DebugEvent>();
    eventChannel.onmessage = (event) => {
      onEvent(event);
    };

    const logChannel = new Channel<LogEvent>();
    logChannel.onmessage = (event) => {
      onLog(event);
    };

    const replaced = await invoke<string | null>('start_ingress_listener', {
      port,
      flow,
      debug,
      onEvent: eventChannel,
      onLog: logChannel,
    });
    return replaced ?? null;
  } catch (error) {
    throw formatError(error, 'start_ingress_listener');
  }
}

/**
 * 停止 Ingress Mock 监听服务
 */
export async function stopIngressListener(port: number): Promise<void> {
  try {
    await invoke('stop_ingress_listener', { port });
  } catch (error) {
    throw formatError(error, 'stop_ingress_listener');
  }
}

/**
 * 获取活跃的 Ingress 监听端口
 */
export async function getActiveIngressPorts(): Promise<number[]> {
  try {
    return await invoke<number[]>('get_active_ingress_ports');
  } catch (error) {
    throw formatError(error, 'get_active_ingress_ports');
  }
}
