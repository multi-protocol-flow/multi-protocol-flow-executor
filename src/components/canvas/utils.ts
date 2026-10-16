/**
 * Flow 数据与 React Flow 数据转换工具
 */
import type { Node, Edge } from '@xyflow/react';
import type { Flow, Connection, NodeData, NodeType, AnyNodeType } from '@/types';
import { getCachedDefaultConfig } from '@/lib/NodeRegistry';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { getStageGraph, type FlowStage } from '@/stores/flow';

/** React Flow 节点数据类型 */
export type FlowNodeData = Record<string, unknown> & {
  label: string;
  config?: NodeData;
  disabled?: boolean;
};

/**
 * 将 Flow 数据转换为 React Flow 格式
 *
 * `stage` 指定渲染哪个生命周期阶段：`main` 渲染流程主体的
 * `flow.nodes` / `flow.connections`，`setup` / `teardown` 渲染对应子图。
 */
export function convertFlowToReactFlow(flow: Flow, stage: FlowStage = 'main'): {
  nodes: Node[];
  edges: Edge[];
} {
  const graph = getStageGraph(flow, stage);
  const nodes: Node[] = graph.nodes.map((node) => {
    const isBypassed = Boolean(node.disabled || node.on_error === 'bypass');
    return {
      id: node.uuid,
      type: node.type,
      position: { x: node.position.x, y: node.position.y },
      className: isBypassed ? 'node-bypassed' : undefined,
      data: {
        label: node.name,
        config: node.data,
        disabled: isBypassed,
      } as FlowNodeData,
    };
  });

  const edges: Edge[] = graph.connections.map((conn, index) => ({
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
/**
 * 获取连线不合规的原因代码
 */
export function getConnectionErrorReason(
  sourceId: string,
  targetId: string,
  sourcePort: string,
  _targetPort: string,
  connections?: Connection[],
): 'self' | 'port_used' | 'cycle' | null {
  if (sourceId === targetId) return 'self';
  if (!connections) return null;
  const portAlreadyUsed = connections.some(
    (c) => c.source_node_uuid === sourceId && c.source_port_id === sourcePort
  );
  if (portAlreadyUsed) return 'port_used';
  if (hasPath(targetId, sourceId, connections)) return 'cycle';
  return null;
}

export function isValidConnection(
  sourceId: string,
  targetId: string,
  sourcePort: string,
  targetPort: string,
  connections?: Connection[],
): boolean {
  return getConnectionErrorReason(sourceId, targetId, sourcePort, targetPort, connections) === null;
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
    case 'http:request':
      return { type: 'http:request', url: '', method: 'get', headers: {}, body: '' };
    case 'conditional':
      return {
        type: 'conditional',
        condition_type: 'simple',
        field_path: '',
        operator: 'equal',
        expected_value: '',
      };
    case 'ws:connect':
      return { type: 'ws:connect', url: '' };
    case 'ws:send_collect':
      return { type: 'ws:send_collect', connection_id: '', message: '', collect_response: false };
    case 'ws:close':
      return { type: 'ws:close', connection_id: '' };
    case 'assertion':
      return {
        type: 'assertion',
        assertions: [],
        mode: 'all',
        on_failure: 'route_false',
        continue_on_failure: false,
      };
    case 'script':
      return { type: 'script', script: '' };
    case 'tcp:connect':
      return { type: 'tcp:connect', host: '', port: 0 };
    case 'tcp:send':
      return { type: 'tcp:send', connection_id: '', data: '' };
    case 'tcp:close':
      return { type: 'tcp:close', connection_id: '' };
    case 'udp:bind':
      return { type: 'udp:bind', local_port: 0 };
    case 'udp:send_to':
      return { type: 'udp:send_to', connection_id: '', data: '', target_addr: '', target_port: 0 };
    case 'udp:recv_from':
      return { type: 'udp:recv_from', connection_id: '' };
    case 'udp:close':
      return { type: 'udp:close', connection_id: '' };
    case 'sse:connect':
      return { type: 'sse:connect', url: '' };
    case 'sse:listen':
      return { type: 'sse:listen', connection_id: '' };
    case 'sse:disconnect':
      return { type: 'sse:disconnect', connection_id: '' };
    case 'graphql:connect':
      return { type: 'graphql:connect', endpoint: '' };
    case 'graphql:query':
      return { type: 'graphql:query', connection_id: '', query: '' };
    case 'graphql:subscribe':
      return { type: 'graphql:subscribe', connection_id: '', subscription: '' };
    case 'graphql:introspect':
      return { type: 'graphql:introspect', connection_id: '' };
    case 'graphql:disconnect':
      return { type: 'graphql:disconnect', connection_id: '' };

    case 'variable:extractor':
      return { type: 'variable:extractor', output_mappings: [], source_node_id: undefined };
    case 'proxy':
      return { type: 'proxy', mode: 'clear' };
    case 'flow:sub':
      return {
        type: 'flow:sub',
        inherit_all_variables: true,
        export_all_variables: true,
      };
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

/**
 * 检测指定坐标是否落在某条连线段附近，返回可被拆分插入的连接
 */
export function findEdgeToInsert(
  pos: { x: number; y: number },
  nodes: Array<{ uuid: string; position: { x: number; y: number } }>,
  connections: Connection[],
  threshold = 45,
): Connection | null {
  if (!connections || connections.length === 0) return null;
  const nodeMap = new Map(nodes.map((n) => [n.uuid, n]));

  for (const conn of connections) {
    const src = nodeMap.get(conn.source_node_uuid);
    const tgt = nodeMap.get(conn.target_node_uuid);
    if (!src || !tgt) continue;

    const x1 = src.position.x + 180;
    const y1 = src.position.y + 35;
    const x2 = tgt.position.x;
    const y2 = tgt.position.y + 35;

    const minX = Math.min(x1, x2) - 30;
    const maxX = Math.max(x1, x2) + 30;
    const minY = Math.min(y1, y2) - 30;
    const maxY = Math.max(y1, y2) + 30;
    if (pos.x < minX || pos.x > maxX || pos.y < minY || pos.y > maxY) {
      continue;
    }

    const l2 = (x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1);
    if (l2 < 100) continue;
    const t = Math.max(0, Math.min(1, ((pos.x - x1) * (x2 - x1) + (pos.y - y1) * (y2 - y1)) / l2));
    if (t < 0.15 || t > 0.85) continue;

    const projX = x1 + t * (x2 - x1);
    const projY = y1 + t * (y2 - y1);
    const dist = Math.hypot(pos.x - projX, pos.y - projY);

    if (dist <= threshold) {
      return conn;
    }
  }

  return null;
}

/**
 * 将节点插入到既有连接线中间（打断原边并建立两条新边）
 */
export function insertNodeIntoConnection(
  newNodeUuid: string,
  edgeToSplit: Connection,
  addConnection: (conn: Connection) => void,
  removeConnection: (id: string) => void,
) {
  if (edgeToSplit.id) {
    removeConnection(edgeToSplit.id);
  }
  addConnection(
    createConnection(edgeToSplit.source_node_uuid, newNodeUuid, edgeToSplit.source_port_id, 'in')
  );
  addConnection(
    createConnection(newNodeUuid, edgeToSplit.target_node_uuid, 'out', edgeToSplit.target_port_id)
  );
}
