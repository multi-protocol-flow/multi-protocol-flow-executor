/**
 * restoreFlowFromSnapshot 单元测试
 */
import { describe, it, expect, vi } from 'vitest';
import { restoreFlowFromSnapshot } from '@/lib/restoreFlow';
import type { Flow } from '@/types';
import type { RestoreDeps } from '@/lib/restoreFlow';

/** 创建测试用快照 */
function createSnapshot(overrides?: Partial<Flow>): Flow {
  return {
    uuid: 'test-uuid-001',
    name: '测试流程',
    nodes: [],
    connections: [],
    ...overrides,
  };
}

/** 创建默认 mock 依赖 */
function createMockDeps(overrides?: Partial<RestoreDeps>): RestoreDeps {
  return {
    isDirty: false,
    showConfirm: vi.fn().mockResolvedValue(true),
    listFlows: vi.fn().mockResolvedValue([]),
    saveFlow: vi.fn().mockResolvedValue(undefined),
    setFlow: vi.fn(),
    generateUuid: vi.fn().mockReturnValue('new-uuid-123'),
    toast: {
      success: vi.fn(),
      error: vi.fn(),
    },
    ...overrides,
  };
}

describe('restoreFlowFromSnapshot', () => {
  it('直接恢复（无脏画布、无冲突）', async () => {
    const snapshot = createSnapshot();
    const deps = createMockDeps();

    const result = await restoreFlowFromSnapshot(snapshot, deps);

    expect(result).toEqual({
      success: true,
      action: 'restored',
      flow: snapshot,
    });
    expect(deps.saveFlow).toHaveBeenCalledWith(snapshot);
    expect(deps.setFlow).toHaveBeenCalledWith(snapshot);
    expect(deps.toast.success).toHaveBeenCalledWith('流程已恢复');
  });

  it('脏画布 — 用户取消', async () => {
    const snapshot = createSnapshot();
    const deps = createMockDeps({
      isDirty: true,
      showConfirm: vi.fn().mockResolvedValue(false),
    });

    const result = await restoreFlowFromSnapshot(snapshot, deps);

    expect(result).toEqual({
      success: false,
      reason: 'dirty_cancelled',
    });
    expect(deps.saveFlow).not.toHaveBeenCalled();
    expect(deps.setFlow).not.toHaveBeenCalled();
    expect(deps.toast.error).toHaveBeenCalledWith('已取消恢复');
  });

  it('脏画布 — 用户确认后继续恢复', async () => {
    const snapshot = createSnapshot();
    const deps = createMockDeps({
      isDirty: true,
      showConfirm: vi.fn().mockResolvedValue(true),
    });

    const result = await restoreFlowFromSnapshot(snapshot, deps);

    expect(result).toEqual({
      success: true,
      action: 'restored',
      flow: snapshot,
    });
    // 第一调用是脏画布确认
    expect(deps.showConfirm).toHaveBeenCalledTimes(1);
    expect(deps.saveFlow).toHaveBeenCalledWith(snapshot);
    expect(deps.setFlow).toHaveBeenCalledWith(snapshot);
  });

  it('UUID 冲突 — 用户选择覆盖', async () => {
    const snapshot = createSnapshot();
    const deps = createMockDeps({
      listFlows: vi.fn().mockResolvedValue([{ uuid: 'test-uuid-001' }]),
      showConfirm: vi.fn().mockResolvedValue(true), // 覆盖
    });

    const result = await restoreFlowFromSnapshot(snapshot, deps);

    expect(result).toEqual({
      success: true,
      action: 'overwritten',
      flow: snapshot,
    });
    expect(deps.saveFlow).toHaveBeenCalledWith(snapshot);
    expect(deps.setFlow).toHaveBeenCalledWith(snapshot);
    expect(deps.toast.success).toHaveBeenCalledWith('流程已覆盖恢复');
  });

  it('UUID 冲突 — 用户选择新建副本', async () => {
    const snapshot = createSnapshot();
    const deps = createMockDeps({
      listFlows: vi.fn().mockResolvedValue([{ uuid: 'test-uuid-001' }]),
      showConfirm: vi.fn().mockResolvedValue(false), // 新建副本
      generateUuid: vi.fn().mockReturnValue('new-copy-uuid'),
    });

    const result = await restoreFlowFromSnapshot(snapshot, deps);

    const expectedFlow: Flow = {
      ...snapshot,
      uuid: 'new-copy-uuid',
      name: '测试流程(恢复副本)',
    };

    expect(result).toEqual({
      success: true,
      action: 'created_copy',
      flow: expectedFlow,
    });
    expect(deps.saveFlow).toHaveBeenCalledWith(expectedFlow);
    expect(deps.setFlow).toHaveBeenCalledWith(expectedFlow);
    expect(deps.toast.success).toHaveBeenCalledWith('已创建恢复副本');
  });

  it('UUID 冲突 — 新建副本保留节点和连接', async () => {
    const snapshot = createSnapshot({
      nodes: [
        { uuid: 'node-1', type: 'entry', name: '入口', position: { x: 0, y: 0 }, data: { type: 'entry' as const } },
      ],
      connections: [
        { id: 'conn-1', source_node_uuid: 'node-1', target_node_uuid: 'node-2', source_port_id: 'output', target_port_id: 'input' },
      ],
    });
    const deps = createMockDeps({
      listFlows: vi.fn().mockResolvedValue([{ uuid: 'test-uuid-001' }]),
      showConfirm: vi.fn().mockResolvedValue(false),
      generateUuid: vi.fn().mockReturnValue('new-copy-uuid'),
    });

    const result = await restoreFlowFromSnapshot(snapshot, deps);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.action).toBe('created_copy');
      expect(result.flow.uuid).toBe('new-copy-uuid');
      expect(result.flow.name).toBe('测试流程(恢复副本)');
      // 节点和连接完整保留
      expect(result.flow.nodes).toEqual(snapshot.nodes);
      expect(result.flow.connections).toEqual(snapshot.connections);
    }
  });

  it('保存失败时返回 load_failed', async () => {
    const snapshot = createSnapshot();
    const deps = createMockDeps({
      saveFlow: vi.fn().mockRejectedValue(new Error('磁盘写入失败')),
    });

    const result = await restoreFlowFromSnapshot(snapshot, deps);

    expect(result).toEqual({
      success: false,
      reason: 'load_failed',
      error: '磁盘写入失败',
    });
    expect(deps.setFlow).not.toHaveBeenCalled();
    expect(deps.toast.error).toHaveBeenCalledWith('恢复失败: 磁盘写入失败');
  });

  it('保存失败（非 Error 对象）时正确提取消息', async () => {
    const snapshot = createSnapshot();
    const deps = createMockDeps({
      saveFlow: vi.fn().mockRejectedValue('未知错误'),
    });

    const result = await restoreFlowFromSnapshot(snapshot, deps);

    expect(result).toEqual({
      success: false,
      reason: 'load_failed',
      error: '未知错误',
    });
    expect(deps.toast.error).toHaveBeenCalledWith('恢复失败: 未知错误');
  });

  it('UUID 冲突覆盖时保存失败', async () => {
    const snapshot = createSnapshot();
    const deps = createMockDeps({
      listFlows: vi.fn().mockResolvedValue([{ uuid: 'test-uuid-001' }]),
      showConfirm: vi.fn().mockResolvedValue(true),
      saveFlow: vi.fn().mockRejectedValue(new Error('权限不足')),
    });

    const result = await restoreFlowFromSnapshot(snapshot, deps);

    expect(result).toEqual({
      success: false,
      reason: 'load_failed',
      error: '权限不足',
    });
    expect(deps.toast.error).toHaveBeenCalledWith('恢复失败: 权限不足');
  });

  it('UUID 冲突新建副本时保存失败', async () => {
    const snapshot = createSnapshot();
    const deps = createMockDeps({
      listFlows: vi.fn().mockResolvedValue([{ uuid: 'test-uuid-001' }]),
      showConfirm: vi.fn().mockResolvedValue(false),
      saveFlow: vi.fn().mockRejectedValue(new Error('空间不足')),
    });

    const result = await restoreFlowFromSnapshot(snapshot, deps);

    expect(result).toEqual({
      success: false,
      reason: 'load_failed',
      error: '空间不足',
    });
    expect(deps.toast.error).toHaveBeenCalledWith('恢复失败: 空间不足');
  });

  it('脏画布确认后遇到 UUID 冲突，完整流程', async () => {
    const snapshot = createSnapshot();
    let confirmCallCount = 0;
    const deps = createMockDeps({
      isDirty: true,
      listFlows: vi.fn().mockResolvedValue([{ uuid: 'test-uuid-001' }]),
      showConfirm: vi.fn().mockImplementation(() => {
        confirmCallCount++;
        // 第一次（脏画布）确认，第二次（冲突）选择新建副本
        return Promise.resolve(confirmCallCount === 1);
      }),
      generateUuid: vi.fn().mockReturnValue('new-uuid-456'),
    });

    const result = await restoreFlowFromSnapshot(snapshot, deps);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.action).toBe('created_copy');
      expect(result.flow.uuid).toBe('new-uuid-456');
      expect(result.flow.name).toBe('测试流程(恢复副本)');
    }
    expect(deps.showConfirm).toHaveBeenCalledTimes(2);
  });

  it('无 UUID 冲突时跳过冲突检查直接恢复', async () => {
    const snapshot = createSnapshot();
    const deps = createMockDeps({
      listFlows: vi.fn().mockResolvedValue([{ uuid: 'other-uuid' }]),
    });

    const result = await restoreFlowFromSnapshot(snapshot, deps);

    expect(result).toEqual({
      success: true,
      action: 'restored',
      flow: snapshot,
    });
    // showConfirm 未被调用（无脏画布、无冲突）
    expect(deps.showConfirm).not.toHaveBeenCalled();
  });
});
