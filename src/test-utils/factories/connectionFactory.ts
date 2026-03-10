/**
 * Connection 工厂函数 - 用于测试
 */

import type { Connection } from '@/types/connection';

/**
 * 创建 Connection 测试数据
 */
export function createConnection(overrides: Partial<Connection> = {}): Connection {
  return {
    id: crypto.randomUUID(),
    source_node_uuid: 'source-uuid',
    target_node_uuid: 'target-uuid',
    source_port_id: 'output',
    target_port_id: 'input',
    ...overrides,
  };
}
