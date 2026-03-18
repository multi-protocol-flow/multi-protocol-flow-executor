/**
 * MainLayout 组件测试 - 右侧面板拖拽功能
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/react';
import { MainLayout } from '@/layouts/MainLayout';
import { useUIStore, useFlowStore, useDebugStore } from '@/stores';
import { useConsoleStore } from '@/stores/console';

// Mock 所有 stores
vi.mock('@/stores', () => ({
  useUIStore: vi.fn(),
  useFlowStore: vi.fn(),
  useDebugStore: vi.fn(),
  useConnectionsStore: vi.fn(),
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
}));

vi.mock('@/components/panels', () => ({
  NodeConfigPanel: () => <div data-testid="node-config-panel">NodeConfig</div>,
  SettingsPanel: () => <div data-testid="settings-panel">Settings</div>,
  DebugPanel: () => <div data-testid="debug-panel">Debug</div>,
}));

vi.mock('@/components/execution', () => ({
  ExecutionReport: () => <div data-testid="execution-report">Report</div>,
  ExecutionHistory: () => <div data-testid="execution-history">History</div>,
  ConsolePanel: () => <div data-testid="console-panel">Console</div>,
}));

vi.mock('@/components/dialogs/NewFlowDialog', () => ({
  NewFlowDialog: () => <div data-testid="new-flow-dialog">NewFlow</div>,
}));

vi.mock('@/components/dialogs/ConfirmDialog', () => ({
  ConfirmDialog: ({ open }: { open: boolean }) => open ? <div data-testid="confirm-dialog">Confirm</div> : null,
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
      
      // 拖拽开始后，className 应该包含单独的 'bg-slate-300'（不是 'active:bg-slate-300'）
      // 通过检查类名列表来验证
      const classList = resizeHandle.className.split(' ');
      expect(classList).toContain('bg-slate-300');
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
      
      // 拖拽开始时应该有 bg-slate-300 类
      let classList = resizeHandle.className.split(' ');
      expect(classList).toContain('bg-slate-300');
      
      fireEvent.mouseUp(document);
      
      // 拖拽结束后，className 中不应该包含单独的 'bg-slate-300'（但 'active:bg-slate-300' 仍然存在）
      classList = resizeHandle.className.split(' ');
      expect(classList).not.toContain('bg-slate-300');
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
});
