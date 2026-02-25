/**
 * Tauri IPC 服务主导出文件
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 */

import type { Flow } from '@/types/flow';
import type { ExecuteFlowCommandOutput } from '@/types/ipc';

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

// 重新导出 IPC 错误类
export { IpcError } from './flow';

// 导出类型
export type { Flow, ExecuteFlowCommandOutput };
