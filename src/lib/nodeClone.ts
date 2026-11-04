/**
 * 节点复制工具
 *
 * `flow:loop` 的 `body` 是内联子图，body 内的节点与连线各自持有 UUID。
 * 复制一个 loop 节点时只给根节点换 UUID 是不够的：副本会与原节点共享同一批
 * body UUID，而 `locateNode()` 深度优先只取第一个匹配，于是对副本 body 节点的
 * 改名/删除/断点会静默改到原节点上，并随保存落盘。
 *
 * 因此复制/粘贴必须走这里：先深拷贝配置，再递归重生成所有嵌套子图的 UUID。
 */
import type { Connection, FlowSubGraph, Node, NodeData } from '@/types';

/** 递归重生成内联子图的节点与连线 UUID（连线端点同步改写） */
function regenerateSubGraph(graph: FlowSubGraph): FlowSubGraph {
  const uuidMap = new Map<string, string>();
  const nodes: Node[] = graph.nodes.map((node) => {
    const uuid = crypto.randomUUID();
    uuidMap.set(node.uuid, uuid);
    return { ...node, uuid, data: regenerateNodeData(node.data) };
  });
  const connections: Connection[] = graph.connections.map((connection) => ({
    ...connection,
    id: crypto.randomUUID(),
    source_node_uuid: uuidMap.get(connection.source_node_uuid) ?? connection.source_node_uuid,
    target_node_uuid: uuidMap.get(connection.target_node_uuid) ?? connection.target_node_uuid,
  }));
  return { ...graph, nodes, connections };
}

/** 递归处理配置中嵌套的子图（当前仅 `flow:loop` 持有内联子图） */
function regenerateNodeData(data: NodeData): NodeData {
  if (data.type !== 'flow:loop') return data;
  return { ...data, body: regenerateSubGraph(data.body) };
}

/**
 * 深拷贝节点配置，并重生成其中嵌套子图的 UUID
 *
 * 复制/粘贴/快捷键复制节点时使用，保证副本与源节点不共享任何嵌套 UUID。
 * 剪贴板里的配置是脱敏后的松散对象（`Record<string, unknown>`），因此额外
 * 提供一个接受松散对象的重载，避免调用点做类型断言。
 */
export function cloneNodeData(data: NodeData): NodeData;
export function cloneNodeData(data: Record<string, unknown>): NodeData;
export function cloneNodeData(data: NodeData | Record<string, unknown>): NodeData {
  const cloned = JSON.parse(JSON.stringify(data)) as NodeData;
  return regenerateNodeData(cloned);
}
