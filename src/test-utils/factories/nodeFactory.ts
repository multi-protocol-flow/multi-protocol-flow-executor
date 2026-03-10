/**
 * Node 工厂函数 - 用于测试
 */

import type { Node, NodeData } from '@/types/node';

/**
 * 创建 Node 测试数据
 */
export function createNode(overrides: Partial<Node> = {}): Node {
  return {
    uuid: crypto.randomUUID(),
    type: 'entry',
    name: 'Test Node',
    position: { x: 0, y: 0 },
    data: { type: 'entry' },
    ...overrides,
  };
}

/**
 * 创建 HTTP Node 测试数据
 */
export function createHttpNode(overrides: Partial<Node> = {}): Node {
  const defaultData: NodeData = {
    type: 'http',
    url: 'https://example.com/api',
    method: 'get',
  };

  return createNode({
    type: 'http',
    name: 'HTTP Request',
    data: defaultData,
    ...overrides,
  });
}

/**
 * 创建 Conditional Node 测试数据
 */
export function createConditionalNode(overrides: Partial<Node> = {}): Node {
  const defaultData: NodeData = {
    type: 'conditional',
    condition_type: 'simple',
    field_path: 'status',
    operator: 'equal',
    expected_value: 200,
  };

  return createNode({
    type: 'conditional',
    name: 'Condition',
    data: defaultData,
    ...overrides,
  });
}
