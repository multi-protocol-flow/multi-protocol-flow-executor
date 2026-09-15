import { describe, it, expect } from 'vitest';
import { checkFlowConflict } from '@/lib/flowConflict';
import type { Flow, FlowTab, FlowTabSession, NodeData } from '@/types';

describe('checkFlowConflict', () => {
  const createFlow = (uuid: string, nodeUuids: string[]): Flow => ({
    uuid,
    name: `Flow ${uuid}`,
    nodes: nodeUuids.map((id) => ({
      uuid: id,
      type: 'http',
      name: `Node ${id}`,
      position: { x: 0, y: 0 },
      data: { type: 'http', url: 'https://example.com' } as unknown as NodeData,
    })),
    connections: [],
  });

  it('全新且互不相同的流程无冲突', () => {
    const existingFlow = createFlow('flow-1', ['node-1', 'node-2']);
    const tabs: FlowTab[] = [
      { uuid: 'flow-1', name: 'Flow 1', isDirty: false, filePath: '/path/1.mpf' },
    ];
    const sessions: Record<string, FlowTabSession> = {
      'flow-1': {
        flow: existingFlow,
        selectedNodeId: null,
        isDirty: false,
        past: [],
        future: [],
        historyVersion: 0,
      },
    };

    const incomingFlow = createFlow('flow-2', ['node-3', 'node-4']);
    const conflict = checkFlowConflict(incomingFlow, '/path/2.mpf', tabs, sessions);

    expect(conflict).toBeNull();
  });

  it('同一文件路径被重复打开不触发冲突（由调用方直接切换）', () => {
    const existingFlow = createFlow('flow-1', ['node-1']);
    const tabs: FlowTab[] = [
      { uuid: 'flow-1', name: 'Flow 1', isDirty: false, filePath: '/path/1.mpf' },
    ];
    const sessions: Record<string, FlowTabSession> = {
      'flow-1': {
        flow: existingFlow,
        selectedNodeId: null,
        isDirty: false,
        past: [],
        future: [],
        historyVersion: 0,
      },
    };

    const conflict = checkFlowConflict(existingFlow, '/path/1.mpf', tabs, sessions);
    expect(conflict).toBeNull();
  });

  it('同 Flow UUID 但不同文件路径时触发 flow_uuid 冲突', () => {
    const existingFlow = createFlow('flow-1', ['node-1']);
    const tabs: FlowTab[] = [
      { uuid: 'flow-1', name: 'Flow 1', isDirty: false, filePath: '/path/flow-a.mpf' },
    ];
    const sessions: Record<string, FlowTabSession> = {
      'flow-1': {
        flow: existingFlow,
        selectedNodeId: null,
        isDirty: false,
        past: [],
        future: [],
        historyVersion: 0,
      },
    };

    // 文件系统外部复制出来的 flow-b.mpf 依然是 flow-1
    const incomingFlow = createFlow('flow-1', ['node-diff']);
    const conflict = checkFlowConflict(incomingFlow, '/path/flow-b.mpf', tabs, sessions);

    expect(conflict).not.toBeNull();
    expect(conflict?.conflictType).toBe('flow_uuid');
    expect(conflict?.conflictingTabName).toBe('Flow 1');
  });

  it('不同 Flow UUID 但存在重复 Node UUID 时触发 node_uuid 冲突', () => {
    const existingFlow = createFlow('flow-1', ['node-1', 'node-2']);
    const tabs: FlowTab[] = [
      { uuid: 'flow-1', name: 'Flow 1', isDirty: false, filePath: '/path/flow-1.mpf' },
    ];
    const sessions: Record<string, FlowTabSession> = {
      'flow-1': {
        flow: existingFlow,
        selectedNodeId: null,
        isDirty: false,
        past: [],
        future: [],
        historyVersion: 0,
      },
    };

    const incomingFlow = createFlow('flow-2', ['node-2', 'node-3']);
    const conflict = checkFlowConflict(incomingFlow, '/path/flow-2.mpf', tabs, sessions);

    expect(conflict).not.toBeNull();
    expect(conflict?.conflictType).toBe('node_uuid');
    expect(conflict?.duplicateNodeCount).toBe(1);
    expect(conflict?.conflictingTabName).toBe('Flow 1');
  });
});
