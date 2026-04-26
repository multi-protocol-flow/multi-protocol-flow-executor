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
    case 'graphql_unsubscribe':
      return { type: 'graphql_unsubscribe', connection_id: '', subscription_id: '' };
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
    case 'variable_extractor':
      return { type: 'variable_extractor', output_mappings: [], source_node_id: undefined };
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
  mcp_connect: 'MCP 连接',
  mcp_call_tool: 'MCP 调用工具',
  mcp_read_resource: 'MCP 读取资源',
  mcp_get_prompt: 'MCP 获取提示词',
  mcp_close: 'MCP 关闭',
  tcp_connect: 'TCP 连接',
  tcp_send: 'TCP 发送',
  tcp_close: 'TCP 关闭',
  udp_bind: 'UDP 绑定',
  udp_send_to: 'UDP 发送',
  udp_recv_from: 'UDP 接收',
  udp_close: 'UDP 关闭',
  mqtt_connect: 'MQTT 连接',
  mqtt_publish: 'MQTT 发布',
  mqtt_subscribe: 'MQTT 订阅',
  mqtt_unsubscribe: 'MQTT 取消订阅',
  mqtt_disconnect: 'MQTT 断开',
  mqtt_wait_message: 'MQTT 等待消息',
  mqtt_request: 'MQTT 请求',
  kafka_connect: 'Kafka 连接',
  kafka_produce: 'Kafka 生产',
  kafka_consume: 'Kafka 消费',
  kafka_consume_batch: 'Kafka 批量消费',
  kafka_admin: 'Kafka 管理',
  kafka_schema_registry: 'Kafka Schema',
  kafka_close: 'Kafka 关闭',
  sse_connect: 'SSE 连接',
  sse_listen: 'SSE 监听',
  sse_disconnect: 'SSE 断开',
  graphql_connect: 'GraphQL 连接',
  graphql_query: 'GraphQL 查询',
  graphql_subscribe: 'GraphQL 订阅',
  graphql_unsubscribe: 'GraphQL 取消订阅',
  graphql_introspect: 'GraphQL 内省',
  graphql_disconnect: 'GraphQL 断开',
  amqp_connect: 'AMQP 连接',
  amqp_close: 'AMQP 关闭',
  amqp_exchange_declare: 'AMQP 声明交换器',
  amqp_exchange_delete: 'AMQP 删除交换器',
  amqp_queue_declare: 'AMQP 声明队列',
  amqp_queue_delete: 'AMQP 删除队列',
  amqp_queue_bind: 'AMQP 绑定队列',
  amqp_publish: 'AMQP 发布',
  amqp_consume: 'AMQP 消费',
  amqp_get: 'AMQP 获取',
  amqp_ack: 'AMQP 确认',
  amqp_management: 'AMQP 管理',
  amqp10_connect: 'AMQP1.0 连接',
  amqp10_send: 'AMQP1.0 发送',
  amqp10_receive: 'AMQP1.0 接收',
  amqp10_accept: 'AMQP1.0 确认',
  amqp10_reject: 'AMQP1.0 拒绝',
  amqp10_disconnect: 'AMQP1.0 断开',
  variable_extractor: '变量提取',
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
