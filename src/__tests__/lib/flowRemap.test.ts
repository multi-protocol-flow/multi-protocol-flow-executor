import { describe, it, expect } from 'vitest';
import { remapFlowUuids } from '@/lib/flowRemap';
import type { Flow, NodeData, NodeType } from '@/types';

describe('remapFlowUuids', () => {
  it('生成全新的 Flow UUID 与节点/连线 UUID，并重映射关联引用', () => {
    const originalFlow: Flow = {
      uuid: 'flow-origin',
      name: 'Original Flow',
      nodes: [
        {
          uuid: 'node-1',
          type: 'tcp:connect',
          name: 'TCP Connect',
          position: { x: 0, y: 0 },
          data: { type: 'tcp:connect', host: '127.0.0.1', port: 8080 } as unknown as NodeData,
        },
        {
          uuid: 'node-2',
          type: 'tcp:send',
          name: 'TCP Send',
          position: { x: 100, y: 100 },
          data: { type: 'tcp:send', connection_id: 'node-1', data: 'hello' } as unknown as NodeData,
        },
        {
          uuid: 'node-3',
          type: 'variable:extractor',
          name: 'Extractor',
          position: { x: 200, y: 200 },
          data: { type: 'variable:extractor', source_node_id: 'node-2', mappings: [] } as unknown as NodeData,
        },
      ],
      connections: [
        {
          id: 'conn-1',
          source_node_uuid: 'node-1',
          target_node_uuid: 'node-2',
          source_port_id: 'output',
          target_port_id: 'input',
        },
      ],
    };

    const remapped = remapFlowUuids(originalFlow, { newName: 'Copied Flow' });

    // 1. 流程 UUID 与名称变更
    expect(remapped.uuid).not.toBe('flow-origin');
    expect(remapped.name).toBe('Copied Flow');

    // 2. 节点 UUID 全部更新且互不相同
    expect(remapped.nodes).toHaveLength(3);
    const [newNode1, newNode2, newNode3] = remapped.nodes;
    expect(newNode1.uuid).not.toBe('node-1');
    expect(newNode2.uuid).not.toBe('node-2');
    expect(newNode3.uuid).not.toBe('node-3');
    expect(newNode1.uuid).not.toBe(newNode2.uuid);
    expect(newNode2.uuid).not.toBe(newNode3.uuid);

    // 3. 连线 ID 更新且端点指向新节点 UUID
    expect(remapped.connections).toHaveLength(1);
    expect(remapped.connections[0].id).not.toBe('conn-1');
    expect(remapped.connections[0].source_node_uuid).toBe(newNode1.uuid);
    expect(remapped.connections[0].target_node_uuid).toBe(newNode2.uuid);

    // 4. 节点内部跨节点引用已级联更新
    const node2Data = newNode2.data as unknown as Record<string, unknown>;
    const node3Data = newNode3.data as unknown as Record<string, unknown>;
    expect(node2Data.connection_id).toBe(newNode1.uuid);
    expect(node3Data.source_node_id).toBe(newNode2.uuid);
  });

  it('正确重映射数据库节点的 connection_uuid 字段', () => {
    const dbFlow: Flow = {
      uuid: 'flow-db',
      name: 'DB Flow',
      nodes: [
        {
          uuid: 'db-conn-1',
          type: 'db:connect' as unknown as NodeType,
          name: 'DB Connect',
          position: { x: 0, y: 0 },
          data: { type: 'db:connect', connection_uuid: '' } as unknown as NodeData,
        },
        {
          uuid: 'db-query-1',
          type: 'db:query' as unknown as NodeType,
          name: 'DB Query',
          position: { x: 200, y: 0 },
          data: { type: 'db:query', connection_uuid: 'db-conn-1', sql: 'SELECT 1' } as unknown as NodeData,
        },
      ],
      connections: [],
    };

    const remapped = remapFlowUuids(dbFlow);
    const [newConnNode, newQueryNode] = remapped.nodes;
    expect(newConnNode.uuid).not.toBe('db-conn-1');
    const queryData = newQueryNode.data as unknown as Record<string, unknown>;
    expect(queryData.connection_uuid).toBe(newConnNode.uuid);
  });
});
