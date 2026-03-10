/**
 * Flow 数据与 React Flow 数据转换工具
 */
import type { Node, Edge } from '@xyflow/react';
import type { Flow, Connection, NodeData } from '@/types';

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
