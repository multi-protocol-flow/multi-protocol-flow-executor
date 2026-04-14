/**
 * UI Store - 面板状态、模态框、布局测试
 */
import { describe, it, expect, beforeEach } from 'vitest';
import {
  useUIStore,
  selectIsFlowListPanelOpen,
  selectIsHistoryPanelOpen,
  selectIsConfigPanelOpen,
  selectIsExecutionPanelOpen,
  selectShowNewFlowDialog,
  selectShowDeleteConfirmDialog,
  selectBottomPanelTab,
} from '@/stores/ui';


describe('UIStore - 面板切换', () => {
  beforeEach(() => {
    useUIStore.setState({
      isFlowListPanelOpen: true,
      isHistoryPanelOpen: false,
      isConfigPanelOpen: true,
      isExecutionPanelOpen: false,
      bottomPanelTab: 'history',
    });
  });

  describe('toggleFlowListPanel', () => {
    it('切换流程列表面板状态', () => {
      expect(useUIStore.getState().isFlowListPanelOpen).toBe(true);

      useUIStore.getState().toggleFlowListPanel();
      expect(useUIStore.getState().isFlowListPanelOpen).toBe(false);

      useUIStore.getState().toggleFlowListPanel();
      expect(useUIStore.getState().isFlowListPanelOpen).toBe(true);
    });
  });

  describe('toggleHistoryPanel', () => {
    it('切换历史面板状态', () => {
      expect(useUIStore.getState().isHistoryPanelOpen).toBe(false);

      useUIStore.getState().toggleHistoryPanel();
      expect(useUIStore.getState().isHistoryPanelOpen).toBe(true);

      useUIStore.getState().toggleHistoryPanel();
      expect(useUIStore.getState().isHistoryPanelOpen).toBe(false);
    });
  });

  describe('toggleConfigPanel', () => {
    it('切换配置面板状态', () => {
      expect(useUIStore.getState().isConfigPanelOpen).toBe(true);

      useUIStore.getState().toggleConfigPanel();
      expect(useUIStore.getState().isConfigPanelOpen).toBe(false);

      useUIStore.getState().toggleConfigPanel();
      expect(useUIStore.getState().isConfigPanelOpen).toBe(true);
    });
  });

  describe('toggleExecutionPanel', () => {
    it('切换执行结果面板状态', () => {
      expect(useUIStore.getState().isExecutionPanelOpen).toBe(false);

      useUIStore.getState().toggleExecutionPanel();
      expect(useUIStore.getState().isExecutionPanelOpen).toBe(true);

      useUIStore.getState().toggleExecutionPanel();
      expect(useUIStore.getState().isExecutionPanelOpen).toBe(false);
    });
  });

  describe('setConfigPanelOpen', () => {
    it('直接设置配置面板状态为 true', () => {
      useUIStore.setState({ isConfigPanelOpen: false });
      useUIStore.getState().setConfigPanelOpen(true);
      expect(useUIStore.getState().isConfigPanelOpen).toBe(true);
    });

    it('直接设置配置面板状态为 false', () => {
      useUIStore.getState().setConfigPanelOpen(false);
      expect(useUIStore.getState().isConfigPanelOpen).toBe(false);
    });
  });

  describe('setExecutionPanelOpen', () => {
    it('直接设置执行结果面板状态为 true', () => {
      useUIStore.getState().setExecutionPanelOpen(true);
      expect(useUIStore.getState().isExecutionPanelOpen).toBe(true);
    });

    it('直接设置执行结果面板状态为 false', () => {
      useUIStore.getState().setExecutionPanelOpen(true);
      useUIStore.getState().setExecutionPanelOpen(false);
      expect(useUIStore.getState().isExecutionPanelOpen).toBe(false);
    });
  });

  describe('setFlowListPanelOpen', () => {
    it('直接设置流程列表面板状态为 true', () => {
      useUIStore.setState({ isFlowListPanelOpen: false });
      useUIStore.getState().setFlowListPanelOpen(true);
      expect(useUIStore.getState().isFlowListPanelOpen).toBe(true);
    });

    it('直接设置流程列表面板状态为 false', () => {
      useUIStore.getState().setFlowListPanelOpen(false);
      expect(useUIStore.getState().isFlowListPanelOpen).toBe(false);
    });
  });

  describe('setBottomPanelTab', () => {
    it('设置底部面板 Tab 为 history', () => {
      useUIStore.setState({ bottomPanelTab: 'console' });
      useUIStore.getState().setBottomPanelTab('history');
      expect(useUIStore.getState().bottomPanelTab).toBe('history');
    });

    it('设置底部面板 Tab 为 console', () => {
      useUIStore.getState().setBottomPanelTab('console');
      expect(useUIStore.getState().bottomPanelTab).toBe('console');
    });
  });

  describe('Selectors', () => {
    it('selectIsFlowListPanelOpen 返回正确值', () => {
      const state = useUIStore.getState();
      expect(selectIsFlowListPanelOpen(state)).toBe(true);
    });

    it('selectIsHistoryPanelOpen 返回正确值', () => {
      const state = useUIStore.getState();
      expect(selectIsHistoryPanelOpen(state)).toBe(false);
    });

    it('selectIsConfigPanelOpen 返回正确值', () => {
      const state = useUIStore.getState();
      expect(selectIsConfigPanelOpen(state)).toBe(true);
    });

    it('selectIsExecutionPanelOpen 返回正确值', () => {
      const state = useUIStore.getState();
      expect(selectIsExecutionPanelOpen(state)).toBe(false);
    });

    it('selectBottomPanelTab 返回正确值', () => {
      const state = useUIStore.getState();
      expect(selectBottomPanelTab(state)).toBe('history');
    });
  });
});

describe('UIStore - 模态框', () => {
  beforeEach(() => {
    useUIStore.setState({
      showNewFlowDialog: false,
      showDeleteConfirmDialog: false,
      pendingDeleteFlowUuid: null,
      showConfirmDialog: false,
      confirmDialogConfig: null,
    });
  });

  describe('openNewFlowDialog', () => {
    it('显示新建流程对话框', () => {
      useUIStore.getState().openNewFlowDialog();
      expect(useUIStore.getState().showNewFlowDialog).toBe(true);
    });
  });

  describe('closeNewFlowDialog', () => {
    it('关闭新建流程对话框', () => {
      useUIStore.getState().openNewFlowDialog();
      expect(useUIStore.getState().showNewFlowDialog).toBe(true);

      useUIStore.getState().closeNewFlowDialog();
      expect(useUIStore.getState().showNewFlowDialog).toBe(false);
    });
  });

  describe('openDeleteConfirmDialog', () => {
    it('显示删除确认对话框并设置待删除 UUID', () => {
      const flowUuid = 'test-flow-uuid-123';
      useUIStore.getState().openDeleteConfirmDialog(flowUuid);

      const state = useUIStore.getState();
      expect(state.showDeleteConfirmDialog).toBe(true);
      expect(state.pendingDeleteFlowUuid).toBe(flowUuid);
    });

    it('多次调用更新待删除 UUID', () => {
      useUIStore.getState().openDeleteConfirmDialog('uuid-1');
      expect(useUIStore.getState().pendingDeleteFlowUuid).toBe('uuid-1');

      useUIStore.getState().openDeleteConfirmDialog('uuid-2');
      expect(useUIStore.getState().pendingDeleteFlowUuid).toBe('uuid-2');
    });
  });

  describe('closeDeleteConfirmDialog', () => {
    it('关闭删除确认对话框并清空待删除 UUID', () => {
      useUIStore.getState().openDeleteConfirmDialog('test-uuid');
      expect(useUIStore.getState().showDeleteConfirmDialog).toBe(true);

      useUIStore.getState().closeDeleteConfirmDialog();

      const state = useUIStore.getState();
      expect(state.showDeleteConfirmDialog).toBe(false);
      expect(state.pendingDeleteFlowUuid).toBe(null);
    });
  });

  describe('showConfirm', () => {
    it('显示通用确认对话框并设置配置', () => {
      const config = {
        title: '确认删除',
        description: '此操作不可撤销',
        confirmText: '删除',
        onConfirm: () => {},
      };

      useUIStore.getState().showConfirm(config);

      const state = useUIStore.getState();
      expect(state.showConfirmDialog).toBe(true);
      expect(state.confirmDialogConfig).toEqual(config);
    });
  });

  describe('hideConfirm', () => {
    it('隐藏通用确认对话框并清空配置', () => {
      useUIStore.getState().showConfirm({
        title: '测试',
        onConfirm: () => {},
      });
      expect(useUIStore.getState().showConfirmDialog).toBe(true);

      useUIStore.getState().hideConfirm();

      const state = useUIStore.getState();
      expect(state.showConfirmDialog).toBe(false);
      expect(state.confirmDialogConfig).toBe(null);
    });
  });

  describe('Selectors', () => {
    it('selectShowNewFlowDialog 返回正确值', () => {
      const state = useUIStore.getState();
      expect(selectShowNewFlowDialog(state)).toBe(false);

      useUIStore.getState().openNewFlowDialog();
      expect(selectShowNewFlowDialog(useUIStore.getState())).toBe(true);
    });

    it('selectShowDeleteConfirmDialog 返回正确值', () => {
      const state = useUIStore.getState();
      expect(selectShowDeleteConfirmDialog(state)).toBe(false);

      useUIStore.getState().openDeleteConfirmDialog('uuid');
      expect(selectShowDeleteConfirmDialog(useUIStore.getState())).toBe(true);
    });
  });
});

describe('UIStore - 左侧栏面板', () => {
  beforeEach(() => {
    useUIStore.setState({
      isFlowListCollapsed: false,
    });
  });

  describe('toggleFlowListCollapsed', () => {
    it('切换流程列表折叠状态', () => {
      expect(useUIStore.getState().isFlowListCollapsed).toBe(false);

      useUIStore.getState().toggleFlowListCollapsed();
      expect(useUIStore.getState().isFlowListCollapsed).toBe(true);

      useUIStore.getState().toggleFlowListCollapsed();
      expect(useUIStore.getState().isFlowListCollapsed).toBe(false);
    });
  });

  describe('toggleVariableDrawer', () => {
    it('切换变量抽屉状态', () => {
      expect(useUIStore.getState().isVariableDrawerOpen).toBe(false);

      useUIStore.getState().toggleVariableDrawer();
      expect(useUIStore.getState().isVariableDrawerOpen).toBe(true);

      useUIStore.getState().toggleVariableDrawer();
      expect(useUIStore.getState().isVariableDrawerOpen).toBe(false);
    });
  });
});

describe('UIStore - 布局', () => {
  beforeEach(() => {
    useUIStore.setState({
      sidebarWidth: 280,
      configPanelWidth: 320,
      bottomPanelHeight: 208,
      canvasZoom: 1,
    });
  });

  describe('setSidebarWidth', () => {
    it('设置侧边栏宽度', () => {
      useUIStore.getState().setSidebarWidth(300);
      expect(useUIStore.getState().sidebarWidth).toBe(300);
    });

    it('宽度有最小值限制（200）', () => {
      useUIStore.getState().setSidebarWidth(100);
      expect(useUIStore.getState().sidebarWidth).toBe(200);
    });

    it('宽度有最大值限制（400）', () => {
      useUIStore.getState().setSidebarWidth(500);
      expect(useUIStore.getState().sidebarWidth).toBe(400);
    });

    it('宽度在有效范围内正常设置', () => {
      useUIStore.getState().setSidebarWidth(250);
      expect(useUIStore.getState().sidebarWidth).toBe(250);

      useUIStore.getState().setSidebarWidth(350);
      expect(useUIStore.getState().sidebarWidth).toBe(350);
    });
  });

  describe('setConfigPanelWidth', () => {
    it('设置配置面板宽度', () => {
      useUIStore.getState().setConfigPanelWidth(400);
      expect(useUIStore.getState().configPanelWidth).toBe(400);
    });

    it('宽度有最小值限制（250）', () => {
      useUIStore.getState().setConfigPanelWidth(200);
      expect(useUIStore.getState().configPanelWidth).toBe(250);
    });

    it('宽度有最大值限制（500）', () => {
      useUIStore.getState().setConfigPanelWidth(600);
      expect(useUIStore.getState().configPanelWidth).toBe(500);
    });

    it('宽度在有效范围内正常设置', () => {
      useUIStore.getState().setConfigPanelWidth(300);
      expect(useUIStore.getState().configPanelWidth).toBe(300);

      useUIStore.getState().setConfigPanelWidth(450);
      expect(useUIStore.getState().configPanelWidth).toBe(450);
    });
  });

  describe('setBottomPanelHeight', () => {
    it('设置底部面板高度', () => {
      useUIStore.getState().setBottomPanelHeight(300);
      expect(useUIStore.getState().bottomPanelHeight).toBe(300);
    });

    it('高度有最小值限制（100）', () => {
      useUIStore.getState().setBottomPanelHeight(50);
      expect(useUIStore.getState().bottomPanelHeight).toBe(100);
    });

    it('高度有最大值限制（700）', () => {
      useUIStore.getState().setBottomPanelHeight(800);
      expect(useUIStore.getState().bottomPanelHeight).toBe(700);
    });

    it('高度在有效范围内正常设置', () => {
      useUIStore.getState().setBottomPanelHeight(150);
      expect(useUIStore.getState().bottomPanelHeight).toBe(150);

      useUIStore.getState().setBottomPanelHeight(400);
      expect(useUIStore.getState().bottomPanelHeight).toBe(400);
    });
  });

  describe('setCanvasZoom', () => {
    it('设置画布缩放比例', () => {
      useUIStore.getState().setCanvasZoom(0.5);
      expect(useUIStore.getState().canvasZoom).toBe(0.5);
    });

    it('缩放有最小值限制（0.25）', () => {
      useUIStore.getState().setCanvasZoom(0.1);
      expect(useUIStore.getState().canvasZoom).toBe(0.25);
    });

    it('缩放有最大值限制（2）', () => {
      useUIStore.getState().setCanvasZoom(3);
      expect(useUIStore.getState().canvasZoom).toBe(2);
    });

    it('缩放在有效范围内正常设置', () => {
      useUIStore.getState().setCanvasZoom(0.5);
      expect(useUIStore.getState().canvasZoom).toBe(0.5);

      useUIStore.getState().setCanvasZoom(1.5);
      expect(useUIStore.getState().canvasZoom).toBe(1.5);

      useUIStore.getState().setCanvasZoom(2);
      expect(useUIStore.getState().canvasZoom).toBe(2);
    });
  });
});

describe('UIStore - 配置面板模式', () => {
  beforeEach(() => {
    useUIStore.setState({
      configPanelMode: 'node',
    });
  });

  describe('setConfigPanelMode', () => {
    it('设置配置面板模式为 node', () => {
      useUIStore.setState({ configPanelMode: 'settings' });
      useUIStore.getState().setConfigPanelMode('node');
      expect(useUIStore.getState().configPanelMode).toBe('node');
    });

    it('设置配置面板模式为 settings', () => {
      useUIStore.getState().setConfigPanelMode('settings');
      expect(useUIStore.getState().configPanelMode).toBe('settings');
    });

    it('设置配置面板模式为 debug', () => {
      useUIStore.getState().setConfigPanelMode('debug');
      expect(useUIStore.getState().configPanelMode).toBe('debug');
    });
  });
});

describe('UIStore - resetPanels 完整测试', () => {
  beforeEach(() => {
    // 设置所有面板为非默认状态
    useUIStore.setState({
      isFlowListPanelOpen: false,
      isHistoryPanelOpen: true,
      isConfigPanelOpen: false,
      isExecutionPanelOpen: true,
      bottomPanelTab: 'console',
      isFlowListCollapsed: true,
    });
  });

  it('resetPanels 重置所有面板状态到初始值', () => {
    useUIStore.getState().resetPanels();

    const state = useUIStore.getState();
    // 主面板状态
    expect(state.isFlowListPanelOpen).toBe(true);
    expect(state.isHistoryPanelOpen).toBe(false);
    expect(state.isConfigPanelOpen).toBe(true);
    expect(state.isExecutionPanelOpen).toBe(false);
    expect(state.bottomPanelTab).toBe('history');

    // 左侧栏面板状态
    expect(state.isFlowListCollapsed).toBe(false);
    expect(state.isVariableDrawerOpen).toBe(false);
  });

  it('resetPanels 不影响布局尺寸', () => {
    useUIStore.setState({
      sidebarWidth: 350,
      configPanelWidth: 400,
      bottomPanelHeight: 300,
      canvasZoom: 0.5,
    });

    useUIStore.getState().resetPanels();

    const state = useUIStore.getState();
    // 布局尺寸不应被 resetPanels 重置
    expect(state.sidebarWidth).toBe(350);
    expect(state.configPanelWidth).toBe(400);
    expect(state.bottomPanelHeight).toBe(300);
    expect(state.canvasZoom).toBe(0.5);
  });

  it('resetPanels 不影响模态框状态', () => {
    useUIStore.setState({
      showNewFlowDialog: true,
      showDeleteConfirmDialog: true,
      pendingDeleteFlowUuid: 'test-uuid',
      showSettingsDialog: true,
      showConfirmDialog: true,
    });

    useUIStore.getState().resetPanels();

    const state = useUIStore.getState();
    // 模态框状态不应被 resetPanels 重置
    expect(state.showNewFlowDialog).toBe(true);
    expect(state.showDeleteConfirmDialog).toBe(true);
    expect(state.pendingDeleteFlowUuid).toBe('test-uuid');
    expect(state.showSettingsDialog).toBe(true);
    expect(state.showConfirmDialog).toBe(true);
  });
});
