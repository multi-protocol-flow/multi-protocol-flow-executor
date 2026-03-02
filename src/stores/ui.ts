/**
 * UI 状态管理
 */
import { create } from 'zustand';

/** 面板类型 */
export type PanelType = 'flowList' | 'history' | 'config' | 'execution';

/** 底部面板 Tab 类型 */
export type BottomPanelTab = 'history' | 'console';

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

  // ===== 模态框状态 =====
  /** 是否显示新建流程对话框 */
  showNewFlowDialog: boolean;
  /** 是否显示删除确认对话框 */
  showDeleteConfirmDialog: boolean;
  /** 待删除的流程 UUID（用于删除确认） */
  pendingDeleteFlowUuid: string | null;
  /** 是否显示设置对话框 */
  showSettingsDialog: boolean;

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

const initialState = {
  // 面板状态
  isFlowListPanelOpen: true,
  isHistoryPanelOpen: false,
  isConfigPanelOpen: true,
  isExecutionPanelOpen: false,
  // 底部面板 Tab
  bottomPanelTab: 'history' as BottomPanelTab,

  // 模态框状态
  showNewFlowDialog: false,
  showDeleteConfirmDialog: false,
  pendingDeleteFlowUuid: null,
  showSettingsDialog: false,
  // 布局状态
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

  openSettingsDialog: () => set({ showSettingsDialog: true }),

  closeSettingsDialog: () => set({ showSettingsDialog: false }),

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
