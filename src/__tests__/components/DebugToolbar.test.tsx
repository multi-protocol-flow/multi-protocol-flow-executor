/**
 * DebugToolbar 组件测试
 * 测试跳过节点功能的端口识别和对话框显示逻辑
 */
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { DebugToolbar } from '@/components/canvas/DebugToolbar';

// Mock stores - 需要模拟 @/stores 模块
vi.mock('@/stores', () => ({
  useDebugStore: vi.fn(),
  useFlowStore: vi.fn(),
  selectIsPaused: (state: { status: string }) => state.status === 'paused',
  selectCurrentNodeUuid: (state: { currentNodeUuid: string | null }) => state.currentNodeUuid,
}));

// 从 mock 中获取引用
import { useDebugStore, useFlowStore } from '@/stores';

// Mock SkipNodeDialog 组件
vi.mock('@/components/dialogs/SkipNodeDialog', () => ({
  SkipNodeDialog: vi.fn(({ open, nodeType, outputPorts, onConfirm, onCancel }) => {
    if (!open) return null;
    return (
      <div data-testid="skip-dialog" data-node-type={nodeType}>
        <span data-testid="port-count">{outputPorts.length}</span>
        {outputPorts.map((p: { id: string; label: string }) => (
          <span key={p.id} data-testid={`port-${p.id}`}>{p.label}</span>
        ))}
        <button onClick={() => onConfirm('true')} data-testid="confirm-btn">确认</button>
        <button onClick={onCancel} data-testid="cancel-btn">取消</button>
      </div>
    );
  }),
}));

describe('DebugToolbar', () => {
  const mockSkipNode = vi.fn();
  const mockContinueExecution = vi.fn();
  const mockStepOver = vi.fn();
  const mockRunToEnd = vi.fn();
  const mockEndDebugSession = vi.fn();

  const createMockFlow = (nodeType: string, nodeUuid: string) => ({
    uuid: 'flow-1',
    name: '测试流程',
    nodes: [
      {
        uuid: nodeUuid,
        type: nodeType,
        name: `${nodeType} 节点`,
        position: { x: 0, y: 0 },
        data: { type: nodeType },
      },
    ],
    connections: [],
  });

  // 辅助函数：设置 mock store
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const mockDebugStore = (status: string, nodeUuid: string) => {
    vi.mocked(useDebugStore).mockImplementation((selector: any) => {
      return selector({
        status,
        currentNodeUuid: nodeUuid,
        skipNode: mockSkipNode,
        continueExecution: mockContinueExecution,
        stepOver: mockStepOver,
        runToEnd: mockRunToEnd,
        endDebugSession: mockEndDebugSession,
      });
    });
  };

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const mockFlowStoreWithNode = (nodeType: string, nodeUuid: string) => {
    const mockFlow = createMockFlow(nodeType, nodeUuid);
    vi.mocked(useFlowStore).mockImplementation((selector: any) => {
      return selector({ flow: mockFlow });
    });
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockDebugStore('paused', 'node-1');
  });

  describe('跳过节点 - 条件分支节点 (conditional)', () => {
    it('应该显示对话框，并有 true/false 两个端口选项', () => {
      mockFlowStoreWithNode('conditional', 'node-1');
      render(<DebugToolbar />);

      const skipButton = screen.getByTitle('跳过当前节点');
      fireEvent.click(skipButton);

      expect(screen.getByTestId('skip-dialog')).toBeInTheDocument();
      expect(screen.getByTestId('skip-dialog')).toHaveAttribute('data-node-type', 'conditional');
      expect(screen.getByTestId('port-count')).toHaveTextContent('2');
      expect(screen.getByTestId('port-true')).toHaveTextContent('True Branch');
      expect(screen.getByTestId('port-false')).toHaveTextContent('False Branch');
    });
  });

  describe('跳过节点 - WebSocket 连接节点 (ws_connect)', () => {
    it('应该显示对话框，并有 true/false 两个端口选项', () => {
      mockFlowStoreWithNode('ws_connect', 'node-1');
      render(<DebugToolbar />);

      const skipButton = screen.getByTitle('跳过当前节点');
      fireEvent.click(skipButton);

      expect(screen.getByTestId('skip-dialog')).toBeInTheDocument();
      expect(screen.getByTestId('skip-dialog')).toHaveAttribute('data-node-type', 'ws_connect');
      expect(screen.getByTestId('port-count')).toHaveTextContent('2');
      expect(screen.getByTestId('port-true')).toBeInTheDocument();
      expect(screen.getByTestId('port-false')).toBeInTheDocument();
    });
  });

  describe('跳过节点 - WebSocket 发送等待节点 (ws_send_collect)', () => {
    it('应该显示对话框，并有 true/false 两个端口选项', () => {
      mockFlowStoreWithNode('ws_send_collect', 'node-1');
      render(<DebugToolbar />);

      const skipButton = screen.getByTitle('跳过当前节点');
      fireEvent.click(skipButton);

      expect(screen.getByTestId('skip-dialog')).toBeInTheDocument();
      expect(screen.getByTestId('skip-dialog')).toHaveAttribute('data-node-type', 'ws_send_collect');
      expect(screen.getByTestId('port-count')).toHaveTextContent('2');
      expect(screen.getByTestId('port-true')).toBeInTheDocument();
      expect(screen.getByTestId('port-false')).toBeInTheDocument();
    });
  });

  describe('跳过节点 - WebSocket 关闭节点 (ws_close)', () => {
    it('应该显示对话框，并有 true/false 两个端口选项', () => {
      mockFlowStoreWithNode('ws_close', 'node-1');
      render(<DebugToolbar />);

      const skipButton = screen.getByTitle('跳过当前节点');
      fireEvent.click(skipButton);

      expect(screen.getByTestId('skip-dialog')).toBeInTheDocument();
      expect(screen.getByTestId('skip-dialog')).toHaveAttribute('data-node-type', 'ws_close');
      expect(screen.getByTestId('port-count')).toHaveTextContent('2');
      expect(screen.getByTestId('port-true')).toBeInTheDocument();
      expect(screen.getByTestId('port-false')).toBeInTheDocument();
    });
  });

  describe('跳过节点 - HTTP 节点 (http)', () => {
    it('应该直接跳过，不显示对话框（只有一个 out 端口）', () => {
      mockFlowStoreWithNode('http', 'node-1');
      render(<DebugToolbar />);

      const skipButton = screen.getByTitle('跳过当前节点');
      fireEvent.click(skipButton);

      // HTTP 只有一个端口，直接跳过，不显示对话框
      expect(mockSkipNode).toHaveBeenCalledWith('out');
      expect(screen.queryByTestId('skip-dialog')).not.toBeInTheDocument();
    });
  });

  describe('跳过节点 - Entry 节点 (entry)', () => {
    it('应该直接跳过，不显示对话框（只有一个 out 端口）', () => {
      mockFlowStoreWithNode('entry', 'node-1');
      render(<DebugToolbar />);

      const skipButton = screen.getByTitle('跳过当前节点');
      fireEvent.click(skipButton);

      expect(mockSkipNode).toHaveBeenCalledWith('out');
      expect(screen.queryByTestId('skip-dialog')).not.toBeInTheDocument();
    });
  });

  describe('跳过节点 - 对话框确认', () => {
    it('点击确认后应该调用 skipNode 并关闭对话框', () => {
      mockFlowStoreWithNode('conditional', 'node-1');
      render(<DebugToolbar />);

      const skipButton = screen.getByTitle('跳过当前节点');
      fireEvent.click(skipButton);

      expect(screen.getByTestId('skip-dialog')).toBeInTheDocument();

      const confirmButton = screen.getByTestId('confirm-btn');
      fireEvent.click(confirmButton);

      // skipNode 被调用，第二个参数 simulatedOutput 是 undefined
      expect(mockSkipNode).toHaveBeenCalledWith('true', undefined);
    });
  });

  describe('按钮禁用状态', () => {
    it('非暂停状态下跳过按钮应该禁用', () => {
      mockDebugStore('running', 'node-1');
      mockFlowStoreWithNode('conditional', 'node-1');
      render(<DebugToolbar />);

      const skipButton = screen.getByTitle('跳过（仅在暂停时可用）');
      expect(skipButton).toBeDisabled();
    });
  });
});
