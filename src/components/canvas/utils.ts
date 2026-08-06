/**
 * Flow 数据与 React Flow 数据转换工具
 */
import type { Node, Edge } from '@xyflow/react';
import type { Flow, Connection, NodeData, NodeType, AnyNodeType } from '@/types';
import { getCachedDefaultConfig } from '@/lib/NodeRegistry';
import { getNodeDisplayName } from '@/lib/nodeMeta';

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
  // 旧版自动生成的节点名格式为 "中文类型名 数字"，匹配则清除以启用 i18n 动态名称
  const autoNameRe = /\s+\d+$/;
  const nodes: Node[] = flow.nodes.map((node) => ({
    id: node.uuid,
    type: node.type,
    position: { x: node.position.x, y: node.position.y },
    data: {
      label: autoNameRe.test(node.name) ? '' : node.name,
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
 * - 源节点的输出端口不能已有出边（每个输出端口只能连一个下游节点）
 * - 不能形成循环依赖（检测从 target 到 source 的路径）
 *
 * @param sourceId - 源节点 UUID
 * @param targetId - 目标节点 UUID
 * @param sourcePort - 源端口 ID
 * @param _targetPort - 目标端口 ID
 * @param connections - 当前流程所有连接（可选，传入后会执行完整校验）
 * @returns 连接是否有效
 */
export function isValidConnection(
  sourceId: string,
  targetId: string,
  sourcePort: string,
  _targetPort: string,
  connections?: Connection[],
): boolean {
  // 规则 1: 不能连接到自己
  if (sourceId === targetId) {
    return false;
  }

  // 以下检查需要 connections 数据
  if (!connections) {
    return true;
  }

  // 规则 2: 每个输出端口只能有一条出边
  const portAlreadyUsed = connections.some(
    (c) => c.source_node_uuid === sourceId && c.source_port_id === sourcePort
  );
  if (portAlreadyUsed) {
    return false;
  }

  // 规则 3: 不能形成循环依赖
  // 检查从 target 到 source 是否存在路径（如果添加了 source→target，会不会有循环）
  if (hasPath(targetId, sourceId, connections)) {
    return false;
  }

  return true;
}

/**
 * 检查在有向图中是否存在从 from 到 to 的路径（BFS）
 */
function hasPath(
  from: string,
  to: string,
  connections: Connection[]
): boolean {
  // 构建邻接表
  const adj = new Map<string, string[]>();
  for (const conn of connections) {
    const neighbors = adj.get(conn.source_node_uuid) || [];
    neighbors.push(conn.target_node_uuid);
    adj.set(conn.source_node_uuid, neighbors);
  }

  // BFS
  const visited = new Set<string>();
  const queue = [from];
  while (queue.length > 0) {
    const current = queue.shift()!;
    if (current === to) return true;
    if (visited.has(current)) continue;
    visited.add(current);
    const neighbors = adj.get(current);
    if (neighbors) {
      for (const n of neighbors) {
        if (!visited.has(n)) {
          queue.push(n);
        }
      }
    }
  }
  return false;
}

/**
 * 创建节点类型的默认配置数据
 * 优先从后端缓存获取，兜底使用静态默认配置
 * D9: 签名宽化为 AnyNodeType —— 接受任意插件 type_id（如 "vendor:my-node"）
 */
export function createDefaultNodeData(type: AnyNodeType): NodeData {
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
    case 'mcp_connect':
      return { type: 'mcp_connect', url: '' };
    case 'mcp_call_tool':
      return { type: 'mcp_call_tool', connection_id: '', tool_name: '' };
    case 'mcp_read_resource':
      return { type: 'mcp_read_resource', connection_id: '', resource_uri: '' };
    case 'mcp_get_prompt':
      return { type: 'mcp_get_prompt', connection_id: '', prompt_name: '', arguments: {} };
    case 'mcp_close':
      return { type: 'mcp_close', connection_id: '' };
    case 'tcp_connect':
      return { type: 'tcp_connect', host: '', port: 0 };
    case 'tcp_send':
      return { type: 'tcp_send', connection_id: '', data: '' };
    case 'tcp_close':
      return { type: 'tcp_close', connection_id: '' };
    case 'udp_bind':
      return { type: 'udp_bind', local_port: 0 };
    case 'udp_send_to':
      return { type: 'udp_send_to', connection_id: '', data: '', target_addr: '', target_port: 0 };
    case 'udp_recv_from':
      return { type: 'udp_recv_from', connection_id: '' };
    case 'udp_close':
      return { type: 'udp_close', connection_id: '' };
    case 'mqtt_connect':
      return { type: 'mqtt_connect', url: 'mqtt://localhost:1883' };
    case 'mqtt_publish':
      return { type: 'mqtt_publish', connection_id: '', topic: '' };
    case 'mqtt_subscribe':
      return { type: 'mqtt_subscribe', connection_id: '', topic: '' };
    case 'mqtt_unsubscribe':
      return { type: 'mqtt_unsubscribe', connection_id: '', topic: '' };
    case 'mqtt_disconnect':
      return { type: 'mqtt_disconnect', connection_id: '' };
    case 'mqtt_wait_message':
      return { type: 'mqtt_wait_message', connection_id: '', topic: '' };
    case 'mqtt_request':
      return { type: 'mqtt_request', connection_id: '', request_topic: '', response_topic: '' };
    case 'kafka_connect':
      return { type: 'kafka_connect', bootstrap_servers: 'localhost:9092' };
    case 'kafka_produce':
      return { type: 'kafka_produce', connection_id: '', topic: '' };
    case 'kafka_consume':
      return { type: 'kafka_consume', connection_id: '', topic: '' };
    case 'kafka_consume_batch':
      return { type: 'kafka_consume_batch', connection_id: '', topic: '' };
    case 'kafka_admin':
      return { type: 'kafka_admin', connection_id: '', operation: 'topic_list' };
    case 'kafka_schema_registry':
      return { type: 'kafka_schema_registry', connection_id: '', operation: 'list_subjects' };
    case 'kafka_close':
      return { type: 'kafka_close', connection_id: '' };
    case 'sse_connect':
      return { type: 'sse_connect', url: '' };
    case 'sse_listen':
      return { type: 'sse_listen', connection_id: '' };
    case 'sse_disconnect':
      return { type: 'sse_disconnect', connection_id: '' };
    case 'graphql_connect':
      return { type: 'graphql_connect', endpoint: '' };
    case 'graphql_query':
      return { type: 'graphql_query', connection_id: '', query: '' };
    case 'graphql_subscribe':
      return { type: 'graphql_subscribe', connection_id: '', subscription: '' };
    case 'graphql_introspect':
      return { type: 'graphql_introspect', connection_id: '' };
    case 'graphql_disconnect':
      return { type: 'graphql_disconnect', connection_id: '' };
    case 'amqp_connect':
      return { type: 'amqp_connect', url: '' };
    case 'amqp_close':
      return { type: 'amqp_close', connection_id: '' };
    case 'amqp_exchange_declare':
      return { type: 'amqp_exchange_declare', connection_id: '', exchange: '', exchange_type: 'direct' };
    case 'amqp_exchange_delete':
      return { type: 'amqp_exchange_delete', connection_id: '', exchange: '' };
    case 'amqp_queue_declare':
      return { type: 'amqp_queue_declare', connection_id: '', queue: '' };
    case 'amqp_queue_delete':
      return { type: 'amqp_queue_delete', connection_id: '', queue: '' };
    case 'amqp_queue_bind':
      return { type: 'amqp_queue_bind', connection_id: '', queue: '', exchange: '' };
    case 'amqp_publish':
      return { type: 'amqp_publish', connection_id: '', routing_key: '' };
    case 'amqp_consume':
      return { type: 'amqp_consume', connection_id: '', queue: '' };
    case 'amqp_get':
      return { type: 'amqp_get', connection_id: '', queue: '' };
    case 'amqp_ack':
      return { type: 'amqp_ack', connection_id: '', action: 'ack', delivery_tag: 0 };
    case 'amqp_management':
      return { type: 'amqp_management', management_url: '', action: 'overview', connection_id: '' };
    case 'amqp10_connect':
      return { type: 'amqp10_connect', url: '' };
    case 'amqp10_send':
      return { type: 'amqp10_send', connection_id: '', address: '' };
    case 'amqp10_receive':
      return { type: 'amqp10_receive', connection_id: '', address: '' };
    case 'amqp10_accept':
      return { type: 'amqp10_accept', connection_id: '', delivery_id: 0 };
    case 'amqp10_reject':
      return { type: 'amqp10_reject', connection_id: '', delivery_id: 0 };
    case 'amqp10_disconnect':
      return { type: 'amqp10_disconnect', connection_id: '' };
    case 'smtp_connect':
      return { type: 'smtp_connect', host: '', port: 25, tls_mode: 'none' };
    case 'smtp_send':
      return { type: 'smtp_send', connection_id: '', from: '', to: [], subject: '', body: '', body_type: 'text' };
    case 'smtp_disconnect':
      return { type: 'smtp_disconnect', connection_id: '' };
    case 'imap_connect':
      return { type: 'imap_connect', host: '', port: 143, tls_mode: 'none' };
    case 'imap_select':
      return { type: 'imap_select', connection_id: '', mailbox: 'INBOX' };
    case 'imap_search':
      return { type: 'imap_search', connection_id: '', criteria: 'ALL' };
    case 'imap_fetch':
      return { type: 'imap_fetch', connection_id: '', uid_set: '', items: '(ENVELOPE)' };
    case 'imap_store_flags':
      return { type: 'imap_store_flags', connection_id: '', uid_set: '', operation: 'add', flags: [] };
    case 'imap_copy':
      return { type: 'imap_copy', connection_id: '', uid_set: '', destination_mailbox: '' };
    case 'imap_move':
      return { type: 'imap_move', connection_id: '', uid_set: '', destination_mailbox: '' };
    case 'imap_expunge':
      return { type: 'imap_expunge', connection_id: '' };
    case 'imap_list':
      return { type: 'imap_list', connection_id: '' };
    case 'imap_folder_create':
      return { type: 'imap_folder_create', connection_id: '', folder_name: '' };
    case 'imap_folder_delete':
      return { type: 'imap_folder_delete', connection_id: '', folder_name: '' };
    case 'imap_idle':
      return { type: 'imap_idle', connection_id: '', mailbox: 'INBOX' };
    case 'imap_close':
      return { type: 'imap_close', connection_id: '' };
    case 'variable_extractor':
      return { type: 'variable_extractor', output_mappings: [], source_node_id: undefined };
    case 'proxy':
      return { type: 'proxy', mode: 'clear' };
    case 'entry':
      return { type: 'entry' };
    default:
      // 未注册/插件节点类型：保留自身 type，避免被静默改写为 entry
      return { type, ...(getCachedDefaultConfig(type) ?? {}) } as NodeData;
  }
}

/**
 * 获取节点类型的显示名称（跟随当前语言自动切换）
 */
export function getNodeTypeName(type: NodeType): string {
  return getNodeDisplayName(type);
}

/**
 * 创建默认节点显示名称
 */
export function createDefaultNodeName(_type: NodeType, count: number): string {
  return `${getNodeTypeName(_type)} ${count}`;
}
