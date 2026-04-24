/**
 * Tauri IPC 服务主导出文件
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 */

import type { Flow } from '@/types/flow';
import type { ExecuteFlowCommandOutput } from '@/types/ipc';
import { invoke } from '@tauri-apps/api/core';
import type { FlowExecutionReport } from '@/types/report';
import type { ReportMetadata } from '@/types/ipc';

// 检测是否在 Tauri 环境中运行
function isTauriEnv(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

// 动态导入正确的服务
async function getService() {
  if (isTauriEnv()) {
    // 在 Tauri 环境中使用真实 IPC
    const { createFlow, IpcError } = await import('./flow');
    return { createFlow, IpcError };
  } else {
    // 在非 Tauri 环境中使用 Mock
    const { createFlow } = await import('./mock');
    // Mock 环境中返回一个简单的错误类
    class IpcError extends Error {
      constructor(
        message: string,
        public readonly command: string,
        public readonly cause?: unknown
      ) {
        super(message);
        this.name = 'IpcError';
      }
    }
    return { createFlow, IpcError };
  }
}

// 缓存服务实例
let servicePromise: ReturnType<typeof getService> | null = null;

/**
 * 获取服务实例（带缓存）
 */
async function getCachedService() {
  if (!servicePromise) {
    servicePromise = getService();
  }
  return servicePromise;
}

/**
 * 创建流程
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param name - 流程名称
 * @returns 创建的流程对象
 */
export async function createFlow(name: string): Promise<Flow> {
  const { createFlow: createFlowImpl } = await getCachedService();
  return createFlowImpl(name);
}

/**
 * 执行流程并订阅日志
 * 使用 Channel API 接收后端日志事件、执行事件和 WS 流事件
 *
 * @param flow - 要执行的流程对象
 * @param onLog - 日志回调函数
 * @param onExecutionEvent - 执行事件回调函数（实时节点报告）
 * @param onWsEvent - WS 流事件回调函数（DebugEvent 中的 WS 变体）
 * @param variables - 可选的执行变量
 * @returns 执行结果
 */
export async function executeFlowWithChannel(
  flow: Flow,
  onLog: (event: import('@/types/events').LogEvent) => void,
  onExecutionEvent: (event: import('@/types/execution_event').ExecutionEvent) => void = () => {},
  onWsEvent: (event: import('@/types/debug').DebugEvent) => void = () => {},
  variables?: Record<string, unknown>
): Promise<ExecuteFlowCommandOutput> {
  // 检测是否在 Tauri 环境中
  if (isTauriEnv()) {
    const { executeFlowWithChannel: executeFlowWithChannelImpl } = await import('./flow');
    return executeFlowWithChannelImpl(flow, onLog, onExecutionEvent, onWsEvent, variables);
  } else {
    // Mock 环境
    const { executeFlowWithChannel: executeFlowWithChannelImpl } = await import('./mock');
    return executeFlowWithChannelImpl(flow, onLog, onExecutionEvent, onWsEvent, variables);
  }
}

// ============================================================================
// 报告存储命令
// ============================================================================

/**
 * 保存流程执行报告
 * @param report 流程执行报告
 * @param reportsDir 报告存储目录
 */
export async function saveFlowReport(report: FlowExecutionReport, reportsDir: string): Promise<void> {
  return await invoke<void>('save_flow_report', { report, reportsDir });
}

/**
 * 加载流程执行报告
 * @param uuid 报告 UUID
 * @param reportsDir 报告存储目录
 */
export async function loadFlowReport(uuid: string, reportsDir: string): Promise<FlowExecutionReport> {
  return await invoke<FlowExecutionReport>('load_flow_report', { uuid, reportsDir });
}

/**
 * 列出所有报告元数据
 * @param reportsDir 报告存储目录
 */
export async function listFlowReports(reportsDir: string): Promise<ReportMetadata[]> {
  return await invoke<ReportMetadata[]>('list_flow_reports', { reportsDir });
}

/**
 * 删除流程执行报告
 * @param uuid 报告 UUID
 * @param reportsDir 报告存储目录
 */
export async function deleteFlowReport(uuid: string, reportsDir: string): Promise<void> {
  return await invoke<void>('delete_flow_report', { uuid, reportsDir });
}

// ============================================================================
// 调试命令
// ============================================================================

/**
 * 启动调试会话
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param flow - 要调试的流程对象
 * @param breakpoints - 断点列表（节点 UUID）
 * @param onEvent - 调试事件回调函数
 * @param onLog - 日志回调函数
 */
export async function startDebug(
  flow: import('@/types/flow').Flow,
  breakpoints: string[],
  onEvent: (event: import('@/types/debug').DebugEvent) => void,
  onLog: (event: import('@/types/events').LogEvent) => void
): Promise<void> {
  if (isTauriEnv()) {
    const { startDebug: startDebugImpl } = await import('./debug');
    return startDebugImpl(flow, breakpoints, onEvent, onLog);
  } else {
    // Mock 环境 - 暂不实现
    console.warn('调试功能在 Mock 环境下不可用');
  }
}

/**
 * 发送调试命令
 *
 * @param command - 调试命令
 */
export async function sendDebugCommand(command: import('@/types/debug').DebugCommand): Promise<void> {
  if (isTauriEnv()) {
    const { sendDebugCommand: sendDebugCommandImpl } = await import('./debug');
    return sendDebugCommandImpl(command);
  } else {
    // Mock 环境 - 暂不实现
    console.warn('调试功能在 Mock 环境下不可用');
  }
}

/**
 * 结束调试会话
 */
export async function endDebug(): Promise<void> {
  if (isTauriEnv()) {
    const { endDebug: endDebugImpl } = await import('./debug');
    return endDebugImpl();
  } else {
    // Mock 环境 - 暂不实现
    console.warn('调试功能在 Mock 环境下不可用');
  }
}

/**
 * 获取调试会话状态
 *
 * 检查后端是否有活跃的调试会话
 * @returns true 如果有活跃的调试会话，false 否则
 */
export async function get_debug_status(): Promise<boolean> {
  if (isTauriEnv()) {
    const { get_debug_status: get_debug_statusImpl } = await import('./debug');
    return get_debug_statusImpl();
  } else {
    // Mock 环境 - 返回 false
    return false;
  }
}

/**
 * 强制结束调试会话
 *
 * 强制清理残留的调试会话状态（用于页面刷新后恢复）
 */
export async function force_end_debug(): Promise<void> {
  if (isTauriEnv()) {
    const { force_end_debug: force_end_debugImpl } = await import('./debug');
    return force_end_debugImpl();
  } else {
    // Mock 环境 - 暂不实现
    console.warn('调试功能在 Mock 环境下不可用');
  }
}

// ============================================================================
// MCP 命令
// ============================================================================

export type { McpToolInfo, McpResourceInfo, McpServerInfo, McpDiscoverToolsOutput, McpDiscoverToolsInput } from './mcp';

/**
 * 发现 MCP 服务器的工具和资源
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param input - MCP 发现工具输入
 * @returns MCP 工具和资源列表
 */
export async function discoverMcpTools(
  input: import('./mcp').McpDiscoverToolsInput
): Promise<import('./mcp').McpDiscoverToolsOutput> {
  if (isTauriEnv()) {
    const { discoverMcpTools: discoverMcpToolsImpl } = await import('./mcp');
    return discoverMcpToolsImpl(input);
  } else {
    const { discoverMcpTools: discoverMcpToolsImpl } = await import('./mock');
    return discoverMcpToolsImpl(input);
  }
}

// ============================================================================
// 压力测试命令
// ============================================================================

/**
 * 取消正在执行的流程
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @throws IpcError 如果取消失败
 */
export async function cancelFlow(): Promise<void> {
  if (isTauriEnv()) {
    const { cancelFlow: cancelFlowImpl } = await import('./flow');
    return cancelFlowImpl();
  } else {
    // Mock 环境 - 无操作
  }
}

// ============================================================================
// 压力测试命令
// ============================================================================

/**
 * 启动压力测试
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param flow - 要压测的流程对象
 * @param config - 压测配置
 * @param onEvent - 压测事件回调函数
 */
export async function startStressTest(
  flow: import('@/types/flow').Flow,
  config: import('@/types/stressTest').StressTestConfig,
  onEvent: (event: import('@/types/stressTest').StressTestEvent) => void
): Promise<void> {
  if (isTauriEnv()) {
    const { startStressTest: startStressTestImpl } = await import('./stressTest');
    return startStressTestImpl(flow, config, onEvent);
  } else {
    // Mock 环境 - 暂不实现
    console.warn('压力测试功能在 Mock 环境下不可用');
  }
}

/**
 * 停止压力测试
 */
export async function stopStressTest(): Promise<void> {
  if (isTauriEnv()) {
    const { stopStressTest: stopStressTestImpl } = await import('./stressTest');
    return stopStressTestImpl();
  } else {
    // Mock 环境 - 暂不实现
    console.warn('压力测试功能在 Mock 环境下不可用');
  }
}

/**
 * 获取压力测试状态
 *
 * @returns 当前压测状态和进度信息
 */
export async function getStressTestStatus(): Promise<import('@/types/stressTest').GetStressTestStatusOutput> {
  if (isTauriEnv()) {
    const { getStressTestStatus: getStressTestStatusImpl } = await import('./stressTest');
    return getStressTestStatusImpl();
  } else {
    // Mock 环境 - 返回空闲状态
    return {
      status: 'idle',
      progress: undefined,
    };
  }
}
