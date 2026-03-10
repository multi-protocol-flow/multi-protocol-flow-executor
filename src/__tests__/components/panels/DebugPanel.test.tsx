/**
 * DebugPanel 组件测试
 * 测试调试面板的渲染和交互
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { DebugPanel } from '@/components/panels/DebugPanel';

// Mock stores
vi.mock('@/stores', () => ({
  useDebugStore: vi.fn(),
  useFlowStore: vi.fn(),
  useUIStore: vi.fn(),
  selectIsPaused: (state: { status: string }) => state.status === 'paused',
  selectCurrentNodeUuid: (state: { currentNodeUuid: string | null }) => state.currentNodeUuid,
  selectIsDebugDataReadOnly: (state: { isDebugDataReadOnly: boolean }) => state.isDebugDataReadOnly,
}));

// Mock tauri service
vi.mock('@/services/tauri', () => ({
  sendDebugCommand: vi.fn(),
}));

import { useDebugStore, useFlowStore, useUIStore } from '@/stores';

describe('DebugPanel', () => {
  const mockFocusOnNode = vi.fn();
  const mockModifyNodeConfig = vi.fn();
  const mockToggleContextVariablesCollapsed = vi.fn();
  const mockToggleExecutionStackCollapsed = vi.fn();
  const mockToggleRecentOutputCollapsed = vi.fn();

  const defaultDebugState = {
    status: 'idle',
    currentNodeUuid: null,
    isDebugDataReadOnly: false,
    variables: {},
    executionStack: [],
    currentNodeOutput: null,
    tempNodeConfigs: {},
    modifyNodeConfig: mockModifyNodeConfig,
  };

  const defaultFlowState = {
    flow: {
      uuid: 'flow-1',
      name: '测试流程',
      nodes: [
        {
          uuid: 'node-1',
          type: 'http',
          name: 'HTTP请求',
          position: { x: 0, y: 0 },
          data: { type: 'http', url: 'https://example.com', method: 'get' },
        },
        {
          uuid: 'node-2',
          type: 'conditional',
          name: '条件判断',
          position: { x: 200, y: 0 },
          data: { type: 'conditional' },
        },
      ],
      connections: [],
    },
    focusOnNode: mockFocusOnNode,
  };

  const defaultUIState = {
    isContextVariablesCollapsed: false,
    isExecutionStackCollapsed: false,
    isRecentOutputCollapsed: false,
    executionStackHeight: 200,
    recentOutputHeight: 150,
    toggleContextVariablesCollapsed: mockToggleContextVariablesCollapsed,
    toggleExecutionStackCollapsed: mockToggleExecutionStackCollapsed,
    toggleRecentOutputCollapsed: mockToggleRecentOutputCollapsed,
    setExecutionStackHeight: vi.fn(),
    setRecentOutputHeight: vi.fn(),
  };

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const mockStores = (debugState: any = {}, flowState: any = {}, uiState: any = {}) => {
    vi.mocked(useDebugStore).mockImplementation((selector: any) => {
      return selector({ ...defaultDebugState, ...debugState });
    });
    vi.mocked(useFlowStore).mockImplementation((selector: any) => {
      return selector({ ...defaultFlowState, ...flowState });
    });
    vi.mocked(useUIStore).mockImplementation((selector: any) => {
      return selector({ ...defaultUIState, ...uiState });
    });
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockStores();
  });

  describe('组件渲染', () => {
    it('应该渲染当前节点区域', () => {
      mockStores({ currentNodeUuid: 'node-1' });

      render(<DebugPanel />);

      // 验证当前节点标题存在
      expect(screen.getByText('当前节点')).toBeDefined();
    });

    it('应该渲染上下文变量区域', () => {
      render(<DebugPanel />);

      // 验证上下文变量标题存在
      expect(screen.getByText('上下文变量')).toBeDefined();
    });

    it('应该渲染执行堆栈区域', () => {
      render(<DebugPanel />);

      // 验证执行堆栈标题存在
      expect(screen.getByText('执行堆栈')).toBeDefined();
    });

    it('应该渲染最近节点输出区域', () => {
      render(<DebugPanel />);

      // 验证最近节点输出标题存在
      expect(screen.getByText('最近节点输出')).toBeDefined();
    });
  });

  describe('当前节点显示', () => {
    it('当无当前节点时应该显示提示信息', () => {
      mockStores({ currentNodeUuid: null });

      render(<DebugPanel />);

      // 验证无节点提示
      expect(screen.getByText('无当前节点')).toBeDefined();
    });

    it('应该显示当前节点名称和类型', () => {
      mockStores({ currentNodeUuid: 'node-1' });

      render(<DebugPanel />);

      // 验证节点名称
      expect(screen.getByText('HTTP请求')).toBeDefined();
      // 验证节点类型
      expect(screen.getByText('http')).toBeDefined();
    });

    it('暂停状态下应该显示编辑配置按钮', () => {
      mockStores({
        status: 'paused',
        currentNodeUuid: 'node-1',
        isDebugDataReadOnly: false,
      });

      render(<DebugPanel />);

      // 验证编辑配置按钮存在
      const editButton = screen.getByRole('button', { name: /编辑配置/i });
      expect(editButton).toBeDefined();
    });

    it('非暂停状态下不应该显示编辑配置按钮', () => {
      mockStores({
        status: 'running',
        currentNodeUuid: 'node-1',
        isDebugDataReadOnly: false,
      });

      render(<DebugPanel />);

      // 编辑配置按钮不应存在
      expect(screen.queryByRole('button', { name: /编辑配置/i })).toBeNull();
    });

    it('只读模式下不应该显示编辑配置按钮', () => {
      mockStores({
        status: 'paused',
        currentNodeUuid: 'node-1',
        isDebugDataReadOnly: true,
      });

      render(<DebugPanel />);

      // 编辑配置按钮不应存在
      expect(screen.queryByRole('button', { name: /编辑配置/i })).toBeNull();
    });
  });

  describe('只读模式', () => {
    it('只读模式下应该显示提示横幅', () => {
      mockStores({ isDebugDataReadOnly: true });

      render(<DebugPanel />);

      // 验证只读模式提示
      expect(screen.getByText('调试已结束')).toBeDefined();
      expect(screen.getByText(/只读模式/)).toBeDefined();
    });

    it('非只读模式下不应该显示提示横幅', () => {
      mockStores({ isDebugDataReadOnly: false });

      render(<DebugPanel />);

      // 只读模式提示不应存在
      expect(screen.queryByText('调试已结束')).toBeNull();
    });
  });

  describe('上下文变量显示', () => {
    it('当无变量时应该显示提示信息', () => {
      mockStores({ variables: {} });

      render(<DebugPanel />);

      // 验证无变量提示
      expect(screen.getByText('暂无变量')).toBeDefined();
    });

    it('应该显示变量列表', () => {
      mockStores({
        variables: {
          status: 200,
          message: 'success',
        },
      });

      render(<DebugPanel />);

      // 验证变量名显示
      expect(screen.getByText('status')).toBeDefined();
      expect(screen.getByText('message')).toBeDefined();
    });

    it('应该显示变量数量', () => {
      mockStores({
        variables: {
          status: 200,
          message: 'success',
          data: { id: 1 },
        },
      });

      render(<DebugPanel />);

      // 验证变量数量显示 (在标题右侧的 headerRight 中)
      // CollapsiblePanel 的 headerRight 渲染在 role="presentation" 的 div 中
      const countElement = screen.getByRole('button', { name: /折叠 上下文变量/ });
      expect(countElement.textContent).toContain('3');
    });
  });

  describe('执行堆栈显示', () => {
    it('当无执行记录时应该显示提示信息', () => {
      mockStores({ executionStack: [] });

      render(<DebugPanel />);

      // 验证无执行记录提示
      expect(screen.getByText('暂无执行记录')).toBeDefined();
    });

    it('应该显示执行堆栈条目', () => {
      mockStores({
        currentNodeUuid: 'node-2',
        executionStack: [
          {
            node_uuid: 'node-1',
            node_name: 'HTTP请求',
            node_type: 'http',
            success: true,
            started_at: 1000,
            ended_at: 1500,
          },
          {
            node_uuid: 'node-2',
            node_name: '条件判断',
            node_type: 'conditional',
            success: true,
            started_at: 1600,
            ended_at: 1700,
          },
        ],
      });

      render(<DebugPanel />);

      // 验证节点名称显示
      // 使用 getAllByText 因为可能有多个相同名称的元素
      expect(screen.getAllByText('HTTP请求').length).toBeGreaterThan(0);
      expect(screen.getAllByText('条件判断').length).toBeGreaterThan(0);
    });

    it('应该显示执行堆栈数量', () => {
      mockStores({
        executionStack: [
          {
            node_uuid: 'node-1',
            node_name: 'HTTP请求',
            node_type: 'http',
            success: true,
          },
        ],
      });

      render(<DebugPanel />);

      // 验证堆栈数量显示 (在标题右侧)
      const countElement = screen.getByRole('button', { name: /折叠 执行堆栈/ });
      expect(countElement.textContent).toContain('1');
    });

    it('点击堆栈条目应该跳转到对应节点', async () => {
      const user = userEvent.setup();
      mockStores({
        currentNodeUuid: 'node-2',
        executionStack: [
          {
            node_uuid: 'node-1',
            node_name: 'HTTP请求',
            node_type: 'http',
            success: true,
          },
        ],
      });

      render(<DebugPanel />);

      // 点击节点名称
      await user.click(screen.getByText('HTTP请求'));

      // 验证跳转函数被调用
      expect(mockFocusOnNode).toHaveBeenCalledWith('node-1');
    });
  });

  describe('最近节点输出显示', () => {
    it('当无输出时应该显示提示信息', () => {
      mockStores({ currentNodeOutput: null });

      render(<DebugPanel />);

      // 验证无输出提示
      expect(screen.getByText('暂无输出')).toBeDefined();
    });

    it('应该显示节点输出', () => {
      mockStores({
        currentNodeOutput: { status: 200, body: { id: 1 } },
      });

      render(<DebugPanel />);

      // 验证输出内容显示（JSON 格式）
      expect(screen.getByText(/"status": 200/)).toBeDefined();
    });
  });

  describe('折叠面板交互', () => {
    it('点击上下文变量标题应该触发折叠切换', async () => {
      const user = userEvent.setup();
      render(<DebugPanel />);

      // 点击上下文变量标题 (CollapsiblePanel header has role="button")
      const header = screen.getByRole('button', { name: /折叠 上下文变量/ });
      await user.click(header);

      // 验证折叠函数被调用
      expect(mockToggleContextVariablesCollapsed).toHaveBeenCalled();
    });

    it('点击执行堆栈标题应该触发折叠切换', async () => {
      const user = userEvent.setup();
      render(<DebugPanel />);

      // 点击执行堆栈标题
      const header = screen.getByRole('button', { name: /折叠 执行堆栈/ });
      await user.click(header);

      // 验证折叠函数被调用
      expect(mockToggleExecutionStackCollapsed).toHaveBeenCalled();
    });

    it('点击最近节点输出标题应该触发折叠切换', async () => {
      const user = userEvent.setup();
      render(<DebugPanel />);

      // 点击最近节点输出标题
      const header = screen.getByRole('button', { name: /折叠 最近节点输出/ });
      await user.click(header);

      // 验证折叠函数被调用
      expect(mockToggleRecentOutputCollapsed).toHaveBeenCalled();
    });
  });

  describe('临时配置标记', () => {
    it('当节点有临时配置时应该显示已修改标记', () => {
      mockStores({
        status: 'paused',
        currentNodeUuid: 'node-1',
        isDebugDataReadOnly: false,
        tempNodeConfigs: {
          'node-1': { type: 'http', url: 'https://modified.com', method: 'get' },
        },
      });

      render(<DebugPanel />);

      // 验证已修改标记显示
      expect(screen.getByText('(已修改)')).toBeDefined();
    });

    it('当节点无临时配置时不应该显示已修改标记', () => {
      mockStores({
        status: 'paused',
        currentNodeUuid: 'node-1',
        isDebugDataReadOnly: false,
        tempNodeConfigs: {},
      });

      render(<DebugPanel />);

      // 已修改标记不应存在
      expect(screen.queryByText('(已修改)')).toBeNull();
    });
  });
});
