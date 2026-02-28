/**
 * Mock 服务 - 用于非 Tauri 环境测试
 * 模拟后端 IPC 命令行为
 */

import type { Flow } from '@/types/flow';
import type { ExecuteFlowCommandOutput } from '@/types/ipc';
import type { FlowExecutionReport } from '@/types/report';

/**
 * 生成 UUID v4
 */
function generateUUID(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

/**
 * Mock 创建流程
 * 模拟后端 create_flow 命令行为
 *
 * @param name - 流程名称
 * @returns 创建的流程对象
 */
export async function createFlow(name: string): Promise<Flow> {
  const flowId = generateUUID();

  return {
    uuid: flowId,
    name,
    nodes: [
      {
        uuid: `start-${flowId}`,
        type: 'entry',
        name: '开始',
        position: { x: 0, y: 0 },
        data: { type: 'entry' },
      },
    ],
    connections: [],
    initial_variables: {},
  };
}

/**
 * Mock 执行流程
 * 模拟后端 execute_flow 命令行为
 *
 * @param flow - 要执行的流程对象
 * @param variables - 可选的执行变量
 * @returns 执行结果
 */
export async function executeFlow(
  flow: Flow,
  _variables?: Record<string, unknown>
): Promise<ExecuteFlowCommandOutput> {
  const startTime = Date.now();

  // 查找开始节点
  const startNode = flow.nodes.find((node) => node.type === 'entry');

  // 验证开始节点存在
  if (!startNode) {
    return {
      success: false,
      error: '流程缺少开始节点',
      execution_time: Date.now() - startTime,
    };
  }

  // 检查开始节点是否有输出连接
  const hasConnection = flow.connections.some(
    (conn) => conn.source_node_uuid === startNode.uuid
  );

  if (!hasConnection) {
    return {
      success: false,
      error: '无连接',
      execution_time: Date.now() - startTime,
    };
  }

  const endTime = Date.now();
  const executionTime = endTime - startTime;

  // 生成模拟的执行报告
  const flowReport: FlowExecutionReport = {
    flow_uuid: flow.uuid,
    flow_name: flow.name,
    start_time: startTime,
    end_time: endTime,
    duration_ms: executionTime,
    total_nodes: flow.nodes.length,
    executed_count: flow.nodes.length,
    status: 'success',
    node_reports: flow.nodes.map((node, index) => ({
      node_uuid: node.uuid,
      node_type: node.type,
      node_name: node.name,
      start_time: startTime + index * 10,
      end_time: startTime + index * 10 + 50,
      duration_ms: 50,
      status: 'success' as const,
      input_data: index === 0 ? null : { from: flow.nodes[index - 1]?.uuid },
      output_data: { result: 'ok' },
    })),
    statistics: {
      success_count: flow.nodes.length,
      failed_count: 0,
      avg_node_duration_ms: 50,
      max_node_duration_ms: 50,
      min_node_duration_ms: 50,
      completion_rate: 1,
      error_rate: 0,
    },
  };

  // 执行成功
  return {
    success: true,
    flow_report: flowReport,
    execution_time: executionTime,
  };
}


/**
 * Mock 事件订阅服务 - 用于非 Tauri 环境测试
 */

import type { LogEvent, LogLevel } from '@/types/events';

// 模拟日志级别
const mockLevels: LogLevel[] = ['INFO', 'DEBUG', 'WARN', 'ERROR'];

/**
 * Mock 日志订阅 Channel
 * 使用 setInterval 模拟日志推送
 *
 * @param onLog - 日志回调函数
 * @returns mock channel 对象和 cleanup 函数
 */
export function subscribeToFlowLogs(
  onLog: (event: LogEvent) => void
): { channel: { mock: true }; cleanup: () => void } {
  // 模拟定时发送日志
  const intervalId = setInterval(() => {
    onLog({
      level: mockLevels[Math.floor(Math.random() * mockLevels.length)],
      message: `[Mock] 模拟日志消息 ${Date.now()}`,
      timestamp: Date.now(),
      nodeUuid: 'mock-node-1',
      nodeName: 'Mock 节点',
      flowUuid: 'mock-flow-1',
    });
  }, 1000);

  return {
    channel: { mock: true },
    cleanup: () => clearInterval(intervalId),
  };
}

/**
 * Mock 执行流程并订阅日志
 *
 * @param _input - 执行输入参数（Mock 中未使用）
 * @param _flow - 流程对象（Mock 中未使用）
 * @param onLog - 日志回调函数
 */
export async function executeFlowWithLogs(
  _input: { flowId: string; variables?: Record<string, unknown> },
  _flow: unknown,
  onLog: (event: LogEvent) => void
): Promise<void> {
  const { cleanup } = subscribeToFlowLogs(onLog);

  // 模拟执行 3 秒后停止
  await new Promise((resolve) => setTimeout(resolve, 3000));

  cleanup();
}