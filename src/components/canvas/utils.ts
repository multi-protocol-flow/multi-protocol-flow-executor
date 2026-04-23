/**
 * Flow 数据与 React Flow 数据转换工具
 */
import type { Node, Edge } from '@xyflow/react';
import type { Flow, Connection, NodeData, NodeType } from '@/types';
import { getCachedDefaultConfig, getCachedDisplayName } from '@/lib/NodeRegistry';

/** React Flow 节点数据类型 */
export type FlowNodeData = Record<string, unknown> & {
  label: string;
  config?: NodeData;
};

/**
 * 将 Flow 数据转换为 React Flow 格式
 */
export function convertFlowToReactFlow(flow: Flow): {
  nodes: Node[];
  edges: Edge[];
} {
  const nodes: Node[] = flow.nodes.map((node) => ({
    id: node.uuid,
    type: node.type,
    position: { x: node.position.x, y: node.position.y },
    data: {
      label: node.name,
      config: node.data,
    } as FlowNodeData,
  }));

  const edges: Edge[] = flow.connections.map((conn, index) => ({
    id: conn.id || `edge-${index}`,
    source: conn.source_node_uuid,
    target: conn.target_node_uuid,
    sourceHandle: conn.source_port_id,
    targetHandle: conn.target_port_id,
    type: 'custom',
    data: {
      sourceHandle: conn.source_port_id,
    },
  }));

  return { nodes, edges };
}

/**
 * 将 React Flow 边转换为 Connection 格式
 */
export function convertEdgeToConnection(
  edge: Edge
): Connection {
  return {
    id: edge.id,
    source_node_uuid: edge.source,
    target_node_uuid: edge.target,
    source_port_id: edge.sourceHandle || 'out',
    target_port_id: edge.targetHandle || 'in',
  };
}

/**
 * 生成唯一 ID
 */
export function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
}

/**
 * 创建新连接
 */
export function createConnection(
  sourceId: string,
  targetId: string,
  sourcePort: string,
  targetPort: string
): Connection {
  return {
    id: generateId(),
    source_node_uuid: sourceId,
    target_node_uuid: targetId,
    source_port_id: sourcePort,
    target_port_id: targetPort,
  };
}

/**
 * 验证连接是否有效
 * - 不能连接到自己
 */
export function isValidConnection(
  sourceId: string,
  targetId: string,
  _sourcePort: string,
  _targetPort: string
): boolean {
  // 不能连接到自己
  if (sourceId === targetId) {
    return false;
  }
  
  return true;
}

/**
 * 创建节点类型的默认配置数据
 * 优先从后端缓存获取，兜底使用静态默认配置
 */
export function createDefaultNodeData(type: NodeType): NodeData {
  // 优先从后端缓存获取默认配置
  const cachedConfig = getCachedDefaultConfig(type);
  if (cachedConfig) {
    return { ...cachedConfig, type } as NodeData;
  }
  // 兜底：静态默认配置
  switch (type) {
    case 'http':
      return { type: 'http', url: '', method: 'get', headers: {}, body: '' };
    case 'conditional':
      return {
        type: 'conditional',
        condition_type: 'simple',
        field_path: '',
        operator: 'equal',
        expected_value: '',
      };
    case 'ws_connect':
      return { type: 'ws_connect', url: '' };
    case 'ws_send_collect':
      return { type: 'ws_send_collect', connection_id: '', message: '', collect_response: false };
    case 'ws_close':
      return { type: 'ws_close', connection_id: '' };
    case 'assertion':
      return {
        type: 'assertion',
        assertions: [],
        mode: 'all',
        on_failure: 'route_false',
        continue_on_failure: false,
      };
    case 'db_connect':
      return { type: 'db_connect' };
    case 'db_query':
      return { type: 'db_query', connection_uuid: '', sql: '', params: [], result_to: '' };
    case 'db_begin':
      return { type: 'db_begin', connection_uuid: '' };
    case 'db_commit':
      return { type: 'db_commit', connection_uuid: '' };
    case 'db_rollback':
      return { type: 'db_rollback', connection_uuid: '' };
    case 'db_close':
      return { type: 'db_close', connection_uuid: '' };
    case 'script':
      return { type: 'script', script: '' };
    case 'grpc_connect':
      return { type: 'grpc_connect', url: '' };
    case 'grpc_call':
      return { type: 'grpc_call', connection_id: '', service_name: '', method_name: '', request_json: '' };
    case 'grpc_close':
      return { type: 'grpc_close', connection_id: '' };
    case 'entry':
      return { type: 'entry' };
  }
}

/**
 * 节点类型对应的显示名称
 * 优先从后端缓存获取，兜底使用静态映射
 */
const STATIC_NODE_TYPE_LABELS: Record<NodeType, string> = {
  http: 'HTTP 请求',
  conditional: '条件判断',
  ws_connect: 'WS 连接',
  ws_send_collect: 'WS 发送采集',
  ws_close: 'WS 关闭',
  assertion: '断言',
  db_connect: 'DB 连接',
  db_query: 'DB 查询',
  db_begin: 'DB 开始事务',
  db_commit: 'DB 提交',
  db_rollback: 'DB 回滚',
  db_close: 'DB 关闭',
  script: '脚本',
  entry: '入口',
  grpc_connect: 'gRPC 连接',
  grpc_call: 'gRPC 调用',
  grpc_close: 'gRPC 关闭',
};

/**
 * 获取节点类型的显示名称（动态，优先后端缓存）
 */
export function getNodeTypeName(type: NodeType): string {
  const cached = getCachedDisplayName(type);
  return cached ?? STATIC_NODE_TYPE_LABELS[type] ?? type;
}

/**
 * 创建默认节点显示名称
 */
export function createDefaultNodeName(type: NodeType, count: number): string {
  return `${getNodeTypeName(type)} ${count}`;
}
