/**
 * Dagre 自动布局工具
 * 使用 @dagrejs/dagre 对画布节点进行自动排列
 */
import dagre from '@dagrejs/dagre';
import type { Node, Connection } from '@/types';

/** 布局方向 */
export type LayoutDirection = 'LR';

/** 所有节点统一尺寸 */
const NODE_WIDTH = 180;
const NODE_HEIGHT = 80;

/** 布局参数配置 */
const LAYOUT_CONFIG = {
  LR: { nodesep: 50, ranksep: 100 },
} as const;

/**
 * 按广度优先拓扑序排序节点
 * 从 entry 节点开始 BFS，保证汇合中的分支节点顺序一致
 */
function topologicalSort(nodes: Node[], connections: Connection[]): Node[] {
  const nodeMap = new Map(nodes.map(n => [n.uuid, n]));
  const inDegree = new Map<string, number>();
  const adjacency = new Map<string, string[]>();

  for (const n of nodes) {
    inDegree.set(n.uuid, 0);
    adjacency.set(n.uuid, []);
  }
  for (const conn of connections) {
    adjacency.get(conn.source_node_uuid)?.push(conn.target_node_uuid);
    inDegree.set(conn.target_node_uuid, (inDegree.get(conn.target_node_uuid) ?? 0) + 1);
  }

  // 从 entry 节点开始 BFS（多源，但优先 entry）
  const entries = nodes.filter(n => n.type === 'entry').map(n => n.uuid);
  const sources = nodes.filter(n => (inDegree.get(n.uuid) ?? 0) === 0).map(n => n.uuid);
  const queue = [...new Set([...entries, ...sources])];
  const visited = new Set<string>();
  const sorted: string[] = [];

  while (queue.length > 0) {
    const uuid = queue.shift()!;
    if (visited.has(uuid)) continue;
    visited.add(uuid);
    sorted.push(uuid);
    for (const next of adjacency.get(uuid) ?? []) {
      const deg = (inDegree.get(next) ?? 1) - 1;
      inDegree.set(next, deg);
      if (deg === 0) queue.push(next);
    }
  }

  // 补上孤立节点
  for (const n of nodes) {
    if (!visited.has(n.uuid)) sorted.push(n.uuid);
  }

  return sorted.map(uuid => nodeMap.get(uuid)!).filter(Boolean);
}

/**
 * 计算节点自动布局后的新位置
 *
 * @param nodes - 流程节点列表
 * @param connections - 流程连接列表
 * @param direction - 布局方向（LR=水平）
 * @returns 每个节点的新位置 Map（uuid → {x, y}，左上角坐标）
 */
export function getLayoutedPositions(
  nodes: Node[],
  connections: Connection[],
  direction: LayoutDirection = 'LR'
): Map<string, { x: number; y: number }> {
  const positions = new Map<string, { x: number; y: number }>();

  if (nodes.length === 0) return positions;

  const g = new dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));

  const config = LAYOUT_CONFIG[direction];
  g.setGraph({
    rankdir: direction,
    nodesep: config.nodesep,
    ranksep: config.ranksep,
    align: 'UL',
    acyclicer: 'greedy',
    edgesep: 15,
  });

  // 按拓扑序注册节点，影响 dagre 内部初始排序
  const sortedNodes = topologicalSort(nodes, connections);
  for (const node of sortedNodes) {
    g.setNode(node.uuid, { width: NODE_WIDTH, height: NODE_HEIGHT });
  }

  // 注册边：false 分支先注册（让 dagre DFS 优先发现 side-branch 的节点）
  // true 分支后注册（权重高，layout 中拉直）
  const falseConns = connections.filter(c => c.source_port_id === 'false');
  const trueConns = connections.filter(c => c.source_port_id === 'true');
  const otherConns = connections.filter(
    c => c.source_port_id !== 'true' && c.source_port_id !== 'false'
  );

  for (const conn of [...falseConns, ...otherConns, ...trueConns]) {
    const weight = conn.source_port_id === 'true' ? 10 : 1;
    g.setEdge(conn.source_node_uuid, conn.target_node_uuid, { weight });
  }

  // 执行布局
  dagre.layout(g);

  // 提取结果：dagre 返回中心坐标，转换为左上角坐标
  for (const node of nodes) {
    const pos = g.node(node.uuid);
    if (pos) {
      positions.set(node.uuid, {
        x: pos.x - NODE_WIDTH / 2,
        y: pos.y - NODE_HEIGHT / 2,
      });
    }
  }

  return positions;
}
