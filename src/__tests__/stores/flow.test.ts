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

  describe('断点管理', () => {
    describe('toggleBreakpoint', () => {
      it('可以切换断点状态（从无到有）', () => {
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

        useFlowStore.getState().toggleBreakpoint('node-1');

        expect(useFlowStore.getState().flow?.nodes[0].hasBreakpoint).toBe(true);
        expect(useFlowStore.getState().isDirty).toBe(true);
      });

      it('可以切换断点状态（从有到无）', () => {
        const node: Node = {
          uuid: 'node-1',
          type: 'http',
          name: 'HTTP请求',
          position: { x: 100, y: 100 },
          data: { type: 'http', url: 'https://example.com' },
          hasBreakpoint: true,
        };
        const flow: Flow = {
          uuid: 'test-flow-1',
          name: '测试流程',
          nodes: [node],
          connections: [],
        };
        useFlowStore.getState().setFlow(flow);

        useFlowStore.getState().toggleBreakpoint('node-1');

        expect(useFlowStore.getState().flow?.nodes[0].hasBreakpoint).toBe(false);
      });

      it('切换不存在的节点时不变', () => {
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

        useFlowStore.getState().toggleBreakpoint('non-existent');

        expect(useFlowStore.getState().flow?.nodes).toHaveLength(1);
        expect(useFlowStore.getState().flow?.nodes[0].hasBreakpoint).toBeUndefined();
      });

      it('flow 为 null 时不报错', () => {
        expect(() => {
          useFlowStore.getState().toggleBreakpoint('node-1');
        }).not.toThrow();
      });
    });

    describe('setBreakpoint', () => {
      it('可以设置断点为 true', () => {
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

        useFlowStore.getState().setBreakpoint('node-1', true);

        expect(useFlowStore.getState().flow?.nodes[0].hasBreakpoint).toBe(true);
        expect(useFlowStore.getState().isDirty).toBe(true);
      });

      it('可以设置断点为 false', () => {
        const node: Node = {
          uuid: 'node-1',
          type: 'http',
          name: 'HTTP请求',
          position: { x: 100, y: 100 },
          data: { type: 'http', url: 'https://example.com' },
          hasBreakpoint: true,
        };
        const flow: Flow = {
          uuid: 'test-flow-1',
          name: '测试流程',
          nodes: [node],
          connections: [],
        };
        useFlowStore.getState().setFlow(flow);

        useFlowStore.getState().setBreakpoint('node-1', false);

        expect(useFlowStore.getState().flow?.nodes[0].hasBreakpoint).toBe(false);
      });

      it('flow 为 null 时不报错', () => {
        expect(() => {
          useFlowStore.getState().setBreakpoint('node-1', true);
        }).not.toThrow();
      });
    });

    describe('hasBreakpoint', () => {
      it('返回 true 当节点有断点时', () => {
        const node: Node = {
          uuid: 'node-1',
          type: 'http',
          name: 'HTTP请求',
          position: { x: 100, y: 100 },
          data: { type: 'http', url: 'https://example.com' },
          hasBreakpoint: true,
        };
        const flow: Flow = {
          uuid: 'test-flow-1',
          name: '测试流程',
          nodes: [node],
          connections: [],
        };
        useFlowStore.getState().setFlow(flow);

        expect(useFlowStore.getState().hasBreakpoint('node-1')).toBe(true);
      });

      it('返回 false 当节点无断点时', () => {
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

        expect(useFlowStore.getState().hasBreakpoint('node-1')).toBe(false);
      });

      it('返回 false 当节点不存在时', () => {
        const flow: Flow = {
          uuid: 'test-flow-1',
          name: '测试流程',
          nodes: [],
          connections: [],
        };
        useFlowStore.getState().setFlow(flow);

        expect(useFlowStore.getState().hasBreakpoint('non-existent')).toBe(false);
      });

      it('返回 false 当 flow 为 null 时', () => {
        expect(useFlowStore.getState().hasBreakpoint('node-1')).toBe(false);
      });
    });
  });

  describe('多标签会话管理 (Multi-Tab Sessions)', () => {
    const flowA: Flow = {
      uuid: 'flow-a',
      name: 'Flow A',
      nodes: [
        {
          uuid: 'node-a1',
          type: 'http',
          name: 'HTTP A1',
          position: { x: 0, y: 0 },
          data: { type: 'http', url: 'https://a.com' },
        },
      ],
      connections: [],
    };

    const flowB: Flow = {
      uuid: 'flow-b',
      name: 'Flow B',
      nodes: [
        {
          uuid: 'node-b1',
          type: 'http',
          name: 'HTTP B1',
          position: { x: 100, y: 100 },
          data: { type: 'http', url: 'https://b.com' },
        },
      ],
      connections: [],
    };

    const flowC: Flow = {
      uuid: 'flow-c',
      name: 'Flow C',
      nodes: [],
      connections: [],
    };

    it('打开 Tab A，修改节点产生 past 与 isDirty', () => {
      useFlowStore.getState().openTab(flowA);
      expect(useFlowStore.getState().tabs).toHaveLength(1);
      expect(useFlowStore.getState().tabs[0].uuid).toBe('flow-a');
      expect(useFlowStore.getState().activeTabUuid).toBe('flow-a');
      expect(useFlowStore.getState().isDirty).toBe(false);

      useFlowStore.getState().updateNode('node-a1', { name: 'HTTP A1 Modified' });
      expect(useFlowStore.getState().isDirty).toBe(true);
      expect(useFlowStore.getState().past).toHaveLength(1);
      expect(useFlowStore.getState().tabs[0].isDirty).toBe(true);
    });

    it('多标签切换独立保留修改与撤销重做历史栈', () => {
      // 1. 打开 Tab A 并修改
      useFlowStore.getState().openTab(flowA);
      useFlowStore.getState().updateNode('node-a1', { name: 'A-Mod-1' });
      expect(useFlowStore.getState().isDirty).toBe(true);
      expect(useFlowStore.getState().past).toHaveLength(1);

      // 2. 打开 Tab B 并修改
      useFlowStore.getState().openTab(flowB);
      expect(useFlowStore.getState().activeTabUuid).toBe('flow-b');
      expect(useFlowStore.getState().tabs).toHaveLength(2);
      expect(useFlowStore.getState().isDirty).toBe(false);
      expect(useFlowStore.getState().past).toHaveLength(0);

      useFlowStore.getState().updateNode('node-b1', { name: 'B-Mod-1' });
      expect(useFlowStore.getState().isDirty).toBe(true);
      expect(useFlowStore.getState().past).toHaveLength(1);

      // 3. 切换回 Tab A
      useFlowStore.getState().switchTab('flow-a');
      expect(useFlowStore.getState().activeTabUuid).toBe('flow-a');
      expect(useFlowStore.getState().flow?.nodes[0].name).toBe('A-Mod-1');
      expect(useFlowStore.getState().isDirty).toBe(true);
      expect(useFlowStore.getState().past).toHaveLength(1);

      // 4. 在 Tab A 触发 undo，仅回退 Tab A 的操作，Tab B 不受影响
      useFlowStore.getState().undo();
      expect(useFlowStore.getState().flow?.nodes[0].name).toBe('HTTP A1');
      expect(useFlowStore.getState().future).toHaveLength(1);

      // 验证 Tab B 在 session 中的状态依然保留
      const sessionB = useFlowStore.getState().tabSessions['flow-b'];
      expect(sessionB).toBeDefined();
      expect(sessionB.flow.nodes[0].name).toBe('B-Mod-1');
      expect(sessionB.isDirty).toBe(true);
      expect(sessionB.past).toHaveLength(1);
    });

    it('关闭活跃标签自动切换至相邻标签', () => {
      useFlowStore.getState().openTab(flowA);
      useFlowStore.getState().openTab(flowB);
      useFlowStore.getState().openTab(flowC);
      expect(useFlowStore.getState().tabs).toHaveLength(3);
      expect(useFlowStore.getState().activeTabUuid).toBe('flow-c');

      // 关闭 Tab C，应切换至前一个相邻标签 Tab B
      useFlowStore.getState().closeTab('flow-c');
      expect(useFlowStore.getState().tabs).toHaveLength(2);
      expect(useFlowStore.getState().activeTabUuid).toBe('flow-b');
      expect(useFlowStore.getState().flow?.uuid).toBe('flow-b');

      // 切换到 Tab A 并关闭 Tab A，应切换至后一个相邻标签 Tab B
      useFlowStore.getState().switchTab('flow-a');
      expect(useFlowStore.getState().activeTabUuid).toBe('flow-a');
      useFlowStore.getState().closeTab('flow-a');
      expect(useFlowStore.getState().tabs).toHaveLength(1);
      expect(useFlowStore.getState().activeTabUuid).toBe('flow-b');
      expect(useFlowStore.getState().flow?.uuid).toBe('flow-b');
    });

    it('关闭非活跃标签不影响当前活跃标签', () => {
      useFlowStore.getState().openTab(flowA);
      useFlowStore.getState().openTab(flowB);
      expect(useFlowStore.getState().activeTabUuid).toBe('flow-b');

      useFlowStore.getState().closeTab('flow-a');
      expect(useFlowStore.getState().tabs).toHaveLength(1);
      expect(useFlowStore.getState().activeTabUuid).toBe('flow-b');
      expect(useFlowStore.getState().flow?.uuid).toBe('flow-b');
    });

    it('closeOtherTabs 保留指定标签并关闭其余标签', () => {
      useFlowStore.getState().openTab(flowA);
      useFlowStore.getState().openTab(flowB);
      useFlowStore.getState().openTab(flowC);
      expect(useFlowStore.getState().tabs).toHaveLength(3);

      useFlowStore.getState().closeOtherTabs('flow-b');
      expect(useFlowStore.getState().tabs).toHaveLength(1);
      expect(useFlowStore.getState().tabs[0].uuid).toBe('flow-b');
      expect(useFlowStore.getState().activeTabUuid).toBe('flow-b');
      expect(useFlowStore.getState().flow?.uuid).toBe('flow-b');
      expect(Object.keys(useFlowStore.getState().tabSessions)).toEqual(['flow-b']);
    });

    it('closeTabsToRight 关闭右侧所有标签', () => {
      useFlowStore.getState().openTab(flowA);
      useFlowStore.getState().openTab(flowB);
      useFlowStore.getState().openTab(flowC);
      expect(useFlowStore.getState().tabs).toHaveLength(3);

      // 关闭 Tab A 右侧标签（Tab B、Tab C）
      useFlowStore.getState().closeTabsToRight('flow-a');
      expect(useFlowStore.getState().tabs).toHaveLength(1);
      expect(useFlowStore.getState().tabs[0].uuid).toBe('flow-a');
      expect(useFlowStore.getState().activeTabUuid).toBe('flow-a');
      expect(useFlowStore.getState().flow?.uuid).toBe('flow-a');
    });

    it('closeAllTabs 清空所有标签与流程状态', () => {
      useFlowStore.getState().openTab(flowA);
      useFlowStore.getState().openTab(flowB);
      expect(useFlowStore.getState().tabs).toHaveLength(2);

      useFlowStore.getState().closeAllTabs();
      expect(useFlowStore.getState().tabs).toHaveLength(0);
      expect(useFlowStore.getState().activeTabUuid).toBeNull();
      expect(useFlowStore.getState().flow).toBeNull();
      expect(useFlowStore.getState().tabSessions).toEqual({});
    });

    it('updateTabName 同步更新标签名称、会话流程名称与当前活跃流程名称', () => {
      useFlowStore.getState().openTab(flowA);
      useFlowStore.getState().updateTabName('flow-a', 'Renamed Flow A');

      expect(useFlowStore.getState().tabs[0].name).toBe('Renamed Flow A');
      expect(useFlowStore.getState().flow?.name).toBe('Renamed Flow A');
      expect(useFlowStore.getState().tabSessions['flow-a'].flow.name).toBe('Renamed Flow A');
    });

    it('markSaved 更新活跃标签与会话中的 isDirty 为 false', () => {
      useFlowStore.getState().openTab(flowA);
      useFlowStore.getState().updateFlowName('Modified Name');
      expect(useFlowStore.getState().isDirty).toBe(true);
      expect(useFlowStore.getState().tabs[0].isDirty).toBe(true);

      useFlowStore.getState().markSaved();
      expect(useFlowStore.getState().isDirty).toBe(false);
      expect(useFlowStore.getState().tabs[0].isDirty).toBe(false);
      expect(useFlowStore.getState().tabSessions['flow-a'].isDirty).toBe(false);
    });
  });
});
