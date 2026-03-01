/**
 * DEMO_FLOW 模板测试 (TDD - RED 阶段)
 * 这些测试预期失败，直到 Task 3 完成 DEMO_FLOW 的实现
 */
import { describe, it, expect } from 'vitest';
import { DEMO_FLOW } from '@/templates/builtinFlows';

describe('DEMO_FLOW 模板', () => {
  it('结构正确 - 包含 4 个节点', () => {
    expect(DEMO_FLOW.nodes.length).toBe(4);
    
    const nodeTypes = DEMO_FLOW.nodes.map(n => n.type);
    expect(nodeTypes).toContain('entry');
    expect(nodeTypes).toContain('http');  // 至少一个 HTTP 节点
    expect(nodeTypes).toContain('conditional');
  });

  it('变量引用语法正确 - 使用顶级键', () => {
    const conditionalNode = DEMO_FLOW.nodes.find(n => n.type === 'conditional');
    expect(conditionalNode).toBeDefined();
    
    // 条件节点应使用顶级键 'status'，而非 'http_response.status'
    const config = conditionalNode!.data as { field_path?: string };
    expect(config.field_path).toBe('status');
    expect(config.field_path).not.toContain('http_response');
  });
});
