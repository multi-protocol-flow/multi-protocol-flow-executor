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

/** 自适应折行布局选项配置 */
export interface AdaptiveLayoutOptions {
  /**
   * 可视画布容器宽度（像素，通常为 canvasRef.current.clientWidth）
   * 默认 1200px
   */
  viewportWidth?: number;
  /**
   * 手动指定的每行最大列数（若不指定，则根据 viewportWidth 自动计算）
   */
  maxColumns?: number;
  /**
   * 节点列间距（水平距离），默认 80px
   */
  colGap?: number;
  /**
   * 折行间距（垂直行与行之间的安全通道距离），默认 120px
   */
  rowGap?: number;
  /**
   * 左侧起始边距，默认 80px
   */
  leftMargin?: number;
  /**
   * 顶部起始边距，默认 80px
   */
  topMargin?: number;
}

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

/**
 * 计算节点在可视画布自适应折行下的新位置
 *
 * 根据当前视口宽度动态计算每行可容纳的节点列数，
 * 当流程横向超出可视范围时自动折行到底部，保证在适度缩放下全流程清晰可读。
 *
 * @param nodes - 流程节点列表
 * @param connections - 流程连接列表
 * @param options - 布局配置参数（可视宽度、列间距、行间距等）
 * @returns 每个节点的新位置 Map（uuid → {x, y}，左上角坐标）
 */
export function getAdaptiveLayoutedPositions(
  nodes: Node[],
  connections: Connection[],
  options?: AdaptiveLayoutOptions
): Map<string, { x: number; y: number }> {
  const positions = new Map<string, { x: number; y: number }>();
  if (nodes.length === 0) return positions;

  const colGap = options?.colGap ?? 80;
  const rowGap = options?.rowGap ?? 120;
  const leftMargin = options?.leftMargin ?? 80;
  const topMargin = options?.topMargin ?? 80;
  const viewportWidth = options?.viewportWidth ?? 1200;

  // 1. 过滤有效连接
  const nodeMap = new Map<string, Node>(nodes.map(n => [n.uuid, n]));
  const validConns = connections.filter(
    c => nodeMap.has(c.source_node_uuid) && nodeMap.has(c.target_node_uuid)
  );

  // 2. 划分为弱连通分量 (Weakly Connected Components)
  const adj = new Map<string, string[]>();
  for (const n of nodes) adj.set(n.uuid, []);
  for (const conn of validConns) {
    adj.get(conn.source_node_uuid)?.push(conn.target_node_uuid);
    adj.get(conn.target_node_uuid)?.push(conn.source_node_uuid);
  }

  const visited = new Set<string>();
  const components: Node[][] = [];

  for (const node of nodes) {
    if (visited.has(node.uuid)) continue;
    const comp: Node[] = [];
    const queue: string[] = [node.uuid];
    visited.add(node.uuid);

    while (queue.length > 0) {
      const u = queue.shift()!;
      const n = nodeMap.get(u);
      if (n) comp.push(n);
      for (const v of adj.get(u) ?? []) {
        if (!visited.has(v)) {
          visited.add(v);
          queue.push(v);
        }
      }
    }
    components.push(comp);
  }

  // 优先排序含 entry 节点的子图，再按节点数量降序排列
  components.sort((a, b) => {
    const aEntry = a.some(n => n.type === 'entry') ? 1 : 0;
    const bEntry = b.some(n => n.type === 'entry') ? 1 : 0;
    if (aEntry !== bEntry) return bEntry - aEntry;
    return b.length - a.length;
  });

  // 分离有连接关系的图分量与孤立节点
  const graphComponents: Node[][] = [];
  const isolatedNodes: Node[] = [];

  for (const comp of components) {
    if (comp.length === 1) {
      const uuid = comp[0].uuid;
      const hasEdges = validConns.some(
        c => c.source_node_uuid === uuid || c.target_node_uuid === uuid
      );
      if (!hasEdges) {
        isolatedNodes.push(comp[0]);
        continue;
      }
    }
    graphComponents.push(comp);
  }

  // 3. 计算视口容量与每行最大列数
  const availableWidth = Math.max(300, viewportWidth - leftMargin * 2);
  const computedColumns = Math.max(
    1,
    Math.floor((availableWidth + colGap) / (NODE_WIDTH + colGap))
  );
  const maxColumns = options?.maxColumns && options.maxColumns > 0
    ? options.maxColumns
    : computedColumns;

  let currentY = topMargin;

  // 4. 对每个连通分量进行拓扑分析与折行排布
  for (const comp of graphComponents) {
    const compUuids = new Set(comp.map(n => n.uuid));
    const compConns = validConns.filter(
      c => compUuids.has(c.source_node_uuid) && compUuids.has(c.target_node_uuid)
    );

    const g = new dagre.graphlib.Graph();
    g.setDefaultEdgeLabel(() => ({}));
    g.setGraph({
      rankdir: 'LR',
      nodesep: 50,
      ranksep: colGap,
      align: 'UL',
      acyclicer: 'greedy',
      edgesep: 15,
    });

    const sortedCompNodes = topologicalSort(comp, compConns);
    for (const node of sortedCompNodes) {
      g.setNode(node.uuid, { width: NODE_WIDTH, height: NODE_HEIGHT });
    }

    const falseConns = compConns.filter(c => c.source_port_id === 'false');
    const trueConns = compConns.filter(c => c.source_port_id === 'true');
    const otherConns = compConns.filter(
      c => c.source_port_id !== 'true' && c.source_port_id !== 'false'
    );

    for (const conn of [...falseConns, ...otherConns, ...trueConns]) {
      const weight = conn.source_port_id === 'true' ? 10 : 1;
      g.setEdge(conn.source_node_uuid, conn.target_node_uuid, { weight });
    }

    dagre.layout(g);

    // 提取节点 Rank 与相对坐标
    interface NodeLayoutInfo {
      uuid: string;
      x: number;
      y: number;
      rank: number;
      rankIdx: number;
      colIdx: number;
    }

    const rawLayouts = comp.map(n => {
      const info = g.node(n.uuid) || { x: 0, y: 0, rank: 0, order: 0 };
      return { uuid: n.uuid, x: info.x, y: info.y, rank: info.rank };
    });

    const uniqueRanks = Array.from(new Set(rawLayouts.map(n => n.rank))).sort((a, b) => a - b);
    const rankToIndex = new Map<number, number>(uniqueRanks.map((r, i) => [r, i]));

    // 按每行 maxColumns 列划分行分段 (Segment)
    const segments: NodeLayoutInfo[][] = [];
    for (const item of rawLayouts) {
      const rankIdx = rankToIndex.get(item.rank) ?? 0;
      const segmentIdx = Math.floor(rankIdx / maxColumns);
      const colIdx = rankIdx % maxColumns;
      if (!segments[segmentIdx]) segments[segmentIdx] = [];
      segments[segmentIdx].push({ ...item, rankIdx, colIdx });
    }

    // 计算各段内坐标并分配给最终位置
    for (const seg of segments) {
      if (!seg || seg.length === 0) continue;
      const ys = seg.map(n => n.y);
      const minY = Math.min(...ys);
      const maxY = Math.max(...ys);
      const segHeight = maxY - minY;

      for (const n of seg) {
        const posX = leftMargin + n.colIdx * (NODE_WIDTH + colGap);
        const posY = currentY + (n.y - minY);
        positions.set(n.uuid, { x: posX, y: posY });
      }

      currentY += segHeight + NODE_HEIGHT + rowGap;
    }

    // 独立子流程之间的垂直间隔
    currentY += 40;
  }

  // 5. 孤立节点在底部整齐排列为网格
  if (isolatedNodes.length > 0) {
    isolatedNodes.forEach((node, idx) => {
      const colIdx = idx % maxColumns;
      const rowIdx = Math.floor(idx / maxColumns);
      const posX = leftMargin + colIdx * (NODE_WIDTH + colGap);
      const posY = currentY + rowIdx * (NODE_HEIGHT + 40);
      positions.set(node.uuid, { x: posX, y: posY });
    });
  }

  return positions;
}
