/**
 * MainLayout 组件测试 - 右侧面板拖拽功能
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/react';
import { MainLayout } from '@/layouts/MainLayout';
import { useUIStore, useFlowStore, useDebugStore, usePluginsStore } from '@/stores';
import { useConsoleStore } from '@/stores/console';

// Mock 所有 stores
vi.mock('@/stores', () => ({
  useUIStore: vi.fn(),
  useFlowStore: vi.fn(),
  useDebugStore: vi.fn(),
  usePluginsStore: vi.fn(),
  selectIsDebugging: (state: unknown) => (state as { isDebugging: boolean }).isDebugging,
  selectIsDebugDataReadOnly: (state: unknown) => (state as { isDebugDataReadOnly: boolean }).isDebugDataReadOnly,
}));

vi.mock('@/stores/console', () => ({
  useConsoleStore: vi.fn(),
}));

// Mock 子组件
vi.mock('@/components/sidebar', () => ({
  LeftSidebarPanel: () => <div data-testid="left-sidebar-panel">LeftSidebar</div>,
}));

vi.mock('@/components/canvas', () => ({
  FlowCanvas: () => <div data-testid="flow-canvas">FlowCanvas</div>,
  FlowTabBar: () => <div data-testid="flow-tab-bar">FlowTabBar</div>,
}));

vi.mock('@/components/panels', () => ({
  NodeConfigPanel: () => <div data-testid="node-config-panel">NodeConfig</div>,
  DebugPanel: () => <div data-testid="debug-panel">Debug</div>,
}));

vi.mock('@/components/execution', () => ({
  ExecutionReport: () => <div data-testid="execution-report">Report</div>,
  ExecutionPanel: () => <div data-testid="execution-panel">ExecutionPanel</div>,
  ExecutionHistory: () => <div data-testid="execution-history">History</div>,
  ConsolePanel: () => <div data-testid="console-panel">Console</div>,
}));

vi.mock('@/components/execution/FlowCompareDialog', () => ({
  FlowCompareDialog: () => <div data-testid="flow-compare-dialog">FC</div>,
}));

vi.mock('@/components/reports', () => ({
  DatasetReportList: () => <div data-testid="dataset-report-list">List</div>,
  DatasetReportDetail: () => <div data-testid="dataset-report-detail">Detail</div>,
  DatasetRowDetail: () => <div data-testid="dataset-row-detail">Row</div>,
  DatasetReportDialog: () => <div data-testid="dataset-report-dialog">Dialog</div>,
}));

vi.mock('@/components/dialogs/NewFlowDialog', () => ({
  NewFlowDialog: () => <div data-testid="new-flow-dialog">NewFlow</div>,
}));

vi.mock('@/components/dialogs/ConfirmDialog', () => ({
  ConfirmDialog: ({ open, onConfirm, onCancel }: { open: boolean; onConfirm?: () => void; onCancel?: () => void }) =>
    open ? (
      <div data-testid="confirm-dialog">
        <button data-testid="confirm-dialog-confirm" onClick={onConfirm}>Confirm</button>
        <button data-testid="confirm-dialog-cancel" onClick={onCancel}>Cancel</button>
      </div>
    ) : null,
}));

vi.mock('@/components/ui/sonner', () => ({
  Toaster: () => <div data-testid="toaster">Toaster</div>,
}));

describe('MainLayout - 右侧面板拖拽功能', () => {
  const mockSetConfigPanelWidth = vi.fn();
  const mockSetBottomPanelHeight = vi.fn();
  const mockSetSidebarWidth = vi.fn();

  const defaultUIState = {
    isFlowListPanelOpen: true,
    isHistoryPanelOpen: false,
    isConfigPanelOpen: true,
    configPanelMode: 'node' as const,
    bottomPanelTab: 'history' as const,
    bottomPanelHeight: 208,
    configPanelWidth: 320,
    sidebarWidth: 280,
    toggleFlowListPanel: vi.fn(),
    toggleHistoryPanel: vi.fn(),
    toggleConfigPanel: vi.fn(),
    setBottomPanelTab: vi.fn(),
    setBottomPanelHeight: mockSetBottomPanelHeight,
    setConfigPanelWidth: mockSetConfigPanelWidth,
    setSidebarWidth: mockSetSidebarWidth,
    openSettingsDialog: vi.fn(),
    showConfirm: vi.fn(),
    isBottomPanelExpanded: false,
    setBottomPanelExpanded: vi.fn(),
    toggleBottomPanel: vi.fn(),
    showConfirmDialog: false,
    confirmDialogConfig: null,
    hideConfirm: vi.fn(),
    currentPage: 'canvas' as const,
    datasetReportPage: { reportId: null, rowIndex: null },
    navigateToDatasetReportDetail: vi.fn(),
    navigateToDatasetRowDetail: vi.fn(),
  };

  const defaultFlowState = {
    isDirty: false,
    flow: { uuid: 'test-flow', name: '测试流程' },
  };

  const defaultDebugState = {
    isDebugging: false,
    isDebugDataReadOnly: false,
  };

  const defaultConsoleState = {
    filter: { level: 'ALL', nodeUuid: 'ALL', searchKeyword: '' },
    clearLogs: vi.fn(),
    setFilter: vi.fn(),
    setSearchKeyword: vi.fn(),
    getUniqueNodes: vi.fn(() => []),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    
    (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
      return selector ? selector(defaultUIState) : defaultUIState;
    });
    
    (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
      return selector ? selector(defaultFlowState) : defaultFlowState;
    });
    
    (useDebugStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
      return selector ? selector(defaultDebugState) : defaultDebugState;
    });

    const defaultPluginsState = {
      pluginDirInfo: null,
      installed: [],
      marketplace: [],
      loading: false,
      error: null,
      loadPluginDirInfo: vi.fn(),
      loadInstalled: vi.fn(),
      reloadPlugins: vi.fn(),
      loadMarketplace: vi.fn(),
      clearMarketplace: vi.fn(),
      reset: vi.fn(),
    };
    (usePluginsStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
      return selector ? selector(defaultPluginsState) : defaultPluginsState;
    });
    
    (useConsoleStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue(defaultConsoleState);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('右侧面板渲染', () => {
    it('右侧面板展开时显示拖拽 handle', () => {
      const { container } = render(<MainLayout />);
      
      // 查找右侧面板（通过 border-l 类）
      const rightAside = container.querySelector('aside.border-l');
      const resizeHandle = rightAside?.querySelector('[title="拖拽调整面板宽度"]');
      expect(resizeHandle).toBeDefined();
    });

    it('右侧面板收起时隐藏拖拽 handle', () => {
      (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = { ...defaultUIState, isConfigPanelOpen: false };
        return selector ? selector(state) : state;
      });

      const { container } = render(<MainLayout />);
      
      // 查找右侧面板（通过 border-l 类）
      const rightAside = container.querySelector('aside.border-l');
      const resizeHandle = rightAside?.querySelector('[title="拖拽调整面板宽度"]');
      expect(resizeHandle).toBeNull();
    });

    it('右侧面板使用 store 中的宽度', () => {
      const { container } = render(<MainLayout />);
      
      // 查找右侧面板 aside 元素
      const aside = container.querySelector('aside.border-l');
      expect(aside).toBeDefined();
      expect((aside as HTMLElement).style.width).toBe('320px');
    });
  });

  describe('右侧面板拖拽交互', () => {
    it('鼠标按下拖拽 handle 开始拖拽', () => {
      const { container } = render(<MainLayout />);
      
      const rightAside = container.querySelector('aside.border-l');
      const resizeHandle = rightAside?.querySelector('[title="拖拽调整面板宽度"]') as HTMLElement;
      expect(resizeHandle).toBeDefined();
      
      fireEvent.mouseDown(resizeHandle);
      
      // 拖拽开始后，className 应该包含单独的 'bg-input'（不是 'active:bg-input'）
      // 通过检查类名列表来验证
      const classList = resizeHandle.className.split(' ');
      expect(classList).toContain('bg-input');
    });

    it('拖拽时鼠标移动更新宽度', () => {
      const { container } = render(<MainLayout />);
      
      const rightAside = container.querySelector('aside.border-l');
      const resizeHandle = rightAside?.querySelector('[title="拖拽调整面板宽度"]') as HTMLElement;
      
      // 开始拖拽
      fireEvent.mouseDown(resizeHandle, { clientX: 500 });
      
      // 模拟鼠标向左移动 50px（宽度增加）
      fireEvent.mouseMove(document, { clientX: 450 });
      
      // 验证 setConfigPanelWidth 被调用
      expect(mockSetConfigPanelWidth).toHaveBeenCalledWith(370); // 320 + 50
    });

    it('鼠标释放后停止拖拽', () => {
      const { container } = render(<MainLayout />);
      
      const rightAside = container.querySelector('aside.border-l');
      const resizeHandle = rightAside?.querySelector('[title="拖拽调整面板宽度"]') as HTMLElement;
      
      fireEvent.mouseDown(resizeHandle);
      
      // 拖拽开始时应该有 bg-input 类
      let classList = resizeHandle.className.split(' ');
      expect(classList).toContain('bg-input');
      
      fireEvent.mouseUp(document);
      
      // 拖拽结束后，className 中不应该包含单独的 'bg-input'（但 'active:bg-input' 仍然存在）
      classList = resizeHandle.className.split(' ');
      expect(classList).not.toContain('bg-input');
    });
  });

  describe('右侧面板宽度限制', () => {
    it('宽度不能小于最小值 250px', () => {
      const { container } = render(<MainLayout />);
      
      const rightAside = container.querySelector('aside.border-l');
      const resizeHandle = rightAside?.querySelector('[title="拖拽调整面板宽度"]') as HTMLElement;
      
      // 开始拖拽，当前宽度 320px
      fireEvent.mouseDown(resizeHandle, { clientX: 500 });
      
      // 模拟鼠标向右移动 100px（宽度减少到 220，但应该限制在 250）
      fireEvent.mouseMove(document, { clientX: 600 });
      
      // 验证宽度被限制在最小值
      expect(mockSetConfigPanelWidth).toHaveBeenCalledWith(250);
    });

    it('宽度不能大于最大值 500px', () => {
      const { container } = render(<MainLayout />);
      
      const rightAside = container.querySelector('aside.border-l');
      const resizeHandle = rightAside?.querySelector('[title="拖拽调整面板宽度"]') as HTMLElement;
      
      // 开始拖拽，当前宽度 320px
      fireEvent.mouseDown(resizeHandle, { clientX: 500 });
      
      // 模拟鼠标向左移动 250px（宽度增加到 570，但应该限制在 500）
      fireEvent.mouseMove(document, { clientX: 250 });
      
      // 验证宽度被限制在最大值
      expect(mockSetConfigPanelWidth).toHaveBeenCalledWith(500);
    });

    it('宽度在范围内时正常更新', () => {
      const { container } = render(<MainLayout />);
      
      const rightAside = container.querySelector('aside.border-l');
      const resizeHandle = rightAside?.querySelector('[title="拖拽调整面板宽度"]') as HTMLElement;
      
      // 开始拖拽，当前宽度 320px
      fireEvent.mouseDown(resizeHandle, { clientX: 500 });
      
      // 模拟鼠标向左移动 30px（宽度增加到 350）
      fireEvent.mouseMove(document, { clientX: 470 });
      
      // 验证宽度正常更新
      expect(mockSetConfigPanelWidth).toHaveBeenCalledWith(350);
    });
  });

  describe('折叠按钮位置', () => {
    it('折叠按钮位置根据面板宽度动态调整', () => {
      const { container } = render(<MainLayout />);
      
      // 查找折叠按钮
      const foldButton = container.querySelector('button[class*="rounded-r-none"]') as HTMLElement;
      expect(foldButton).toBeDefined();
      
      // 验证按钮位置（right 应该是 configPanelWidth = 320，紧贴面板）
      expect(foldButton.style.right).toBe('320px');
    });

    it('面板收起时折叠按钮在右边缘', () => {
      (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = { ...defaultUIState, isConfigPanelOpen: false };
        return selector ? selector(state) : state;
      });

      const { container } = render(<MainLayout />);
      
      const foldButton = container.querySelector('button[class*="rounded-r-none"]') as HTMLElement;
      expect(foldButton).toBeDefined();
      expect(foldButton.style.right).toBe('0px');
    });
  });

  // ===== 新测试：Tab 切换功能 =====
  describe('Tab 切换功能', () => {
    it('点击 console 按钮切换到控制台面板', () => {
      (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = { ...defaultUIState, bottomPanelTab: 'history' as const, isBottomPanelExpanded: false };
        return selector ? selector(state) : state;
      });

      const { container } = render(<MainLayout />);
      
      const buttons = container.querySelectorAll('button');
      const consoleBtn = Array.from(buttons).find((btn) => btn.textContent === '控制台');
      expect(consoleBtn).toBeDefined();
      
      fireEvent.click(consoleBtn!);
      
      expect(defaultUIState.setBottomPanelTab).toHaveBeenCalledWith('console');
    });

    it('点击当前激活的 tab 收起面板', () => {
      (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = { ...defaultUIState, bottomPanelTab: 'history' as const, isBottomPanelExpanded: true };
        return selector ? selector(state) : state;
      });

      const { container } = render(<MainLayout />);
      
      const buttons = container.querySelectorAll('button');
      const historyBtn = Array.from(buttons).find((btn) => btn.textContent === '执行历史');
      expect(historyBtn).toBeDefined();
      
      fireEvent.click(historyBtn!);
      
      expect(defaultUIState.setBottomPanelTab).toHaveBeenCalledWith('');
      expect(defaultUIState.setBottomPanelExpanded).toHaveBeenCalledWith(false);
    });

    it('展开状态下点击不同 tab 切换', () => {
      (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = { ...defaultUIState, bottomPanelTab: 'history' as const, isBottomPanelExpanded: true };
        return selector ? selector(state) : state;
      });

      const { container } = render(<MainLayout />);
      
      const buttons = container.querySelectorAll('button');
      const consoleBtn = Array.from(buttons).find((btn) => btn.textContent === '控制台');
      expect(consoleBtn).toBeDefined();
      
      fireEvent.click(consoleBtn!);
      
      expect(defaultUIState.setBottomPanelTab).toHaveBeenCalledWith('console');
      // 因为已经展开，不应调用 setBottomPanelExpanded(true)
      expect(defaultUIState.setBottomPanelExpanded).not.toHaveBeenCalledWith(true);
    });
  });

  // ===== 新测试：底部面板拖拽功能 =====
  describe('底部面板拖拽功能', () => {
    it('底部面板展开时显示拖拽 handle', () => {
      (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = { ...defaultUIState, isBottomPanelExpanded: true };
        return selector ? selector(state) : state;
      });

      const { container } = render(<MainLayout />);
      
      const handle = container.querySelector('[title="拖拽调整面板高度"]');
      expect(handle).toBeDefined();
    });

    it('拖拽底部面板更新高度', () => {
      (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = { ...defaultUIState, isBottomPanelExpanded: true, bottomPanelHeight: 208 };
        return selector ? selector(state) : state;
      });

      const { container } = render(<MainLayout />);
      
      const handle = container.querySelector('[title="拖拽调整面板高度"]') as HTMLElement;
      expect(handle).toBeDefined();
      
      fireEvent.mouseDown(handle, { clientY: 300 });
      // delta = 300 - 250 = 50 → new height = 208 + 50 = 258
      fireEvent.mouseMove(document, { clientY: 250 });
      
      expect(mockSetBottomPanelHeight).toHaveBeenCalledWith(258);
    });

    it('底部面板高度不能小于最小值 100px', () => {
      (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = { ...defaultUIState, isBottomPanelExpanded: true, bottomPanelHeight: 208 };
        return selector ? selector(state) : state;
      });

      const { container } = render(<MainLayout />);
      
      const handle = container.querySelector('[title="拖拽调整面板高度"]') as HTMLElement;
      expect(handle).toBeDefined();
      
      fireEvent.mouseDown(handle, { clientY: 300 });
      // delta = 300 - 600 = -300 → new height = 208 - 300 = -92 → clamped to 100
      fireEvent.mouseMove(document, { clientY: 600 });
      
      expect(mockSetBottomPanelHeight).toHaveBeenCalledWith(100);
    });

    it('底部面板高度支持向上拖拽扩大', () => {
      (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = { ...defaultUIState, isBottomPanelExpanded: true, bottomPanelHeight: 208 };
        return selector ? selector(state) : state;
      });

      const { container } = render(<MainLayout />);
      
      const handle = container.querySelector('[title="拖拽调整面板高度"]') as HTMLElement;
      expect(handle).toBeDefined();
      
      fireEvent.mouseDown(handle, { clientY: 300 });
      // delta = 300 - (-250) = 550 → new height = 208 + 550 = 758 → clamped to max (700 in jsdom window)
      fireEvent.mouseMove(document, { clientY: -250 });
      
      expect(mockSetBottomPanelHeight).toHaveBeenCalledWith(700);
    });
  });

  // ===== 新测试：报告对比对话框 =====
  describe('报告对比对话框', () => {
    it('报告对比对话框默认不渲染', () => {
      const { queryByTestId } = render(<MainLayout />);
      
      // compareSnapshotFlow 初始为 null，对话框不应渲染
      expect(queryByTestId('flow-compare-dialog')).toBeNull();
    });
  });

  // ===== 新测试：数据集页面导航 =====
  describe('数据集页面导航', () => {
    it('数据集报告列表页面渲染', () => {
      (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = {
          ...defaultUIState,
          currentPage: 'dataset-reports' as const,
          datasetReportPage: { reportId: null, rowIndex: null },
        };
        return selector ? selector(state) : state;
      });

      const { getByTestId } = render(<MainLayout />);
      expect(getByTestId('dataset-report-list')).toBeDefined();
    });

    it('数据集报告详情页面渲染', () => {
      (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = {
          ...defaultUIState,
          currentPage: 'dataset-reports' as const,
          datasetReportPage: { reportId: 'test-report', rowIndex: null },
        };
        return selector ? selector(state) : state;
      });

      const { getByTestId } = render(<MainLayout />);
      expect(getByTestId('dataset-report-detail')).toBeDefined();
    });

    it('数据集行详情页面渲染', () => {
      (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = {
          ...defaultUIState,
          currentPage: 'dataset-reports' as const,
          datasetReportPage: { reportId: 'test-report', rowIndex: 0 },
        };
        return selector ? selector(state) : state;
      });

      const { getByTestId } = render(<MainLayout />);
      expect(getByTestId('dataset-row-detail')).toBeDefined();
    });
  });

  // ===== 新测试：确认对话框 =====
  describe('确认对话框', () => {
    it('确认对话框显示', () => {
      (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = {
          ...defaultUIState,
          showConfirmDialog: true,
          confirmDialogConfig: {
            title: '确认删除',
            description: '确定要删除吗？',
            confirmText: '确定',
            onConfirm: vi.fn(),
            onCancel: vi.fn(),
          },
        };
        return selector ? selector(state) : state;
      });

      const { getByTestId } = render(<MainLayout />);
      expect(getByTestId('confirm-dialog')).toBeDefined();
    });

    it('确认对话框确认按钮触发回调', () => {
      const mockOnConfirm = vi.fn();
      const mockOnCancel = vi.fn();
      (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = {
          ...defaultUIState,
          showConfirmDialog: true,
          confirmDialogConfig: {
            title: '确认删除',
            description: '确定要删除吗？',
            confirmText: '确定',
            onConfirm: mockOnConfirm,
            onCancel: mockOnCancel,
          },
        };
        return selector ? selector(state) : state;
      });

      const { getByTestId } = render(<MainLayout />);
      
      fireEvent.click(getByTestId('confirm-dialog-confirm'));
      
      expect(mockOnConfirm).toHaveBeenCalled();
      expect(defaultUIState.hideConfirm).toHaveBeenCalled();
    });
  });
});
