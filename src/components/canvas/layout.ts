/**
 * Dagre 自动布局工具
 * 使用 @dagrejs/dagre 对画布节点进行自动排列
 */
import dagre from '@dagrejs/dagre';
import type { Node, Connection } from '@/types';

/** 布局方向 */
export type LayoutDirection = 'LR' | 'TB';

/** 所有节点统一尺寸 */
const NODE_WIDTH = 180;
const NODE_HEIGHT = 80;

/** 布局参数配置 */
const LAYOUT_CONFIG = {
  LR: { nodesep: 50, ranksep: 100 },
  TB: { nodesep: 60, ranksep: 80 },
} as const;

/**
 * 计算节点自动布局后的新位置
 *
 * @param nodes - 流程节点列表
 * @param connections - 流程连接列表
 * @param direction - 布局方向（LR=水平，TB=垂直）
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
  });

  // 注册所有节点
  for (const node of nodes) {
    g.setNode(node.uuid, { width: NODE_WIDTH, height: NODE_HEIGHT });
  }

  // 注册所有边（连接）
  for (const conn of connections) {
    g.setEdge(conn.source_node_uuid, conn.target_node_uuid);
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
