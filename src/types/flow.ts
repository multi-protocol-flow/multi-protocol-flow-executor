import type { Node } from './node';
import type { Connection } from './connection';

/**
 * 流程接口
 */
export interface Flow {
  id: string;
  name: string;
  nodes: Node[];
  connections: Connection[];
}
