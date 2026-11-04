/**
 * cloneNodeData 测试
 *
 * 回归背景：复制/粘贴 loop 节点时若只给根节点换 UUID，副本与原节点会共享
 * body 内的节点/连线 UUID；`locateNode()` 只返回第一个匹配，导致改副本改到原节点。
 */
import { describe, expect, it } from 'vitest';
import { cloneNodeData } from '@/lib/nodeClone';
import type { NodeData } from '@/types';

/** 构造双层嵌套的 loop 配置：outer -> inner -> delay */
function makeNestedLoopData(): NodeData {
  return {
    type: 'flow:loop',
    source: 'count',
    count: 2,
    max_iterations: 100,
    concurrency: 1,
    break_on_error: true,
    report: { detail_iterations: 20, always_keep_failed: true },
    body: {
      nodes: [
        {
          uuid: 'inner-loop',
          type: 'flow:loop',
          name: 'Inner',
          position: { x: 0, y: 0 },
          data: {
            type: 'flow:loop',
            source: 'count',
            count: 1,
            max_iterations: 100,
            concurrency: 1,
            break_on_error: true,
            report: { detail_iterations: 20, always_keep_failed: true },
            body: {
              nodes: [
                {
                  uuid: 'deep-delay',
                  type: 'flow:delay',
                  name: 'Deep',
                  position: { x: 0, y: 0 },
                  data: { type: 'flow:delay', duration_ms: '10', jitter_ms: 0 },
                },
              ],
              connections: [],
            },
          },
        },
        {
          uuid: 'body-script',
          type: 'script',
          name: 'Script',
          position: { x: 100, y: 0 },
          data: { type: 'script', script: 'return 1;' },
        },
      ],
      connections: [
        {
          id: 'body-conn',
          source_node_uuid: 'inner-loop',
          target_node_uuid: 'body-script',
          source_port_id: 'done',
          target_port_id: 'in',
        },
      ],
    },
  };
}

describe('cloneNodeData', () => {
  it('深拷贝配置且不修改源数据', () => {
    const source = makeNestedLoopData();
    const snapshot = JSON.stringify(source);
    const cloned = cloneNodeData(source);

    expect(JSON.stringify(source)).toBe(snapshot);
    expect(cloned).not.toBe(source);
    expect(cloned).toEqual(expect.objectContaining({ type: 'flow:loop' }));
  });

  it('重生成所有嵌套层级的节点 UUID', () => {
    const source = makeNestedLoopData();
    const cloned = cloneNodeData(source);

    if (source.type !== 'flow:loop' || cloned.type !== 'flow:loop') {
      throw new Error('expected flow:loop');
    }
    const srcInner = source.body.nodes.find((n) => n.uuid === 'inner-loop');
    const clonedInner = cloned.body.nodes.find((n) => n.name === 'Inner');
    expect(srcInner).toBeTruthy();
    expect(clonedInner).toBeTruthy();
    expect(clonedInner!.uuid).not.toBe('inner-loop');

    // 第二层同样要换
    if (clonedInner!.data.type !== 'flow:loop') throw new Error('expected inner loop');
    expect(clonedInner!.data.body.nodes[0].uuid).not.toBe('deep-delay');

    // 非 loop 的 body 节点同样换 UUID
    const clonedScript = cloned.body.nodes.find((n) => n.name === 'Script');
    expect(clonedScript!.uuid).not.toBe('body-script');
  });

  it('连线 id 与端点同步改写为新 UUID', () => {
    const source = makeNestedLoopData();
    const cloned = cloneNodeData(source);
    if (cloned.type !== 'flow:loop') throw new Error('expected flow:loop');

    const [connection] = cloned.body.connections;
    expect(connection.id).not.toBe('body-conn');
    expect(connection.source_node_uuid).not.toBe('inner-loop');
    expect(connection.target_node_uuid).not.toBe('body-script');

    // 端点必须指向副本自己的节点
    const clonedUuids = cloned.body.nodes.map((n) => n.uuid);
    expect(clonedUuids).toContain(connection.source_node_uuid);
    expect(clonedUuids).toContain(connection.target_node_uuid);
  });

  it('两次复制互不共享 UUID', () => {
    const source = makeNestedLoopData();
    const first = cloneNodeData(source);
    const second = cloneNodeData(source);
    if (first.type !== 'flow:loop' || second.type !== 'flow:loop') {
      throw new Error('expected flow:loop');
    }
    const firstUuids = first.body.nodes.map((n) => n.uuid);
    const secondUuids = second.body.nodes.map((n) => n.uuid);
    expect(firstUuids.some((u) => secondUuids.includes(u))).toBe(false);
  });

  it('非 loop 节点只做深拷贝', () => {
    const source: NodeData = { type: 'flow:delay', duration_ms: '10', jitter_ms: 0 };
    const cloned = cloneNodeData(source);
    expect(cloned).toEqual(source);
    expect(cloned).not.toBe(source);
  });
});
