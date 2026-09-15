import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FlowConflictDialog } from '@/components/dialogs/FlowConflictDialog';
import { useUIStore, useFlowStore } from '@/stores';
import { flowStorage } from '@/services/storage';
import type { Flow, NodeData } from '@/types';

// Mock storage
vi.mock('@/services/storage', () => ({
  flowStorage: {
    saveFlow: vi.fn().mockResolvedValue(undefined),
    listFlows: vi.fn().mockResolvedValue([]),
  },
}));

// Mock toast
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

describe('FlowConflictDialog', () => {
  const existingFlow: Flow = {
    uuid: 'flow-existing',
    name: 'Existing Flow',
    nodes: [
      {
        uuid: 'node-same',
        type: 'http',
        name: 'Node 1',
        position: { x: 0, y: 0 },
        data: { type: 'http', url: 'https://example.com' } as unknown as NodeData,
      },
    ],
    connections: [],
  };

  const incomingFlow: Flow = {
    uuid: 'flow-incoming',
    name: 'Incoming Flow',
    nodes: [
      {
        uuid: 'node-same',
        type: 'http',
        name: 'Node 1 Duplicate',
        position: { x: 10, y: 10 },
        data: { type: 'http', url: 'https://example.com' } as unknown as NodeData,
      },
    ],
    connections: [],
  };

  beforeEach(() => {
    vi.clearAllMocks();
    useUIStore.getState().resetPanels();
    useFlowStore.getState().reset();
    useFlowStore.getState().openTab(existingFlow, '/path/existing.mpf');
  });

  it('没有冲突状态时不渲染任何内容', () => {
    const { container } = render(<FlowConflictDialog />);
    expect(container.firstChild).toBeNull();
  });

  it('展示冲突详情与重映射选项', () => {
    useUIStore.getState().openFlowConflictDialog({
      incomingFlow,
      incomingFilePath: '/path/incoming.mpf',
      conflictingTabUuid: 'flow-existing',
      conflictingTabName: 'Existing Flow',
      conflictType: 'node_uuid',
      duplicateNodeCount: 1,
    });

    render(<FlowConflictDialog />);

    expect(screen.getByText('检测到流程标识冲突')).toBeDefined();
    expect(screen.getByText('重构并分配全新唯一标识（推荐）')).toBeDefined();
    expect(screen.getByText('强制原样打开')).toBeDefined();
  });

  it('点击重映射选项后，保存并打开拥有全新 UUID 的流程', async () => {
    const user = userEvent.setup();
    useUIStore.getState().openFlowConflictDialog({
      incomingFlow,
      incomingFilePath: '/path/incoming.mpf',
      conflictingTabUuid: 'flow-existing',
      conflictingTabName: 'Existing Flow',
      conflictType: 'node_uuid',
      duplicateNodeCount: 1,
    });

    render(<FlowConflictDialog />);

    const remapBtn = screen.getByText('重构并分配全新唯一标识（推荐）');
    await user.click(remapBtn);

    await waitFor(() => {
      expect(flowStorage.saveFlow).toHaveBeenCalledWith(
        expect.objectContaining({ name: incomingFlow.name }),
        '/path/incoming.mpf',
      );
      const currentTabs = useFlowStore.getState().tabs;
      expect(currentTabs).toHaveLength(2);
      // 新打开的 tab 拥有新的 uuid
      const newTab = currentTabs.find((t) => t.uuid !== 'flow-existing');
      expect(newTab).toBeDefined();
      expect(newTab?.uuid).not.toBe('flow-incoming');

      // 弹窗已关闭
      expect(useUIStore.getState().flowConflict).toBeNull();
    });
  });

  it('同 flow_uuid 冲突时点击定位到已有流程直接切换标签', async () => {
    const user = userEvent.setup();
    useUIStore.getState().openFlowConflictDialog({
      incomingFlow: { ...incomingFlow, uuid: 'flow-existing' },
      incomingFilePath: '/path/duplicate-file.mpf',
      conflictingTabUuid: 'flow-existing',
      conflictingTabName: 'Existing Flow',
      conflictType: 'flow_uuid',
    });

    render(<FlowConflictDialog />);

    const switchBtn = screen.getByText('直接切换到已打开的标签');
    await user.click(switchBtn);

    expect(useFlowStore.getState().activeTabUuid).toBe('flow-existing');
    expect(useUIStore.getState().flowConflict).toBeNull();
  });

  it('点击强制原样打开后，按原样将流程打开到工作区', async () => {
    const user = userEvent.setup();
    useUIStore.getState().openFlowConflictDialog({
      incomingFlow,
      incomingFilePath: '/path/incoming.mpf',
      conflictingTabUuid: 'flow-existing',
      conflictingTabName: 'Existing Flow',
      conflictType: 'node_uuid',
      duplicateNodeCount: 1,
    });

    render(<FlowConflictDialog />);

    const forceBtn = screen.getByText('强制原样打开');
    await user.click(forceBtn);

    const currentTabs = useFlowStore.getState().tabs;
    expect(currentTabs).toHaveLength(2);
    expect(useUIStore.getState().flowConflict).toBeNull();
  });
});
