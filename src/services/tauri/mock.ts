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
    uuid: `report-${Date.now()}`,
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

import type { LogEvent } from '@/types/events';

/**
 * Mock 执行流程并订阅日志
 * 模拟执行过程中发送日志事件
 *
 * @param flow - 要执行的流程对象
 * @param onLog - 日志回调函数
 * @param _variables - 可选的执行变量（Mock 中未使用）
 * @returns 执行结果
 */
export async function executeFlowWithChannel(
  flow: Flow,
  onLog: (event: LogEvent) => void,
  _variables?: Record<string, unknown>
): Promise<ExecuteFlowCommandOutput> {
  const startTime = Date.now();

  // 发送流程开始日志
  onLog({
    level: 'INFO',
    message: `流程 '${flow.name}' 开始执行`,
    timestamp: startTime,
    flowUuid: flow.uuid,
  });

  // 查找开始节点
  const startNode = flow.nodes.find((node) => node.type === 'entry');

  // 验证开始节点存在
  if (!startNode) {
    onLog({
      level: 'ERROR',
      message: '流程缺少开始节点',
      timestamp: Date.now(),
      flowUuid: flow.uuid,
    });
    return {
      success: false,
      error: '流程缺少开始节点',
      execution_time: Date.now() - startTime,
    };
  }

  // 发送节点开始日志
  onLog({
    level: 'INFO',
    message: `执行节点 ${startNode.name} (entry)`,
    timestamp: Date.now(),
    flowUuid: flow.uuid,
    nodeUuid: startNode.uuid,
    nodeName: startNode.name,
  });

  // 模拟节点执行延迟
  await new Promise((resolve) => setTimeout(resolve, 100));

  // 发送节点完成日志
  onLog({
    level: 'INFO',
    message: `节点 ${startNode.name} 执行完成`,
    timestamp: Date.now(),
    flowUuid: flow.uuid,
    nodeUuid: startNode.uuid,
    nodeName: startNode.name,
  });

  // 检查开始节点是否有输出连接
  const hasConnection = flow.connections.some(
    (conn) => conn.source_node_uuid === startNode.uuid
  );

  if (!hasConnection) {
    onLog({
      level: 'INFO',
      message: `节点 ${startNode.name} 没有下一个节点，执行完成`,
      timestamp: Date.now(),
      flowUuid: flow.uuid,
      nodeUuid: startNode.uuid,
      nodeName: startNode.name,
    });

    const endTime = Date.now();
    onLog({
      level: 'INFO',
      message: '流程执行成功',
      timestamp: endTime,
      flowUuid: flow.uuid,
    });

    // 生成模拟的执行报告（单个 Entry 节点无连接是正常终止）
    const flowReport: FlowExecutionReport = {
      uuid: `report-${Date.now()}`,
      flow_uuid: flow.uuid,
      flow_name: flow.name,
      start_time: startTime,
      end_time: endTime,
      duration_ms: endTime - startTime,
      total_nodes: flow.nodes.length,
      executed_count: 1,
      status: 'success',
      node_reports: [{
        node_uuid: startNode.uuid,
        node_type: startNode.type,
        node_name: startNode.name,
        start_time: startTime,
        end_time: endTime,
        duration_ms: endTime - startTime,
        status: 'success',
        input_data: null,
        output_data: null,
      }],
      statistics: {
        success_count: 1,
        failed_count: 0,
        avg_node_duration_ms: endTime - startTime,
        max_node_duration_ms: endTime - startTime,
        min_node_duration_ms: endTime - startTime,
        completion_rate: 1,
        error_rate: 0,
      },
    };

    return {
      success: true,
      flow_report: flowReport,
      execution_time: endTime - startTime,
    };
  }

  // 模拟执行后续节点
  let executedCount = 1; // 已执行开始节点
  const nodeReports: FlowExecutionReport['node_reports'] = [
    {
      node_uuid: startNode.uuid,
      node_type: startNode.type,
      node_name: startNode.name,
      start_time: startTime,
      end_time: startTime + 100,
      duration_ms: 100,
      status: 'success',
      input_data: null,
      output_data: { result: 'ok' },
    },
  ];

  // 遍历并模拟执行其他节点
  for (let i = 0; i < flow.nodes.length; i++) {
    const node = flow.nodes[i];
    if (node.type === 'entry') continue; // 跳过已执行的开始节点

    const nodeStartTime = Date.now();

    // 发送节点开始日志
    onLog({
      level: 'INFO',
      message: `执行节点 ${node.name} (${node.type})`,
      timestamp: nodeStartTime,
      flowUuid: flow.uuid,
      nodeUuid: node.uuid,
      nodeName: node.name,
    });

    // 模拟节点执行延迟
    await new Promise((resolve) => setTimeout(resolve, 50 + Math.random() * 100));

    const nodeEndTime = Date.now();

    // 发送节点完成日志
    onLog({
      level: 'INFO',
      message: `节点 ${node.name} 执行完成`,
      timestamp: nodeEndTime,
      flowUuid: flow.uuid,
      nodeUuid: node.uuid,
      nodeName: node.name,
    });

    executedCount++;
    nodeReports.push({
      node_uuid: node.uuid,
      node_type: node.type,
      node_name: node.name,
      start_time: nodeStartTime,
      end_time: nodeEndTime,
      duration_ms: nodeEndTime - nodeStartTime,
      status: 'success',
      input_data: { from: flow.nodes[i - 1]?.uuid },
      output_data: { result: 'ok' },
    });
  }

  const endTime = Date.now();

  // 发送流程完成日志
  onLog({
    level: 'INFO',
    message: '流程执行成功',
    timestamp: endTime,
    flowUuid: flow.uuid,
  });

  // 生成模拟的执行报告
  const flowReport: FlowExecutionReport = {
    uuid: `report-${Date.now()}`,
    flow_uuid: flow.uuid,
    flow_name: flow.name,
    start_time: startTime,
    end_time: endTime,
    duration_ms: endTime - startTime,
    total_nodes: flow.nodes.length,
    executed_count: executedCount,
    status: 'success',
    node_reports: nodeReports,
    statistics: {
      success_count: executedCount,
      failed_count: 0,
      avg_node_duration_ms: Math.round((endTime - startTime) / executedCount),
      max_node_duration_ms: Math.max(...nodeReports.map((r) => r.duration_ms)),
      min_node_duration_ms: Math.min(...nodeReports.map((r) => r.duration_ms)),
      completion_rate: 1,
      error_rate: 0,
    },
  };

  return {
    success: true,
    flow_report: flowReport,
    execution_time: endTime - startTime,
  };
}