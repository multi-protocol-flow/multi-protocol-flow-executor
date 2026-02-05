import type { Node as CustomNode } from './node';
import type { Connection as CustomConnection } from './connection';

/**
 * 流程接口
 */
export interface Flow {
  id: string;
  name: string;
  nodes: CustomNode[];
  connections: CustomConnection[];
}

/**
 * Flow interface using custom Node type
 */
export interface Flow {
  id: string;
  name: string;
  nodes: CustomNode[];
  connections: CustomConnection[];
}
