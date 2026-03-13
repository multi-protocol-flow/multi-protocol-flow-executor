/**
 * UI 状态管理
 */
import { create } from 'zustand';

/** 面板类型 */
export type PanelType = 'flowList' | 'history' | 'config' | 'execution';

/** 右侧配置面板模式 */
export type ConfigPanelMode = 'node' | 'settings' | 'debug';
/** 底部面板 Tab 类型 */
export type BottomPanelTab = 'history' | 'console';

/** 确认对话框配置 */
export interface ConfirmDialogConfig {
  /** 对话框标题 */
  title: string;
  /** 对话框描述 */
  description?: string;
  /** 确认按钮文本 */
  confirmText?: string;
  /** 确认回调 */
  onConfirm: () => void | Promise<void>;
}

interface UIState {
  // ===== 面板状态 =====
  /** 流程列表面板是否展开 */
  isFlowListPanelOpen: boolean;
  /** 历史面板是否展开 */
  isHistoryPanelOpen: boolean;
  /** 配置面板是否展开 */
  isConfigPanelOpen: boolean;
  /** 执行结果面板是否展开 */
  isExecutionPanelOpen: boolean;
  /** 底部面板当前 Tab */
  bottomPanelTab: BottomPanelTab;

  // ===== 左侧栏面板折叠状态 =====
  /** 流程列表是否折叠 */
  isFlowListCollapsed: boolean;
  /** 节点库是否折叠 */
  isNodeLibraryCollapsed: boolean;
  /** 变量面板是否折叠 */
  isVariablePanelCollapsed: boolean;
  /** 节点库高度（像素） */
  nodeLibraryHeight: number;
  /** 变量面板高度（像素） */
  variablePanelHeight: number;

  // ===== 调试面板折叠状态 =====
  /** 上下文变量是否折叠 */
  isContextVariablesCollapsed: boolean;
  /** 执行堆栈是否折叠 */
  isExecutionStackCollapsed: boolean;
  /** WebSocket 消息流是否折叠 */
  isWsStreamCollapsed: boolean;
  /** 执行堆栈高度（像素） */
  executionStackHeight: number;
  /** WebSocket 消息流高度（像素） */
  wsStreamHeight: number;

  // ===== 模态框状态 =====
  /** 是否显示新建流程对话框 */
  showNewFlowDialog: boolean;
  /** 是否显示删除确认对话框 */
  showDeleteConfirmDialog: boolean;
  /** 待删除的流程 UUID（用于删除确认） */
  pendingDeleteFlowUuid: string | null;
  /** 是否显示设置对话框 */
  showSettingsDialog: boolean;
  /** 右侧配置面板模式（节点配置/全局设置） */
  configPanelMode: ConfigPanelMode;
  /** 是否显示压力测试配置对话框 */
  showStressTestConfigDialog: boolean;

  // ===== 通用确认对话框状态 =====
  /** 是否显示通用确认对话框 */
  showConfirmDialog: boolean;
  /** 确认对话框配置 */
  confirmDialogConfig: ConfirmDialogConfig | null;
  // ===== 其他 UI 状态 =====
  /** 侧边栏宽度（像素） */
  sidebarWidth: number;
  /** 配置面板宽度（像素） */
  configPanelWidth: number;
  /** 底部面板高度（像素） */
  bottomPanelHeight: number;
  /** 画布缩放比例 */
  canvasZoom: number;

  // ===== Actions =====
  /** 切换流程列表面板 */
  toggleFlowListPanel: () => void;
  /** 切换历史面板 */
  toggleHistoryPanel: () => void;
  /** 切换配置面板 */
  toggleConfigPanel: () => void;
  /** 切换执行结果面板 */
  toggleExecutionPanel: () => void;
  /** 设置配置面板状态 */
  setConfigPanelOpen: (open: boolean) => void;
  /** 设置执行结果面板状态 */
  setExecutionPanelOpen: (open: boolean) => void;
  /** 设置流程列表面板状态 */
  setFlowListPanelOpen: (open: boolean) => void;
  /** 设置底部面板 Tab */
  setBottomPanelTab: (tab: BottomPanelTab) => void;

  // ===== 模态框 Actions =====
  /** 显示新建流程对话框 */
  openNewFlowDialog: () => void;
  /** 关闭新建流程对话框 */
  closeNewFlowDialog: () => void;
  /** 显示删除确认对话框 */
  openDeleteConfirmDialog: (flowUuid: string) => void;
  /** 关闭删除确认对话框 */
  closeDeleteConfirmDialog: () => void;
  /** 显示设置对话框 */
  openSettingsDialog: () => void;
  /** 关闭设置对话框 */
  closeSettingsDialog: () => void;
  /** 设置右侧面板模式 */
  setConfigPanelMode: (mode: ConfigPanelMode) => void;
  /** 打开压力测试配置对话框 */
  openStressTestConfigDialog: () => void;
  /** 关闭压力测试配置对话框 */
  closeStressTestConfigDialog: () => void;

  // ===== 通用确认对话框 Actions =====
  /** 显示确认对话框 */
  showConfirm: (config: ConfirmDialogConfig) => void;
  /** 关闭确认对话框 */
  hideConfirm: () => void;
  // ===== 左侧栏面板 Actions =====
  /** 切换流程列表折叠状态 */
  toggleFlowListCollapsed: () => void;
  /** 切换节点库折叠状态 */
  toggleNodeLibraryCollapsed: () => void;
  /** 切换变量面板折叠状态 */
  toggleVariablePanelCollapsed: () => void;
  /** 设置节点库高度 */
  setNodeLibraryHeight: (height: number) => void;
  /** 设置变量面板高度 */
  setVariablePanelHeight: (height: number) => void;

  // ===== 调试面板 Actions =====
  /** 切换上下文变量折叠状态 */
  toggleContextVariablesCollapsed: () => void;
  /** 切换执行堆栈折叠状态 */
  toggleExecutionStackCollapsed: () => void;
  /** 切换 WebSocket 消息流折叠状态 */
  toggleWsStreamCollapsed: () => void;
  /** 设置执行堆栈高度 */
  setExecutionStackHeight: (height: number) => void;
  /** 设置 WebSocket 消息流高度 */
  setWsStreamHeight: (height: number) => void;

  // ===== 布局 Actions =====
  /** 设置侧边栏宽度 */
  setSidebarWidth: (width: number) => void;
  /** 设置配置面板宽度 */
  setConfigPanelWidth: (width: number) => void;
  /** 设置底部面板高度 */
  setBottomPanelHeight: (height: number) => void;
  /** 设置画布缩放 */
  setCanvasZoom: (zoom: number) => void;

  // ===== 工具方法 =====
  /** 重置所有面板状态 */
  resetPanels: () => void;
}

const MIN_SIDEBAR_WIDTH = 200;
const MAX_SIDEBAR_WIDTH = 400;
const MIN_CONFIG_WIDTH = 250;
const MAX_CONFIG_WIDTH = 500;
const MIN_BOTTOM_PANEL_HEIGHT = 100;
const MAX_BOTTOM_PANEL_HEIGHT = 500;
const MIN_LEFT_PANEL_HEIGHT = 100;
const MAX_LEFT_PANEL_HEIGHT = 400;

const initialState = {
  // 面板状态
  isFlowListPanelOpen: true,
  isHistoryPanelOpen: false,
  isConfigPanelOpen: true,
  isExecutionPanelOpen: false,
  // 底部面板 Tab
  bottomPanelTab: 'history' as BottomPanelTab,

  // 左侧栏面板折叠状态
  isFlowListCollapsed: false,
  isNodeLibraryCollapsed: false,
  isVariablePanelCollapsed: false,
  nodeLibraryHeight: 200,
  variablePanelHeight: 200,

  // 调试面板折叠状态
  isContextVariablesCollapsed: false,
  isExecutionStackCollapsed: false,
  isWsStreamCollapsed: false,
  executionStackHeight: 192,
  wsStreamHeight: 200,

  // 模态框状态
  showNewFlowDialog: false,
  showDeleteConfirmDialog: false,
  pendingDeleteFlowUuid: null,
  showSettingsDialog: false,
  configPanelMode: 'node' as ConfigPanelMode,
  showStressTestConfigDialog: false,
  // 通用确认对话框状态
  showConfirmDialog: false,
  confirmDialogConfig: null,
  sidebarWidth: 280,
  configPanelWidth: 320,
  bottomPanelHeight: 208,
  canvasZoom: 1,
};

export const useUIStore = create<UIState>((set) => ({
  ...initialState,

  // ===== 面板 Actions =====
  toggleFlowListPanel: () =>
    set((state) => ({
      isFlowListPanelOpen: !state.isFlowListPanelOpen,
    })),

  toggleHistoryPanel: () =>
    set((state) => ({
      isHistoryPanelOpen: !state.isHistoryPanelOpen,
    })),

  toggleConfigPanel: () =>
    set((state) => ({
      isConfigPanelOpen: !state.isConfigPanelOpen,
    })),

  toggleExecutionPanel: () =>
    set((state) => ({
      isExecutionPanelOpen: !state.isExecutionPanelOpen,
    })),

  setConfigPanelOpen: (isConfigPanelOpen) => set({ isConfigPanelOpen }),

  setExecutionPanelOpen: (isExecutionPanelOpen) => set({ isExecutionPanelOpen }),

  setFlowListPanelOpen: (isFlowListPanelOpen) => set({ isFlowListPanelOpen }),

  setBottomPanelTab: (bottomPanelTab) => set({ bottomPanelTab }),

  // ===== 模态框 Actions =====
  openNewFlowDialog: () => set({ showNewFlowDialog: true }),

  closeNewFlowDialog: () => set({ showNewFlowDialog: false }),

  openDeleteConfirmDialog: (flowUuid: string) =>
    set({
      showDeleteConfirmDialog: true,
      pendingDeleteFlowUuid: flowUuid,
    }),

  closeDeleteConfirmDialog: () =>
    set({
      showDeleteConfirmDialog: false,
      pendingDeleteFlowUuid: null,
    }),

  openSettingsDialog: () => set({ showSettingsDialog: true, configPanelMode: 'settings', isConfigPanelOpen: true }),

  closeSettingsDialog: () => set({ showSettingsDialog: false, configPanelMode: 'node' }),

  // ===== 通用确认对话框 Actions =====
  showConfirm: (config: ConfirmDialogConfig) =>
    set({
      showConfirmDialog: true,
      confirmDialogConfig: config,
    }),

  hideConfirm: () =>
    set({
      showConfirmDialog: false,
      confirmDialogConfig: null,
    }),

  setConfigPanelMode: (configPanelMode) => set({ configPanelMode }),

  openStressTestConfigDialog: () => set({ showStressTestConfigDialog: true }),

  closeStressTestConfigDialog: () => set({ showStressTestConfigDialog: false }),

  // ===== 左侧栏面板 Actions =====
  toggleFlowListCollapsed: () =>
    set((state) => ({
      isFlowListCollapsed: !state.isFlowListCollapsed,
    })),

  toggleNodeLibraryCollapsed: () =>
    set((state) => ({
      isNodeLibraryCollapsed: !state.isNodeLibraryCollapsed,
    })),

  toggleVariablePanelCollapsed: () =>
    set((state) => ({
      isVariablePanelCollapsed: !state.isVariablePanelCollapsed,
    })),

  setNodeLibraryHeight: (height) =>
    set({
      nodeLibraryHeight: Math.max(MIN_LEFT_PANEL_HEIGHT, Math.min(MAX_LEFT_PANEL_HEIGHT, height)),
    }),

  setVariablePanelHeight: (height) =>
    set({
      variablePanelHeight: Math.max(MIN_LEFT_PANEL_HEIGHT, Math.min(MAX_LEFT_PANEL_HEIGHT, height)),
    }),

  // ===== 调试面板 Actions =====
  toggleContextVariablesCollapsed: () =>
    set((state) => ({
      isContextVariablesCollapsed: !state.isContextVariablesCollapsed,
    })),

  toggleExecutionStackCollapsed: () =>
    set((state) => ({
      isExecutionStackCollapsed: !state.isExecutionStackCollapsed,
    })),

  toggleWsStreamCollapsed: () =>
    set((state) => ({
      isWsStreamCollapsed: !state.isWsStreamCollapsed,
    })),

  setExecutionStackHeight: (height) =>
    set({
      executionStackHeight: Math.max(MIN_LEFT_PANEL_HEIGHT, Math.min(MAX_LEFT_PANEL_HEIGHT, height)),
    }),

  setWsStreamHeight: (height) =>
    set({
      wsStreamHeight: Math.max(MIN_LEFT_PANEL_HEIGHT, Math.min(MAX_LEFT_PANEL_HEIGHT, height)),
    }),

  // ===== 布局 Actions =====
  setSidebarWidth: (width) =>
    set({
      sidebarWidth: Math.max(MIN_SIDEBAR_WIDTH, Math.min(MAX_SIDEBAR_WIDTH, width)),
    }),

  setConfigPanelWidth: (width) =>
    set({
      configPanelWidth: Math.max(MIN_CONFIG_WIDTH, Math.min(MAX_CONFIG_WIDTH, width)),
    }),

  setBottomPanelHeight: (height) =>
    set({
      bottomPanelHeight: Math.max(MIN_BOTTOM_PANEL_HEIGHT, Math.min(MAX_BOTTOM_PANEL_HEIGHT, height)),
    }),

  setCanvasZoom: (zoom) =>
    set({
      canvasZoom: Math.max(0.25, Math.min(2, zoom)),
    }),

  // ===== 工具方法 =====
  resetPanels: () =>
    set({
      isFlowListPanelOpen: true,
      isHistoryPanelOpen: false,
      isConfigPanelOpen: true,
      isExecutionPanelOpen: false,
      bottomPanelTab: 'history' as BottomPanelTab,
      isFlowListCollapsed: false,
      isNodeLibraryCollapsed: false,
      isVariablePanelCollapsed: false,
      nodeLibraryHeight: 200,
      variablePanelHeight: 200,
      isContextVariablesCollapsed: false,
      isExecutionStackCollapsed: false,
      isWsStreamCollapsed: false,
      executionStackHeight: 192,
      wsStreamHeight: 200,
    }),
}));

// ===== Selectors =====
/** 选择流程列表面板状态 */
export const selectIsFlowListPanelOpen = (state: UIState) =>
  state.isFlowListPanelOpen;
/** 选择历史面板状态 */
export const selectIsHistoryPanelOpen = (state: UIState) =>
  state.isHistoryPanelOpen;
/** 选择配置面板状态 */
export const selectIsConfigPanelOpen = (state: UIState) =>
  state.isConfigPanelOpen;
/** 选择执行结果面板状态 */
export const selectIsExecutionPanelOpen = (state: UIState) =>
  state.isExecutionPanelOpen;
/** 选择新建流程对话框状态 */
export const selectShowNewFlowDialog = (state: UIState) =>
  state.showNewFlowDialog;
/** 选择删除确认对话框状态 */
export const selectShowDeleteConfirmDialog = (state: UIState) =>
  state.showDeleteConfirmDialog;
/** 选择设置对话框状态 */
export const selectShowSettingsDialog = (state: UIState) =>
  state.showSettingsDialog;

/** 选择底部面板 Tab */
export const selectBottomPanelTab = (state: UIState) =>
  state.bottomPanelTab;
