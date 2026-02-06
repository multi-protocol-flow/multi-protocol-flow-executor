interface Position {
  x: number;
  y: number;
}

type TaskNodeType = "http" | "transform" | "condition" | "email" | "database";

interface NodeData {
  id: string;
  type: "start" | "task";
  taskType?: TaskNodeType;
  name: string;
  position: Position;
  createdAt: number;
  error?: string;
  isExecuting?: boolean;
}

/**
 * HTTP 节点数据接口
 * 定义 HTTP 请求节点的特定字段
 */
export interface HttpNodeData {
  url: string;
  method: 'GET' | 'POST' | 'PUT' | 'DELETE';
  headers?: Array<{ name: string; value: string }>;
}

interface Connection {
  id: string;
  source: string;
  target: string;
}

interface Workflow {
  id: string;
  name: string;
  nodes: NodeData[];
  connections: Connection[];
  createdAt: number;
  updatedAt: number;
}

export type { Position, TaskNodeType, NodeData, Connection, Workflow };
