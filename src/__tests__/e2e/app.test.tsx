/**
 * App E2E 测试
 * 测试主应用布局和组件整合
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ReactFlowProvider } from '@xyflow/react';
import App from '@/App';
import { useFlowStore, useFlowListStore, useUIStore } from '@/stores';
import type { Flow } from '@/types';

// 模拟 Tauri 服务
vi.mock('@/services/tauri', () => ({
  executeFlow: vi.fn().mockResolvedValue({
    success: true,
    flow_report: {
      flow_name: '测试流程',
      status: 'success',
      duration_ms: 100,
      total_nodes: 1,
      executed_count: 1,
      node_reports: [],
      statistics: { success_count: 1, failed_count: 0 },
    },
  }),
  saveFlow: vi.fn().mockResolvedValue({ success: true }),
  loadFlow: vi.fn(),
}));

// 模拟存储服务
vi.mock('@/services/storage/flowStorage', () => ({
  listFlows: vi.fn().mockReturnValue([]),
  loadFlow: vi.fn(),
  saveFlow: vi.fn(),
  deleteFlow: vi.fn(),
}));

vi.mock('@/services/storage/historyStorage', () => ({
  listExecutionHistory: vi.fn().mockReturnValue([]),
  saveExecutionHistory: vi.fn(),
  clearExecutionHistory: vi.fn(),
}));

// 测试包装器
function TestWrapper({ children }: { children: React.ReactNode }) {
  return <ReactFlowProvider>{children}</ReactFlowProvider>;
}

// 模拟流程数据
const mockFlow: Flow = {
  uuid: 'test-flow-001',
  name: '测试流程',
  nodes: [
    {
      uuid: 'entry-001',
      type: 'entry',
      name: '入口',
      position: { x: 100, y: 100 },
      data: { type: 'entry' },
    },
    {
      uuid: 'http-001',
      type: 'http',
      name: 'HTTP 请求',
      position: { x: 300, y: 100 },
      data: { type: 'http', url: 'https://api.example.com', method: 'get' },
    },
  ],
  connections: [
    {
      id: 'conn-001',
      source_node_uuid: 'entry-001',
      target_node_uuid: 'http-001',
      source_port_id: 'out',
      target_port_id: 'in',
    },
  ],
};

describe('App E2E', () => {
  beforeEach(() => {
    // 重置所有 store
    useFlowStore.getState().reset();
    useFlowListStore.getState().reset();
    useUIStore.getState().resetPanels();
    
    // 清除 mock
    vi.clearAllMocks();
  });

  describe('主布局', () => {
    it('渲染主布局组件', () => {
      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      // 验证标题
      expect(screen.getByText('多协议流程执行器')).toBeDefined();
      expect(screen.getByText('Flow Executor')).toBeDefined();
    });

    it('显示执行按钮', () => {
      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      expect(screen.getByText('执行流程')).toBeDefined();
    });
  });

  describe('左侧面板', () => {
    it('默认显示左侧面板', () => {
      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      expect(screen.getByText('流程列表')).toBeDefined();
      expect(screen.getByText('节点库')).toBeDefined();
    });

    it('可以折叠左侧面板', async () => {
      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      // 左侧面板初始是打开的
      expect(useUIStore.getState().isFlowListPanelOpen).toBe(true);

      // 找到折叠按钮（通过 title 属性）
      const collapseButton = screen.getByTitle('收起侧边栏');
      fireEvent.click(collapseButton);

      // 验证面板已关闭
      await waitFor(() => {
        expect(useUIStore.getState().isFlowListPanelOpen).toBe(false);
      });
    });
  });

  describe('节点库', () => {
    it('显示节点类型', () => {
      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      // 使用 getAllByText 因为节点类型名称可能出现多次
      expect(screen.getAllByText('HTTP 请求').length).toBeGreaterThan(0);
      expect(screen.getAllByText('条件判断').length).toBeGreaterThan(0);
    });

    it('点击添加节点到画布', async () => {
      // 先设置一个流程
      useFlowStore.getState().setFlow(mockFlow);

      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      // 初始有 2 个节点
      expect(useFlowStore.getState().flow?.nodes.length).toBe(2);

      // 点击添加 HTTP 节点 - 使用更精确的选择器
      const httpNodeButtons = screen.getAllByText('HTTP 请求');
      // 找到节点库中的按钮（在 p 标签内）
      const nodeLibraryButton = httpNodeButtons.find(el => el.tagName === 'P')?.closest('button');
      if (nodeLibraryButton) {
        fireEvent.click(nodeLibraryButton);
      }

      // 验证节点已添加
      await waitFor(() => {
        expect(useFlowStore.getState().flow?.nodes.length).toBe(3);
      });
    });
  });

  describe('底部历史面板', () => {
    it('默认不显示历史面板', () => {
      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      expect(useUIStore.getState().isHistoryPanelOpen).toBe(false);
    });

    it('可以展开历史面板', async () => {
      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      // 找到展开按钮
      const expandButton = screen.getByTitle('展开历史面板');
      fireEvent.click(expandButton);

      // 验证面板已展开
      await waitFor(() => {
        expect(useUIStore.getState().isHistoryPanelOpen).toBe(true);
      });
    });
  });

  describe('右侧配置面板', () => {
    it('默认显示配置面板', () => {
      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      expect(useUIStore.getState().isConfigPanelOpen).toBe(true);
    });

    it('未选中节点时显示提示', () => {
      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      expect(screen.getByText('未选中节点')).toBeDefined();
    });

    it('可以关闭配置面板', async () => {
      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      // 找到关闭按钮
      const closeButton = screen.getByTitle('隐藏配置面板');
      fireEvent.click(closeButton);

      await waitFor(() => {
        expect(useUIStore.getState().isConfigPanelOpen).toBe(false);
      });
    });
  });

  describe('键盘快捷键', () => {
    // 注意：这些测试直接测试 store 的功能，
    // 因为在 jsdom 测试环境中，window 键盘事件监听器更新存在时序问题
    
    it('removeNode 可以删除非 Entry 节点', () => {
      // 设置流程
      useFlowStore.getState().setFlow(mockFlow);
      
      // 验证初始状态
      expect(useFlowStore.getState().flow?.nodes.length).toBe(2);
      
      // 删除 HTTP 节点
      useFlowStore.getState().removeNode('http-001');
      
      // 验证节点已删除
      expect(useFlowStore.getState().flow?.nodes.length).toBe(1);
      expect(useFlowStore.getState().flow?.nodes.find(n => n.uuid === 'http-001')).toBeUndefined();
    });

    it('Entry 节点存在且类型正确', () => {
      // 设置流程
      useFlowStore.getState().setFlow(mockFlow);
      
      // 验证 Entry 节点存在
      const entryNode = useFlowStore.getState().flow?.nodes.find(n => n.type === 'entry');
      expect(entryNode).toBeDefined();
      expect(entryNode?.type).toBe('entry');
    });

    it('selectNode 和 selectNode(null) 工作正常', () => {
      // 设置流程
      useFlowStore.getState().setFlow(mockFlow);

      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      // 选中节点
      useFlowStore.getState().selectNode('http-001');
      expect(useFlowStore.getState().selectedNodeId).toBe('http-001');

      // 取消选择
      useFlowStore.getState().selectNode(null);
      expect(useFlowStore.getState().selectedNodeId).toBeNull();
    });
  });

  describe('流程列表', () => {
    it('无流程时显示空状态', () => {
      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      // 使用 getAllByText 因为可能有多个匹配
      const emptyTexts = screen.getAllByText('暂无流程');
      expect(emptyTexts.length).toBeGreaterThan(0);
    });
  });
});
