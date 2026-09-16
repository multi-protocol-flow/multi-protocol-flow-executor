/**
 * 流程 UUID 全量重映射工具
 * 为流程本身、所有节点、所有连线生成全新 UUID，并级联重映射节点内部跨节点引用
 */
import type { Flow } from '@/types';

export function remapFlowUuids(flow: Flow, options?: { newName?: string }): Flow {
  const idMap = new Map<string, string>();

  // 1. 为每个节点分配新 UUID，构建映射表
  const newNodes = (flow.nodes || []).map((node) => {
    const newUuid = crypto.randomUUID();
    idMap.set(node.uuid, newUuid);
    return {
      ...node,
      uuid: newUuid,
      data: node.data ? JSON.parse(JSON.stringify(node.data)) : {},
    };
  });

  // 2. 为每条连线分配新 UUID，并将 source_node_uuid 和 target_node_uuid 映射为新节点 UUID
  const newConnections = (flow.connections || []).map((conn) => {
    return {
      ...conn,
      id: crypto.randomUUID(),
      source_node_uuid: idMap.get(conn.source_node_uuid) ?? conn.source_node_uuid,
      target_node_uuid: idMap.get(conn.target_node_uuid) ?? conn.target_node_uuid,
    };
  });

  // 3. 级联重映射节点配置内部的跨节点引用（如 connection_id, connection_node_id, source_node_id, connection_uuid）
  const refKeys = ['connection_id', 'connection_node_id', 'source_node_id', 'connection_uuid'] as const;
  for (const node of newNodes) {
    if (node.data && typeof node.data === 'object') {
      const dataObj = node.data as Record<string, unknown>;
      for (const key of refKeys) {
        const val = dataObj[key];
        if (typeof val === 'string' && idMap.has(val)) {
          dataObj[key] = idMap.get(val);
        }
      }
    }
  }

  return {
    ...flow,
    uuid: crypto.randomUUID(),
    name: options?.newName ?? flow.name,
    nodes: newNodes,
    connections: newConnections,
  };
}
