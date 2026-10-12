import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { ReactFlow, ReactFlowProvider } from '@xyflow/react';
import { NodeContextMenu } from '@/components/canvas/NodeContextMenu';
import { useFlowStore, useUIStore, useExecutionStore } from '@/stores';
import * as tauriService from '@/services/tauri';
import type { Flow } from '@/types/flow';

vi.mock('@/services/tauri', async () => {
  const actual = await vi.importActual<typeof import('@/services/tauri')>('@/services/tauri');
  return {
    ...actual,
    startInteractiveSliceExecution: vi.fn(),
  };
});

describe('NodeContextMenu', () => {
  const sampleFlow: Flow = {
    uuid: 'flow-test',
    name: 'Test Flow',
    nodes: [
      { uuid: 'node-1', type: 'http:request', name: 'HTTP 1', position: { x: 0, y: 0 }, data: { type: 'http:request', url: 'https://example.com' } },
      { uuid: 'node-2', type: 'script', name: 'Script 2', position: { x: 100, y: 0 }, data: { type: 'script', script: 'return 1;' } },
    ],
    connections: [],
    initial_variables: {},
  };

  beforeEach(() => {
    vi.clearAllMocks();
    useFlowStore.getState().reset();
    useUIStore.getState().resetPanels();
    useExecutionStore.getState().reset();
    useFlowStore.setState({ flow: sampleFlow });
  });

  it('渲染单节点右键菜单项并触发 startInteractiveSliceExecution', async () => {
    const onClose = vi.fn();

    render(
      <ReactFlowProvider>
        <NodeContextMenu
          x={100}
          y={100}
          nodeId="node-1"
          nodeType="http:request"
          nodeName="HTTP 1"
          onClose={onClose}
          onCopy={vi.fn()}
          onDelete={vi.fn()}
          onRename={vi.fn()}
          onCopyConfig={vi.fn()}
          onPasteConfig={vi.fn()}
          hasClipboardConfig={false}
        />
      </ReactFlowProvider>
    );

    const testBtn = screen.getByText('测试此节点');
    expect(testBtn).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(testBtn);
    });

    expect(onClose).toHaveBeenCalled();
    expect(tauriService.startInteractiveSliceExecution).toHaveBeenCalledWith(sampleFlow, ['node-1']);
  });

  it('当多选节点时展示运行所选节点并触发切片执行', async () => {
    const onClose = vi.fn();
    const initialNodes = [
      { id: 'node-1', selected: true, position: { x: 0, y: 0 }, data: {} },
      { id: 'node-2', selected: true, position: { x: 100, y: 0 }, data: {} },
    ];

    render(
      <ReactFlowProvider>
        <ReactFlow defaultNodes={initialNodes}>
          <NodeContextMenu
            x={100}
            y={100}
            nodeId="node-1"
            nodeType="http:request"
            nodeName="HTTP 1"
            onClose={onClose}
            onCopy={vi.fn()}
            onDelete={vi.fn()}
            onRename={vi.fn()}
            onCopyConfig={vi.fn()}
            onPasteConfig={vi.fn()}
            hasClipboardConfig={false}
          />
        </ReactFlow>
      </ReactFlowProvider>
    );

    const runSelectedBtn = screen.getByText(/运行所选节点/);
    expect(runSelectedBtn).toBeInTheDocument();
    expect(runSelectedBtn.textContent).toContain('2');

    await act(async () => {
      fireEvent.click(runSelectedBtn);
    });

    expect(onClose).toHaveBeenCalled();
    expect(tauriService.startInteractiveSliceExecution).toHaveBeenCalledWith(sampleFlow, ['node-1', 'node-2']);
  });

  it('当点击菜单外部区域或按下 Escape 键时触发 onClose 关闭菜单', async () => {
    const onClose = vi.fn();

    render(
      <ReactFlowProvider>
        <div data-testid="outside-area">空白区域</div>
        <NodeContextMenu
          x={100}
          y={100}
          nodeId="node-1"
          nodeType="http:request"
          nodeName="HTTP 1"
          onClose={onClose}
          onCopy={vi.fn()}
          onDelete={vi.fn()}
          onRename={vi.fn()}
          onCopyConfig={vi.fn()}
          onPasteConfig={vi.fn()}
          hasClipboardConfig={false}
        />
      </ReactFlowProvider>
    );

    const outsideArea = screen.getByTestId('outside-area');
    await vi.waitFor(() => {
      fireEvent.pointerDown(outsideArea);
      expect(onClose).toHaveBeenCalledTimes(1);
    });

    await act(async () => {
      fireEvent.keyDown(document, { key: 'Escape' });
    });
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});
