/**
 * CanvasToolbar 组件测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { CanvasToolbar } from '@/components/canvas/CanvasToolbar';
import { useFlowStore, useExecutionStore, useUIStore, useDebugStore, useStressTestStore } from '@/stores';
import { useConsoleStore } from '@/stores/console';
import { useSettingsStore } from '@/stores/settings';
import { useWsStreamStore } from '@/stores/wsStream';

// Mock stores
vi.mock('@/stores', () => ({
  useFlowStore: vi.fn(),
  useExecutionStore: vi.fn(),
  useUIStore: vi.fn(),
  useDebugStore: vi.fn(),
  useStressTestStore: vi.fn(),
  selectFlow: vi.fn((state) => state.flow),
  selectNodes: vi.fn((state) => state.nodes ?? []),
  selectIsDebugging: vi.fn((state) => state.status !== 'idle' && state.status !== 'ended'),
  selectCanUndo: vi.fn((state) => state.past?.length > 0),
  selectCanRedo: vi.fn((state) => state.future?.length > 0),
}));

// Mock console store
vi.mock('@/stores/console', () => ({
  useConsoleStore: vi.fn(),
}));

// Mock settings store
vi.mock('@/stores/settings', () => ({
  useSettingsStore: vi.fn(),
}));

// Mock wsStream store
vi.mock('@/stores/wsStream', () => ({
  useWsStreamStore: vi.fn(),
}));

// Mock tauri service
vi.mock('@/services/tauri', () => ({
  executeFlowWithChannel: vi.fn(),
  saveFlowReport: vi.fn(),
}));

describe('CanvasToolbar', () => {
  const mockSetReport = vi.fn();
  const mockSetError = vi.fn();
  const mockSetConfigPanelOpen = vi.fn();
  const mockClearLogs = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    // Default mocks for disabled state (no flow)
    (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: { flow: null; nodes: never[]; undo: ReturnType<typeof vi.fn>; redo: ReturnType<typeof vi.fn>; past: never[]; future: never[] }) => unknown) => selector({ flow: null, nodes: [], undo: vi.fn(), redo: vi.fn(), past: [], future: [] })
    );
    const mockExecutionState = {
      setReport: mockSetReport,
      setError: mockSetError,
      clearExecutionState: vi.fn(),
      handleExecutionEvent: vi.fn(),
    };
    (useExecutionStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) =>
        selector(mockExecutionState)
    );
    // ExecuteButton 使用 getState() 直接调用
    (useExecutionStore as unknown as Record<string, unknown>).getState = vi.fn(() => mockExecutionState);
    const mockWsStreamState = {
      handleWsEvent: vi.fn(),
    };
    (useWsStreamStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) =>
        selector(mockWsStreamState)
    );
    // ExecuteButton 使用 getState() 直接调用 wsStreamStore
    (useWsStreamStore as unknown as Record<string, unknown>).getState = vi.fn(() => mockWsStreamState);
    const mockUIState = { setConfigPanelOpen: mockSetConfigPanelOpen, setExecutionPanelOpen: vi.fn() };
    (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) =>
        selector(mockUIState)
    );
    // ExecuteButton 使用 getState() 直接调用
    (useUIStore as unknown as Record<string, unknown>).getState = vi.fn(() => mockUIState);
    // ExecuteButton 使用 setState 直接调用
    (useUIStore as unknown as Record<string, unknown>).setState = vi.fn();

    // Mock console store
    (useConsoleStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) =>
        selector({ clearLogs: mockClearLogs, addLog: vi.fn() })
    );

    // Mock settings store
    (useSettingsStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) =>
        selector({ reportDir: '/test/reports' })
    );

    // Mock debug store
    const mockDebugState = {
      status: 'idle',
      currentNodeUuid: null,
      startDebugSession: vi.fn(),
      isDebugging: false,
      clearDebugData: vi.fn(),
    };
    (useDebugStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) =>
        selector(mockDebugState)
    );
    // CanvasToolbar 使用 getState() 直接调用
    (useDebugStore as unknown as Record<string, unknown>).getState = vi.fn(() => mockDebugState);

    // Mock stress test store
    (useStressTestStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) =>
        selector({
          status: 'idle',
          config: null,
          progress: null,
          report: null,
          isLoading: false,
          error: null,
          startStressTest: vi.fn(),
          stopStressTest: vi.fn(),
          reset: vi.fn(),
          handleStressTestEvent: vi.fn(),
        })
    );
  });

  describe('渲染测试', () => {
    it('正确渲染工具栏容器', () => {
      render(<CanvasToolbar />);
      const toolbar = document.querySelector('.rounded-full');
      expect(toolbar).toBeInTheDocument();
    });

    it('正确渲染执行按钮', () => {
      render(<CanvasToolbar />);
      const executeButton = screen.getByRole('button', { name: /执行流程/ });
      expect(executeButton).toBeInTheDocument();
    });

    it('正确渲染调试按钮', () => {
      render(<CanvasToolbar />);
      const debugButton = screen.getByTitle('开始调试');
      expect(debugButton).toBeInTheDocument();
    });

    it('工具栏有胶囊形样式', () => {
      render(<CanvasToolbar />);
      const toolbar = document.querySelector('.rounded-full');
      expect(toolbar).toBeInTheDocument();
      expect(toolbar).toHaveClass('bg-background/90');
      expect(toolbar).toHaveClass('backdrop-blur-sm');
    });

    it('工具栏居中定位', () => {
      render(<CanvasToolbar />);
      const toolbar = document.querySelector('.absolute.top-4');
      expect(toolbar).toHaveClass('left-1/2');
      expect(toolbar).toHaveClass('-translate-x-1/2');
    });

    it('执行按钮在调试按钮左边', () => {
      render(<CanvasToolbar />);
      const toolbar = document.querySelector('.flex.items-center.gap-3');
      expect(toolbar).toBeInTheDocument();

      const children = toolbar?.children;
      // 无流程时：添加节点按钮 + 分隔线 + 撤销 + 重做 + 分隔线 + 执行 + 调试 + 压测 + 数据驱动 = 9
      expect(children).toHaveLength(9);

      // 前两个是添加节点按钮和分隔线
      expect(children?.[0]).toHaveAttribute('title', '添加节点（也可右键画布空白处）');
      expect(children?.[1]).toHaveClass('bg-border');
      // 第三/四个是撤销/重做按钮（i18n 已包含快捷键）
      expect(children?.[2]).toHaveAttribute('title', '撤销 (Ctrl+Z)');
      expect(children?.[3]).toHaveAttribute('title', '重做 (Ctrl+Shift+Z)');
      // 第五个是分隔线
      expect(children?.[4]).toHaveClass('bg-border');
      // 第六个是执行按钮
      expect(children?.[5]).toHaveAttribute('title', '执行流程 (Ctrl+Enter)');
      // 第七个是调试按钮
      expect(children?.[6]).toHaveAttribute('title', '开始调试');
      // 第八个是压测按钮
      expect(children?.[7]).toHaveAttribute('title', '压力测试');
      // 第九个是数据驱动按钮
      expect(children?.[8]).toHaveAttribute('title', '数据驱动');
    });
  });

  describe('执行按钮状态', () => {
    it('当没有流程时按钮被禁用', () => {
      render(<CanvasToolbar />);
      const button = screen.getByRole('button', { name: /执行流程/ });
      expect(button).toBeDisabled();
    });

    it('流程没有入口节点时按钮禁用', () => {
      const mockFlow = {
        uuid: 'test-flow',
        name: '测试流程',
        nodes: [
          {
            uuid: 'http-1',
            type: 'http' as const,
            name: 'HTTP',
            position: { x: 0, y: 0 },
            data: { type: 'http' as const, url: '', method: 'get' as const },
          },
        ],
        connections: [],
        initial_variables: {},
      };

      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: { flow: typeof mockFlow; nodes: typeof mockFlow.nodes; undo: ReturnType<typeof vi.fn>; redo: ReturnType<typeof vi.fn>; past: never[]; future: never[] }) => unknown) => selector({ flow: mockFlow, nodes: mockFlow.nodes, undo: vi.fn(), redo: vi.fn(), past: [], future: [] })
      );

      render(<CanvasToolbar />);
      const button = screen.getByRole('button', { name: /执行流程/ });
      expect(button).toBeDisabled();
    });

    it('当有有效流程和入口节点时按钮可用', () => {
      const mockFlow = {
        uuid: 'test-flow',
        name: '测试流程',
        nodes: [
          {
            uuid: 'entry-1',
            type: 'entry' as const,
            name: '入口',
            position: { x: 0, y: 0 },
            data: { type: 'entry' as const },
          },
        ],
        connections: [],
        initial_variables: {},
      };

      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: { flow: typeof mockFlow; nodes: typeof mockFlow.nodes; undo: ReturnType<typeof vi.fn>; redo: ReturnType<typeof vi.fn>; past: never[]; future: never[] }) => unknown) => selector({ flow: mockFlow, nodes: mockFlow.nodes, undo: vi.fn(), redo: vi.fn(), past: [], future: [] })
      );

      render(<CanvasToolbar />);
      const button = screen.getByRole('button', { name: /执行流程/ });
      expect(button).not.toBeDisabled();
    });

    it('可用状态下有红色样式', () => {
      const mockFlow = {
        uuid: 'test-flow',
        name: '测试流程',
        nodes: [
          {
            uuid: 'entry-1',
            type: 'entry' as const,
            name: '入口',
            position: { x: 0, y: 0 },
            data: { type: 'entry' as const },
          },
        ],
        connections: [],
        initial_variables: {},
      };

      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: { flow: typeof mockFlow; nodes: typeof mockFlow.nodes; undo: ReturnType<typeof vi.fn>; redo: ReturnType<typeof vi.fn>; past: never[]; future: never[] }) => unknown) => selector({ flow: mockFlow, nodes: mockFlow.nodes, undo: vi.fn(), redo: vi.fn(), past: [], future: [] })
      );

      render(<CanvasToolbar />);
      const button = screen.getByRole('button', { name: /执行流程/ });
      expect(button).toHaveClass('text-red-500');
    });

    it('禁用状态下有灰色样式', () => {
      render(<CanvasToolbar />);
      const button = screen.getByRole('button', { name: /执行流程/ });
      expect(button).toHaveClass('text-muted-foreground');
    });
  });

  describe('调试按钮', () => {
    it('调试按钮是 button 元素', () => {
      render(<CanvasToolbar />);
      const debugButton = screen.getByTitle('开始调试');
      expect(debugButton.tagName).toBe('BUTTON');
    });

    it('调试按钮有灰色样式（无流程时）', () => {
      render(<CanvasToolbar />);
      const debugButton = screen.getByTitle('开始调试');
      expect(debugButton).toHaveClass('text-muted-foreground');
    });

    it('调试按钮有 cursor-not-allowed 样式', () => {
      render(<CanvasToolbar />);
      const debugButton = screen.getByTitle('开始调试');
      expect(debugButton).toHaveClass('cursor-not-allowed');
    });
  });

  describe('执行交互', () => {
    it('点击可用按钮触发执行', async () => {
      const user = userEvent.setup();
      const mockFlow = {
        uuid: 'test-flow',
        name: '测试流程',
        nodes: [
          {
            uuid: 'entry-1',
            type: 'entry' as const,
            name: '入口',
            position: { x: 0, y: 0 },
            data: { type: 'entry' as const },
          },
        ],
        connections: [],
        initial_variables: {},
      };

      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: { flow: typeof mockFlow; nodes: typeof mockFlow.nodes; undo: ReturnType<typeof vi.fn>; redo: ReturnType<typeof vi.fn>; past: never[]; future: never[] }) => unknown) => selector({ flow: mockFlow, nodes: mockFlow.nodes, undo: vi.fn(), redo: vi.fn(), past: [], future: [] })
      );

      const { executeFlowWithChannel } = await import('@/services/tauri');
      (executeFlowWithChannel as ReturnType<typeof vi.fn>).mockResolvedValue({
        success: true,
        flow_report: { uuid: 'report-1', status: 'success' },
      });

      render(<CanvasToolbar />);
      const button = screen.getByRole('button', { name: /执行流程/ });
      await user.click(button);

      expect(mockClearLogs).toHaveBeenCalled();
      expect(mockSetConfigPanelOpen).toHaveBeenCalledWith(false);
    });

    it('禁用按钮点击不触发执行', async () => {
      const user = userEvent.setup();

      render(<CanvasToolbar />);
      const button = screen.getByRole('button', { name: /执行流程/ });
      await user.click(button);

      expect(mockClearLogs).not.toHaveBeenCalled();
    });
  });
});
