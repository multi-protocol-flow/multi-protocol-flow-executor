import { describe, it, expect, beforeEach } from 'vitest';
import { useFlowStore } from '@/stores/flow';
import type { Flow, NodeData } from '@/types';

describe('多标签与跨流程节点隔离 (Cross-Flow Isolation)', () => {
  const flowA: Flow = {
    uuid: 'flow-a',
    name: 'Flow A',
    nodes: [
      {
        uuid: 'node-same',
        type: 'http',
        name: 'Node Same A',
        position: { x: 0, y: 0 },
        data: { type: 'http', url: 'https://a.com' } as unknown as NodeData,
      },
    ],
    connections: [],
  };

  const flowB: Flow = {
    uuid: 'flow-b',
    name: 'Flow B',
    nodes: [
      {
        uuid: 'node-same',
        type: 'http',
        name: 'Node Same B',
        position: { x: 100, y: 100 },
        data: { type: 'http', url: 'https://b.com' } as unknown as NodeData,
      },
    ],
    connections: [],
  };

  beforeEach(() => {
    useFlowStore.getState().reset();
  });

  it('标签切换时独立记住各自选中的节点 ID，未选中时保持 null', () => {
    // 强制打开两个具有相同节点 UUID 的流程（模拟脏数据共存场景）
    useFlowStore.getState().openTab(flowA, '/path/a.mpf', { force: true });
    useFlowStore.getState().openTab(flowB, '/path/b.mpf', { force: true });

    // 当前在 Tab B，默认 selectedNodeId 为 null
    expect(useFlowStore.getState().activeTabUuid).toBe('flow-b');
    expect(useFlowStore.getState().selectedNodeId).toBeNull();

    // 在 Tab B 中不选中任何节点
    // 切换到 Tab A 并选中 node-same
    useFlowStore.getState().switchTab('flow-a');
    expect(useFlowStore.getState().activeTabUuid).toBe('flow-a');
    useFlowStore.getState().selectNode('node-same');
    expect(useFlowStore.getState().selectedNodeId).toBe('node-same');

    // 切换回 Tab B：应恢复 Tab B 的选中状态（null）
    useFlowStore.getState().switchTab('flow-b');
    expect(useFlowStore.getState().activeTabUuid).toBe('flow-b');
    expect(useFlowStore.getState().selectedNodeId).toBeNull();

    // 再次切回 Tab A：应精确恢复 Tab A 选中的 node-same
    useFlowStore.getState().switchTab('flow-a');
    expect(useFlowStore.getState().activeTabUuid).toBe('flow-a');
    expect(useFlowStore.getState().selectedNodeId).toBe('node-same');
  });

  it('当节点所属 flow 不是当前活跃 flow 时，定向更新源 session，绝不串写活跃 flow', () => {
    useFlowStore.getState().openTab(flowA, '/path/a.mpf', { force: true });
    useFlowStore.getState().openTab(flowB, '/path/b.mpf', { force: true });

    // 当前活跃流是 Flow B
    expect(useFlowStore.getState().activeTabUuid).toBe('flow-b');
    const flowBBefore = useFlowStore.getState().flow;
    const flowBNodeData = flowBBefore?.nodes.find((n) => n.uuid === 'node-same')?.data as unknown as Record<string, unknown>;
    expect(flowBNodeData.url).toBe('https://b.com');

    // 模拟来自 Flow A 的异步或卸载写入（renderFlowUuid = 'flow-a'）
    // 模拟 NodeConfigPanel 的安全隔离写入逻辑
    const renderFlowUuid = 'flow-a';
    const renderNodeUuid = 'node-same';
    const newDirtyData = { type: 'http', url: 'https://modified-in-a.com' } as unknown as NodeData;

    const { flow: activeFlow, tabSessions } = useFlowStore.getState();
    // activeFlow 是 flow-b，不等于 renderFlowUuid
    expect(activeFlow?.uuid).not.toBe(renderFlowUuid);

    // 触发隔离机制定向写入 tabSessions['flow-a']
    if (renderFlowUuid && tabSessions[renderFlowUuid]) {
      const targetSession = tabSessions[renderFlowUuid];
      const targetFlow = targetSession.flow;
      const updatedNodes = targetFlow.nodes.map((n) =>
        n.uuid === renderNodeUuid ? { ...n, data: newDirtyData } : n
      );
      useFlowStore.setState((state) => ({
        tabSessions: {
          ...state.tabSessions,
          [renderFlowUuid]: {
            ...state.tabSessions[renderFlowUuid],
            flow: { ...targetFlow, nodes: updatedNodes },
            isDirty: true,
          },
        },
      }));
    }

    // 验证：Flow B 的 node-same 数据完全未被污染！
    const flowBAfter = useFlowStore.getState().flow;
    const flowBNodeDataAfter = flowBAfter?.nodes.find((n) => n.uuid === 'node-same')?.data as unknown as Record<string, unknown>;
    expect(flowBNodeDataAfter.url).toBe('https://b.com');

    // 切换到 Flow A：Flow A 的 node-same 正确得到了修改
    useFlowStore.getState().switchTab('flow-a');
    const flowAAfter = useFlowStore.getState().flow;
    const flowANodeDataAfter = flowAAfter?.nodes.find((n) => n.uuid === 'node-same')?.data as unknown as Record<string, unknown>;
    expect(flowANodeDataAfter.url).toBe('https://modified-in-a.com');
  });
});
