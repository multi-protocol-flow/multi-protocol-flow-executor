/**
 * FlowListPanel 组件测试
 * 重点测试切换流程时自动保存功能
 * 使用真实 Zustand store 进行集成测试
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FlowListPanel } from '@/components/sidebar/FlowListPanel';
import { useUIStore, useFlowStore, useFlowListStore } from '@/stores';
import * as flowStorage from '@/services/storage/flowStorage';
import type { Flow } from '@/types';

// Mock flowStorage - 只 mock 存储服务
vi.mock('@/services/storage/flowStorage', () => ({
  listFlows: vi.fn(),
  loadFlow: vi.fn(),
  deleteFlow: vi.fn(),
  saveFlow: vi.fn(),
}));

// Mock templates
vi.mock('@/templates/builtinFlows', () => ({
  DEMO_FLOW: {
    uuid: 'demo-flow',
    name: '示例流程',
    nodes: [],
    connections: [],
  },
  BUILTIN_FLOW_VERSION: 1,
}));

describe('FlowListPanel', () => {
  const createMockFlow = (uuid: string, name: string): Flow => ({
    uuid,
    name,
    nodes: [],
    connections: [],
  });

  beforeEach(() => {
    vi.clearAllMocks();
    
    // 重置 Zustand stores
    useFlowStore.getState().reset();
    useFlowListStore.getState().reset();
    useUIStore.getState().closeNewFlowDialog();
    useUIStore.getState().hideConfirm();
    
    // 默认存储状态
    vi.mocked(flowStorage.listFlows).mockReturnValue([]);
    vi.mocked(flowStorage.loadFlow).mockReturnValue(null);
  });

  afterEach(() => {
    useFlowStore.getState().reset();
    useFlowListStore.getState().reset();
  });

  describe('流程列表渲染', () => {
    it('渲染流程列表', async () => {
      const flowMetas = [
        { uuid: 'flow-1', name: '流程1', updated_at: Date.now() },
        { uuid: 'flow-2', name: '流程2', updated_at: Date.now() },
      ];
      vi.mocked(flowStorage.listFlows).mockReturnValue(flowMetas);

      // 先设置 flowListStore
      act(() => {
        useFlowListStore.getState().setFlows(flowMetas);
      });

      render(<FlowListPanel />);
      expect(screen.getByText('流程1')).toBeDefined();
      expect(screen.getByText('流程2')).toBeDefined();
    });
  });

  describe('切换流程时自动保存 (核心修复验证)', () => {
    it('切换流程时，如果当前流程有未保存修改，应自动保存', async () => {
      const user = userEvent.setup();
      const currentFlow = createMockFlow('flow-1', '当前流程');
      const modifiedFlow = { ...currentFlow, name: '修改后的流程' };
      const targetFlow = createMockFlow('flow-2', '目标流程');
      
      const flowMetas = [
        { uuid: 'flow-1', name: '当前流程', updated_at: Date.now() },
        { uuid: 'flow-2', name: '目标流程', updated_at: Date.now() },
      ];

      // 设置当前流程为已修改状态
      act(() => {
        useFlowStore.getState().setFlow(modifiedFlow);
        // 手动设置 isDirty（模拟用户编辑后）
        useFlowStore.setState({ isDirty: true });
        useFlowListStore.getState().setFlows(flowMetas);
      });

      vi.mocked(flowStorage.listFlows).mockReturnValue(flowMetas);
      vi.mocked(flowStorage.loadFlow).mockReturnValue(targetFlow);

      render(<FlowListPanel />);

      // 点击切换到另一个流程
      await user.click(screen.getByText('目标流程'));

      // 验证：应调用 saveFlow 保存当前修改
      expect(flowStorage.saveFlow).toHaveBeenCalledWith(modifiedFlow);
      // 验证：isDirty 应被重置
      expect(useFlowStore.getState().isDirty).toBe(false);
      // 验证：应加载目标流程
      expect(useFlowStore.getState().flow).toEqual(targetFlow);
    });

    it('切换流程时，如果当前流程无修改，不调用 saveFlow (点击后)', async () => {
      const user = userEvent.setup();
      const currentFlow = createMockFlow('flow-1', '当前流程');
      const targetFlow = createMockFlow('flow-2', '目标流程');
      
      const flowMetas = [
        { uuid: 'flow-1', name: '当前流程', updated_at: Date.now() },
        { uuid: 'flow-2', name: '目标流程', updated_at: Date.now() },
      ];

      // 设置当前流程，无未保存修改
      act(() => {
        useFlowStore.getState().setFlow(currentFlow);
        // isDirty 默认为 false
        useFlowListStore.getState().setFlows(flowMetas);
      });

      vi.mocked(flowStorage.listFlows).mockReturnValue(flowMetas);
      vi.mocked(flowStorage.loadFlow).mockReturnValue(targetFlow);

      render(<FlowListPanel />);
      
      // 清除初始化调用
      vi.mocked(flowStorage.saveFlow).mockClear();

      await user.click(screen.getByText('目标流程'));

      // 验证：点击后不应调用 saveFlow（因为 isDirty=false）
      // 注意：只验证点击行为后的调用，不包括组件初始化
      const saveFlowCalls = vi.mocked(flowStorage.saveFlow).mock.calls;
      const callsAfterClick = saveFlowCalls.filter(
        call => call[0].uuid !== 'demo-flow'
      );
      expect(callsAfterClick.length).toBe(0);
      // 验证：应加载目标流程
      expect(useFlowStore.getState().flow).toEqual(targetFlow);
    });
  });

  describe('新建流程', () => {
    it('点击新建按钮打开对话框', async () => {
      const user = userEvent.setup();

      render(<FlowListPanel />);

      // 找到 + 按钮
      const newButton = screen.getByRole('button', { name: '新建流程' });
      await user.click(newButton);

      expect(useUIStore.getState().showNewFlowDialog).toBe(true);
    });
  });
});
