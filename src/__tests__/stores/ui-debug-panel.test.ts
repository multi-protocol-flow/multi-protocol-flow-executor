/**
 * UI Store - 调试面板折叠状态测试
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { useUIStore } from '@/stores/ui';

describe('UIStore - 调试面板折叠状态', () => {
  beforeEach(() => {
    // 重置调试面板相关状态到初始状态
    useUIStore.setState({
      isContextVariablesCollapsed: false,
      isExecutionStackCollapsed: false,
      isRecentOutputCollapsed: false,
      executionStackHeight: 192,
      recentOutputHeight: 192,
    });
  });

  describe('初始状态', () => {
    it('调试面板折叠状态默认为 false（展开）', () => {
      const state = useUIStore.getState();
      expect(state.isContextVariablesCollapsed).toBe(false);
      expect(state.isExecutionStackCollapsed).toBe(false);
      expect(state.isRecentOutputCollapsed).toBe(false);
    });

    it('调试面板高度默认为 192', () => {
      const state = useUIStore.getState();
      expect(state.executionStackHeight).toBe(192);
      expect(state.recentOutputHeight).toBe(192);
    });
  });

  describe('toggleContextVariablesCollapsed', () => {
    it('切换上下文变量折叠状态', () => {
      expect(useUIStore.getState().isContextVariablesCollapsed).toBe(false);

      useUIStore.getState().toggleContextVariablesCollapsed();
      expect(useUIStore.getState().isContextVariablesCollapsed).toBe(true);

      useUIStore.getState().toggleContextVariablesCollapsed();
      expect(useUIStore.getState().isContextVariablesCollapsed).toBe(false);
    });
  });

  describe('toggleExecutionStackCollapsed', () => {
    it('切换执行堆栈折叠状态', () => {
      expect(useUIStore.getState().isExecutionStackCollapsed).toBe(false);

      useUIStore.getState().toggleExecutionStackCollapsed();
      expect(useUIStore.getState().isExecutionStackCollapsed).toBe(true);

      useUIStore.getState().toggleExecutionStackCollapsed();
      expect(useUIStore.getState().isExecutionStackCollapsed).toBe(false);
    });
  });

  describe('toggleRecentOutputCollapsed', () => {
    it('切换最近节点输出折叠状态', () => {
      expect(useUIStore.getState().isRecentOutputCollapsed).toBe(false);

      useUIStore.getState().toggleRecentOutputCollapsed();
      expect(useUIStore.getState().isRecentOutputCollapsed).toBe(true);

      useUIStore.getState().toggleRecentOutputCollapsed();
      expect(useUIStore.getState().isRecentOutputCollapsed).toBe(false);
    });
  });

  describe('setExecutionStackHeight', () => {
    it('设置执行堆栈高度', () => {
      useUIStore.getState().setExecutionStackHeight(250);
      expect(useUIStore.getState().executionStackHeight).toBe(250);
    });

    it('高度有最小值限制（100）', () => {
      useUIStore.getState().setExecutionStackHeight(50);
      expect(useUIStore.getState().executionStackHeight).toBe(100);
    });

    it('高度有最大值限制（400）', () => {
      useUIStore.getState().setExecutionStackHeight(500);
      expect(useUIStore.getState().executionStackHeight).toBe(400);
    });

    it('高度在有效范围内正常设置', () => {
      useUIStore.getState().setExecutionStackHeight(150);
      expect(useUIStore.getState().executionStackHeight).toBe(150);

      useUIStore.getState().setExecutionStackHeight(300);
      expect(useUIStore.getState().executionStackHeight).toBe(300);
    });
  });

  describe('setRecentOutputHeight', () => {
    it('设置最近节点输出高度', () => {
      useUIStore.getState().setRecentOutputHeight(280);
      expect(useUIStore.getState().recentOutputHeight).toBe(280);
    });

    it('高度有最小值限制（100）', () => {
      useUIStore.getState().setRecentOutputHeight(30);
      expect(useUIStore.getState().recentOutputHeight).toBe(100);
    });

    it('高度有最大值限制（400）', () => {
      useUIStore.getState().setRecentOutputHeight(600);
      expect(useUIStore.getState().recentOutputHeight).toBe(400);
    });

    it('高度在有效范围内正常设置', () => {
      useUIStore.getState().setRecentOutputHeight(180);
      expect(useUIStore.getState().recentOutputHeight).toBe(180);

      useUIStore.getState().setRecentOutputHeight(350);
      expect(useUIStore.getState().recentOutputHeight).toBe(350);
    });
  });

  describe('resetPanels', () => {
    it('resetPanels 重置调试面板折叠状态', () => {
      // 设置一些非默认状态
      useUIStore.setState({
        isContextVariablesCollapsed: true,
        isExecutionStackCollapsed: true,
        isRecentOutputCollapsed: true,
        executionStackHeight: 300,
        recentOutputHeight: 350,
      });

      // 调用 resetPanels
      useUIStore.getState().resetPanels();

      // 验证调试面板状态被重置
      const state = useUIStore.getState();
      expect(state.isContextVariablesCollapsed).toBe(false);
      expect(state.isExecutionStackCollapsed).toBe(false);
      expect(state.isRecentOutputCollapsed).toBe(false);
      expect(state.executionStackHeight).toBe(192);
      expect(state.recentOutputHeight).toBe(192);
    });

    it('resetPanels 同时重置左侧栏面板状态', () => {
      // 设置左侧栏和调试面板为非默认状态
      useUIStore.setState({
        isFlowListCollapsed: true,
        isNodeLibraryCollapsed: true,
        isVariablePanelCollapsed: true,
        isContextVariablesCollapsed: true,
        isExecutionStackCollapsed: true,
        isRecentOutputCollapsed: true,
      });

      useUIStore.getState().resetPanels();

      // 验证所有面板状态都被重置
      const state = useUIStore.getState();
      expect(state.isFlowListCollapsed).toBe(false);
      expect(state.isNodeLibraryCollapsed).toBe(false);
      expect(state.isVariablePanelCollapsed).toBe(false);
      expect(state.isContextVariablesCollapsed).toBe(false);
      expect(state.isExecutionStackCollapsed).toBe(false);
      expect(state.isRecentOutputCollapsed).toBe(false);
    });
  });

  describe('独立操作', () => {
    it('折叠上下文变量不影响其他面板', () => {
      useUIStore.setState({
        isExecutionStackCollapsed: false,
        isRecentOutputCollapsed: false,
      });

      useUIStore.getState().toggleContextVariablesCollapsed();

      const state = useUIStore.getState();
      expect(state.isContextVariablesCollapsed).toBe(true);
      expect(state.isExecutionStackCollapsed).toBe(false);
      expect(state.isRecentOutputCollapsed).toBe(false);
    });

    it('折叠执行堆栈不影响其他面板', () => {
      useUIStore.setState({
        isContextVariablesCollapsed: false,
        isRecentOutputCollapsed: false,
      });

      useUIStore.getState().toggleExecutionStackCollapsed();

      const state = useUIStore.getState();
      expect(state.isContextVariablesCollapsed).toBe(false);
      expect(state.isExecutionStackCollapsed).toBe(true);
      expect(state.isRecentOutputCollapsed).toBe(false);
    });

    it('设置执行堆栈高度不影响最近节点输出高度', () => {
      useUIStore.setState({
        recentOutputHeight: 250,
      });

      useUIStore.getState().setExecutionStackHeight(300);

      const state = useUIStore.getState();
      expect(state.executionStackHeight).toBe(300);
      expect(state.recentOutputHeight).toBe(250);
    });
  });
});
