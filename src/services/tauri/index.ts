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
    const { createFlow, executeFlow, IpcError } = await import('./flow');
    return { createFlow, executeFlow, IpcError };
  } else {
    // 在非 Tauri 环境中使用 Mock
    const { createFlow, executeFlow } = await import('./mock');
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
    return { createFlow, executeFlow, IpcError };
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
 * 执行流程
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param flow - 要执行的流程对象
 * @param variables - 可选的执行变量
 * @returns 执行结果
 */
export async function executeFlow(
  flow: Flow,
  variables?: Record<string, unknown>
): Promise<ExecuteFlowCommandOutput> {
  const { executeFlow: executeFlowImpl } = await getCachedService();
  return executeFlowImpl(flow, variables);
}

/**
 * 执行流程并订阅日志（Tauri 环境）
 * 使用 Channel API 接收后端日志事件
 *
 * @param flow - 要执行的流程对象
 * @param onLog - 日志回调函数
 * @param variables - 可选的执行变量
 * @returns 执行结果
 */
export async function executeFlowWithChannel(
  flow: Flow,
  onLog: (event: import('@/types/events').LogEvent) => void,
  variables?: Record<string, unknown>
): Promise<ExecuteFlowCommandOutput> {
  // 检测是否在 Tauri 环境中
  if (isTauriEnv()) {
    const { executeFlowWithChannel: executeFlowWithChannelImpl } = await import('./flow');
    return executeFlowWithChannelImpl(flow, onLog, variables);
  } else {
    // Mock 环境
    const { executeFlowWithChannel: executeFlowWithChannelImpl } = await import('./mock');
    return executeFlowWithChannelImpl(flow, onLog, variables);
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
