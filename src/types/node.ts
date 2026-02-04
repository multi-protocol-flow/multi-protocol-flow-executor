/**
 * 节点类型枚举
 */
export enum NodeType {
  Start = 'start',
  Http = 'http',
  Conditional = 'conditional',
  Transform = 'transform',
  End = 'end',
}

/**
 * 节点位置接口
 */
export interface NodePosition {
  x: number;
  y: number;
}

/**
 * 节点数据接口（使用 any 以支持不同类型节点的特定数据）
 */
export type NodeData = any;

/**
 * 节点接口
 */
export interface Node {
  id: string;
  type: NodeType;
  name: string;
  position: NodePosition;
  data: NodeData;
  ports?: any[]; // 端口数组，类型待定
}
