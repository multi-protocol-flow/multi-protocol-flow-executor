/**
 * Flow 工厂函数 - 用于测试
 */

import type { Flow } from '@/types/flow';

/**
 * 创建 Flow 测试数据
 */
export function createFlow(overrides: Partial<Flow> = {}): Flow {
  return {
    uuid: crypto.randomUUID(),
    name: 'Test Flow',
    nodes: [],
    connections: [],
    initial_variables: {},
    ...overrides,
  };
}
