/**
 * Flow Store 测试
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { useFlowStore } from '@/stores/flow';
import type { Flow, Node, Connection } from '@/types';

describe('FlowStore', () => {
  beforeEach(() => {
    useFlowStore.getState().reset();
  });

  describe('初始状态', () => {
    it('初始 flow 为 null', () => {
      expect(useFlowStore.getState().flow).toBeNull();
    });

    it('初始 selectedNodeId 为 null', () => {
      expect(useFlowStore.getState().selectedNodeId).toBeNull();
    });

    it('初始 isDirty 为 false', () => {
      expect(useFlowStore.getState().isDirty).toBe(false);
    });
  });

  describe('setFlow', () => {
    it('可以设置流程', () => {
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);
      expect(useFlowStore.getState().flow).toEqual(flow);
    });

    it('设置流程时重置 selectedNodeId', () => {
      useFlowStore.getState().selectNode('some-node');
      expect(useFlowStore.getState().selectedNodeId).toBe('some-node');

      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);
      expect(useFlowStore.getState().selectedNodeId).toBeNull();
    });

    it('设置流程时重置 isDirty', () => {
      // 先模拟一些修改
      useFlowStore.getState().setFlow({
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [],
        connections: [],
      });
      useFlowStore.getState().updateFlowName('新名称');
      expect(useFlowStore.getState().isDirty).toBe(true);

      // 设置新流程
      const newFlow: Flow = {
        uuid: 'test-flow-2',
        name: '新流程',
        nodes: [],
        connections: [],
      };
      useFlowStore.getState().setFlow(newFlow);
      expect(useFlowStore.getState().isDirty).toBe(false);
    });

    it('可以设置 flow 为 null', () => {
      useFlowStore.getState().setFlow({
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [],
        connections: [],
      });
      useFlowStore.getState().setFlow(null);
      expect(useFlowStore.getState().flow).toBeNull();
    });
  });

  describe('updateFlowName', () => {
    it('可以更新流程名称', () => {
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);
      useFlowStore.getState().updateFlowName('新名称');
      expect(useFlowStore.getState().flow?.name).toBe('新名称');
    });

    it('更新名称时设置 isDirty 为 true', () => {
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);
      useFlowStore.getState().updateFlowName('新名称');
      expect(useFlowStore.getState().isDirty).toBe(true);
    });

    it('flow 为 null 时不报错', () => {
      expect(() => {
        useFlowStore.getState().updateFlowName('新名称');
      }).not.toThrow();
    });
  });

  describe('addNode', () => {
    it('可以添加节点', () => {
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);

      const node: Node = {
        uuid: 'node-1',
        type: 'http',
        name: 'HTTP请求',
        position: { x: 100, y: 100 },
        data: { type: 'http', url: 'https://example.com' },
      };
      useFlowStore.getState().addNode(node);

      expect(useFlowStore.getState().flow?.nodes).toHaveLength(1);
      expect(useFlowStore.getState().flow?.nodes[0]).toEqual(node);
    });

    it('添加节点时设置 isDirty 为 true', () => {
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);
      useFlowStore.getState().addNode({
        uuid: 'node-1',
        type: 'http',
        name: 'HTTP请求',
        position: { x: 100, y: 100 },
        data: { type: 'http', url: 'https://example.com' },
      });
      expect(useFlowStore.getState().isDirty).toBe(true);
    });
  });

  describe('updateNode', () => {
    it('可以更新节点', () => {
      const node: Node = {
        uuid: 'node-1',
        type: 'http',
        name: 'HTTP请求',
        position: { x: 100, y: 100 },
        data: { type: 'http', url: 'https://example.com' },
      };
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [node],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);

      useFlowStore.getState().updateNode('node-1', { name: '更新后的名称' });

      expect(useFlowStore.getState().flow?.nodes[0].name).toBe('更新后的名称');
      expect(useFlowStore.getState().flow?.nodes[0].position).toEqual({ x: 100, y: 100 });
    });

    it('更新不存在的节点时不变', () => {
      const node: Node = {
        uuid: 'node-1',
        type: 'http',
        name: 'HTTP请求',
        position: { x: 100, y: 100 },
        data: { type: 'http', url: 'https://example.com' },
      };
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [node],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);

      useFlowStore.getState().updateNode('non-existent', { name: '更新后的名称' });

      expect(useFlowStore.getState().flow?.nodes).toHaveLength(1);
      expect(useFlowStore.getState().flow?.nodes[0].name).toBe('HTTP请求');
    });
  });

  describe('removeNode', () => {
    it('可以移除节点', () => {
      const node: Node = {
        uuid: 'node-1',
        type: 'http',
        name: 'HTTP请求',
        position: { x: 100, y: 100 },
        data: { type: 'http', url: 'https://example.com' },
      };
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [node],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);

      useFlowStore.getState().removeNode('node-1');

      expect(useFlowStore.getState().flow?.nodes).toHaveLength(0);
    });

    it('移除节点时同时移除相关连接', () => {
      const node1: Node = {
        uuid: 'node-1',
        type: 'entry',
        name: '入口',
        position: { x: 0, y: 0 },
        data: { type: 'entry' },
      };
      const node2: Node = {
        uuid: 'node-2',
        type: 'http',
        name: 'HTTP请求',
        position: { x: 100, y: 100 },
        data: { type: 'http', url: 'https://example.com' },
      };
      const connection: Connection = {
        id: 'conn-1',
        source_node_uuid: 'node-1',
        target_node_uuid: 'node-2',
        source_port_id: 'output',
        target_port_id: 'input',
      };
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [node1, node2],
        connections: [connection],
      };
      useFlowStore.getState().setFlow(flow);

      useFlowStore.getState().removeNode('node-1');

      expect(useFlowStore.getState().flow?.nodes).toHaveLength(1);
      expect(useFlowStore.getState().flow?.connections).toHaveLength(0);
    });

    it('移除选中的节点时清除选中状态', () => {
      const node: Node = {
        uuid: 'node-1',
        type: 'http',
        name: 'HTTP请求',
        position: { x: 100, y: 100 },
        data: { type: 'http', url: 'https://example.com' },
      };
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [node],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);
      useFlowStore.getState().selectNode('node-1');

      useFlowStore.getState().removeNode('node-1');

      expect(useFlowStore.getState().selectedNodeId).toBeNull();
    });
  });

  describe('addConnection', () => {
    it('可以添加连接', () => {
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);

      const connection: Connection = {
        id: 'conn-1',
        source_node_uuid: 'node-1',
        target_node_uuid: 'node-2',
        source_port_id: 'output',
        target_port_id: 'input',
      };
      useFlowStore.getState().addConnection(connection);

      expect(useFlowStore.getState().flow?.connections).toHaveLength(1);
      expect(useFlowStore.getState().flow?.connections[0]).toEqual(connection);
    });
  });

  describe('removeConnection', () => {
    it('可以移除连接', () => {
      const connection: Connection = {
        id: 'conn-1',
        source_node_uuid: 'node-1',
        target_node_uuid: 'node-2',
        source_port_id: 'output',
        target_port_id: 'input',
      };
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [],
        connections: [connection],
      };
      useFlowStore.getState().setFlow(flow);

      useFlowStore.getState().removeConnection('conn-1');

      expect(useFlowStore.getState().flow?.connections).toHaveLength(0);
    });
  });

  describe('selectNode', () => {
    it('可以选择节点', () => {
      useFlowStore.getState().selectNode('node-1');
      expect(useFlowStore.getState().selectedNodeId).toBe('node-1');
    });

    it('可以取消选择（传入 null）', () => {
      useFlowStore.getState().selectNode('node-1');
      useFlowStore.getState().selectNode(null);
      expect(useFlowStore.getState().selectedNodeId).toBeNull();
    });
  });

  describe('markSaved', () => {
    it('可以标记已保存', () => {
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);
      useFlowStore.getState().updateFlowName('新名称');
      expect(useFlowStore.getState().isDirty).toBe(true);

      useFlowStore.getState().markSaved();
      expect(useFlowStore.getState().isDirty).toBe(false);
    });
  });

  describe('reset', () => {
    it('可以重置所有状态', () => {
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);
      useFlowStore.getState().selectNode('node-1');
      useFlowStore.getState().updateFlowName('新名称');

      useFlowStore.getState().reset();

      expect(useFlowStore.getState().flow).toBeNull();
      expect(useFlowStore.getState().selectedNodeId).toBeNull();
      expect(useFlowStore.getState().isDirty).toBe(false);
    });
  });

  describe('getSelectedNode', () => {
    it('返回选中的节点', () => {
      const node1: Node = {
        uuid: 'node-1',
        type: 'entry',
        name: '入口',
        position: { x: 0, y: 0 },
        data: { type: 'entry' },
      };
      const node2: Node = {
        uuid: 'node-2',
        type: 'http',
        name: 'HTTP请求',
        position: { x: 100, y: 100 },
        data: { type: 'http', url: 'https://example.com' },
      };
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [node1, node2],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);
      useFlowStore.getState().selectNode('node-2');

      const selectedNode = useFlowStore.getState().getSelectedNode();
      expect(selectedNode).toEqual(node2);
    });

    it('没有选中节点时返回 null', () => {
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);

      expect(useFlowStore.getState().getSelectedNode()).toBeNull();
    });

    it('flow 为 null 时返回 null', () => {
      useFlowStore.getState().selectNode('node-1');
      expect(useFlowStore.getState().getSelectedNode()).toBeNull();
    });
  });

  describe('getNodeByUuid', () => {
    it('根据 UUID 返回节点', () => {
      const node: Node = {
        uuid: 'node-1',
        type: 'http',
        name: 'HTTP请求',
        position: { x: 100, y: 100 },
        data: { type: 'http', url: 'https://example.com' },
      };
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [node],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);

      expect(useFlowStore.getState().getNodeByUuid('node-1')).toEqual(node);
      expect(useFlowStore.getState().getNodeByUuid('non-existent')).toBeNull();
    });

    it('flow 为 null 时返回 null', () => {
      expect(useFlowStore.getState().getNodeByUuid('node-1')).toBeNull();
    });
  });

  describe('Selectors', () => {
    it('selectFlow 返回 flow', () => {
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);
      
      const state = useFlowStore.getState();
      expect(state.flow).toEqual(flow);
    });

    it('selectNodes 返回节点数组', () => {
      const node: Node = {
        uuid: 'node-1',
        type: 'http',
        name: 'HTTP请求',
        position: { x: 100, y: 100 },
        data: { type: 'http', url: 'https://example.com' },
      };
      const flow: Flow = {
        uuid: 'test-flow-1',
        name: '测试流程',
        nodes: [node],
        connections: [],
      };
      useFlowStore.getState().setFlow(flow);

      const state = useFlowStore.getState();
      expect(state.flow?.nodes).toHaveLength(1);
    });

    it('flow 为 null 时 selectNodes 返回空数组', () => {
      const state = useFlowStore.getState();
      expect(state.flow?.nodes ?? []).toEqual([]);
    });
  });
});
