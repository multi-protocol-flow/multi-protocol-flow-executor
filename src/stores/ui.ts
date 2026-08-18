/**
 * UI 状态管理
 */
import { create } from 'zustand';

/** 面板类型 */
export type PanelType = 'flowList' | 'history' | 'config' | 'execution';

/** 右侧配置面板模式 */
export type ConfigPanelMode = 'node' | 'settings' | 'debug';
/** 底部面板 Tab 类型（空字符串表示未选中） */
export type BottomPanelTab = '' | 'history' | 'console' | 'running';

/** 页面导航类型 */
export type PageType = 'canvas' | 'dataset-reports';

/** 节点选择器位置 */
export interface NodePalettePosition {
  x: number;
  y: number;
}

/** 数据集报告子页面状态 */
export interface DatasetReportPageState {
  /** 当前报告 ID（查看详情时设置） */
  reportId: string | null;
  /** 当前行索引（查看行详情时设置） */
  rowIndex: number | null;
}

/** 确认对话框配置 */
export interface ConfirmDialogConfig {
  /** 对话框标题（支持函数以延迟求值，适配语言切换） */
  title: string | (() => string);
  /** 对话框描述 */
  description?: string | (() => string);
  /** 确认按钮文本 */
  confirmText?: string | (() => string);
  /** 确认回调 */
  onConfirm: () => void | Promise<void>;
  /** 取消/关闭回调 */
  onCancel?: () => void;
}

interface UIState {
  // ===== 面板状态 =====
  /** 流程列表面板是否展开 */
  isFlowListPanelOpen: boolean;
  /** 历史面板是否展开 */
  isHistoryPanelOpen: boolean;
  /** 底部面板内容区是否展开 */
  isBottomPanelExpanded: boolean;
  /** 配置面板是否展开 */
  isConfigPanelOpen: boolean;
  /** 执行结果面板是否展开 */
  isExecutionPanelOpen: boolean;
  /** 底部面板当前 Tab */
  bottomPanelTab: BottomPanelTab;

  // ===== 左侧栏面板折叠状态 =====
  /** 流程列表是否折叠 */
  isFlowListCollapsed: boolean;
  /** 连接面板是否折叠 */
  isConnectionsPanelCollapsed: boolean;
  /** 连接面板高度（像素） */
  connectionsPanelHeight: number;
  /** 变量抽屉是否打开 */
  isVariableDrawerOpen: boolean;

  // ===== 模态框状态 =====
  /** 是否显示新建流程对话框 */
  showNewFlowDialog: boolean;
  /** 新建流程对话框预选的文件夹路径 */
  newFlowDialogFolderPath: string;
  /** 新建流程对话框是否默认打开模板 Tab */
  newFlowDialogDefaultTemplate: boolean;
  /** 是否显示删除确认对话框 */
  showDeleteConfirmDialog: boolean;
  /** 待删除的流程 UUID（用于删除确认） */
  pendingDeleteFlowUuid: string | null;
  /** 是否显示设置对话框 */
  showSettingsDialog: boolean;
  /** 是否显示插件设置对话框 */
  showPluginSettingsDialog: boolean;
  /** 右侧配置面板模式（节点配置/全局设置） */
  configPanelMode: ConfigPanelMode;
  /** 是否显示压力测试配置对话框 */
  showStressTestConfigDialog: boolean;
  /** 是否显示数据驱动执行面板 */
  showDatasetExecutionPanel: boolean;
  /** 是否显示环境管理对话框 */
  showEnvironmentManagerDialog: boolean;
  /** 是否显示快捷键面板 */
  showShortcutsDialog: boolean;
  /** 是否显示搜索面板 */
  showSearchOverlay: boolean;

  // ===== 通用确认对话框状态 =====
  /** 是否显示通用确认对话框 */
  showConfirmDialog: boolean;
  /** 确认对话框配置 */
  confirmDialogConfig: ConfirmDialogConfig | null;
  // ===== 其他 UI 状态 =====
  /** 当前页面 */
  currentPage: PageType;
  /** 数据集报告页面状态 */
  datasetReportPage: DatasetReportPageState;
  /** 侧边栏宽度（像素） */
  sidebarWidth: number;
  /** 配置面板宽度（像素） */
  configPanelWidth: number;
  /** 底部面板高度（像素） */
  bottomPanelHeight: number;
  /** 画布缩放比例 */
  canvasZoom: number;
  /** 小地图是否显示 */
  showMinimap: boolean;

  // ===== 节点选择器状态 =====
  /** 节点选择器是否打开 */
  isNodePaletteOpen: boolean;
  /** 节点选择器弹出位置（null 时使用默认位置） */
  nodePalettePosition: NodePalettePosition | null;

  // ===== Actions =====
  /** 切换流程列表面板 */
  toggleFlowListPanel: () => void;
  /** 切换历史面板 */
  toggleHistoryPanel: () => void;
  /** 切换底部面板展开/收起 */
  toggleBottomPanel: () => void;
  /** 设置底部面板展开状态 */
  setBottomPanelExpanded: (expanded: boolean) => void;
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
  /** 显示新建流程对话框（可指定预选的文件夹路径，以及是否默认打开模板 Tab） */
  openNewFlowDialog: (folderPath?: string, defaultTemplate?: boolean) => void;
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
  /** 显示插件设置对话框 */
  openPluginSettingsDialog: () => void;
  /** 关闭插件设置对话框 */
  closePluginSettingsDialog: () => void;
  /** 设置右侧面板模式 */
  setConfigPanelMode: (mode: ConfigPanelMode) => void;
  /** 打开压力测试配置对话框 */
  openStressTestConfigDialog: () => void;
  /** 关闭压力测试配置对话框 */
  closeStressTestConfigDialog: () => void;
  /** 打开数据驱动执行面板 */
  openDatasetExecutionPanel: () => void;
  /** 关闭数据驱动执行面板 */
  closeDatasetExecutionPanel: () => void;
  /** 打开环境管理对话框 */
  openEnvironmentManagerDialog: () => void;
  /** 关闭环境管理对话框 */
  closeEnvironmentManagerDialog: () => void;
  /** 打开快捷键面板 */
  openShortcutsDialog: () => void;
  /** 关闭快捷键面板 */
  closeShortcutsDialog: () => void;
  /** 打开搜索面板 */
  openSearchOverlay: () => void;
  /** 关闭搜索面板 */
  closeSearchOverlay: () => void;

  // ===== 通用确认对话框 Actions =====
  /** 显示确认对话框 */
  showConfirm: (config: ConfirmDialogConfig) => void;
  /** 关闭确认对话框 */
  hideConfirm: () => void;
  // ===== 左侧栏面板 Actions =====
  /** 切换流程列表折叠状态 */
  toggleFlowListCollapsed: () => void;
  /** 切换连接面板折叠状态 */
  toggleConnectionsPanelCollapsed: () => void;
  /** 设置连接面板高度 */
  setConnectionsPanelHeight: (height: number) => void;
  /** 切换变量抽屉状态 */
  toggleVariableDrawer: () => void;

  // ===== 布局 Actions =====
  /** 设置侧边栏宽度 */
  setSidebarWidth: (width: number) => void;
  /** 设置配置面板宽度 */
  setConfigPanelWidth: (width: number) => void;
  /** 设置底部面板高度 */
  setBottomPanelHeight: (height: number) => void;
  /** 设置画布缩放 */
  setCanvasZoom: (zoom: number) => void;
  /** 切换小地图显隐 */
  toggleMinimap: () => void;

  // ===== 节点选择器 Actions =====
  /** 打开节点选择器（可指定位置，不指定则默认居中） */
  openNodePalette: (position?: NodePalettePosition) => void;
  /** 关闭节点选择器 */
  closeNodePalette: () => void;

  // ===== 欢迎引导状态 =====
  /** 当前引导步骤（0 表示未激活） */
  welcomeStep: number;
  /** 设置引导步骤 */
  setWelcomeStep: (step: number) => void;

  // ===== 工具方法 =====
  /** 重置所有面板状态 */
  resetPanels: () => void;

  // ===== 页面导航 Actions =====
  /** 导航到指定页面 */
  navigateTo: (page: PageType) => void;
  /** 导航到数据集报告列表 */
  navigateToDatasetReports: () => void;
  /** 导航到数据集报告详情 */
  navigateToDatasetReportDetail: (reportId: string) => void;
  /** 导航到数据集行详情 */
  navigateToDatasetRowDetail: (reportId: string, rowIndex: number) => void;
  /** 返回画布 */
  navigateToCanvas: () => void;
}

const MIN_SIDEBAR_WIDTH = 200;
const MAX_SIDEBAR_WIDTH = 400;
const MIN_CONFIG_WIDTH = 250;
const MAX_CONFIG_WIDTH = 500;
const MIN_BOTTOM_PANEL_HEIGHT = 100;
const MAX_BOTTOM_PANEL_HEIGHT = 700;
const MIN_LEFT_PANEL_HEIGHT = 100;
const MAX_LEFT_PANEL_HEIGHT = 400;

const initialState = {
  // 面板状态（启动时流程列表 / 节点列表两个抽屉默认收起）
  isFlowListPanelOpen: false,
  isHistoryPanelOpen: false,
  isBottomPanelExpanded: false,
  isConfigPanelOpen: false,
  isExecutionPanelOpen: false,
  // 底部面板 Tab
      bottomPanelTab: '' as BottomPanelTab,

  // 左侧栏面板折叠状态
  isFlowListCollapsed: false,
  isConnectionsPanelCollapsed: false,
  connectionsPanelHeight: 150,
  isVariableDrawerOpen: false,

  // 模态框状态
  showNewFlowDialog: false,
  newFlowDialogFolderPath: '',
  newFlowDialogDefaultTemplate: false,
  showDeleteConfirmDialog: false,
  pendingDeleteFlowUuid: null,
  showSettingsDialog: false,
  showPluginSettingsDialog: false,
  configPanelMode: 'node' as ConfigPanelMode,
  showStressTestConfigDialog: false,
  showDatasetExecutionPanel: false,
  showEnvironmentManagerDialog: false,
  showShortcutsDialog: false,
  showSearchOverlay: false,

  // 通用确认对话框状态
  showConfirmDialog: false,
  confirmDialogConfig: null,
  // 页面导航状态
  currentPage: 'canvas' as PageType,
  datasetReportPage: {
    reportId: null,
    rowIndex: null,
  },
  sidebarWidth: 280,
  configPanelWidth: 320,
  bottomPanelHeight: 208,
  canvasZoom: 1,
      showMinimap: false,
      // 节点选择器
      isNodePaletteOpen: false,
      nodePalettePosition: null,
      // 欢迎引导
      welcomeStep: 0,
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

  toggleBottomPanel: () =>
    set((state) => ({
      isBottomPanelExpanded: !state.isBottomPanelExpanded,
    })),

  setBottomPanelExpanded: (isBottomPanelExpanded) => set({ isBottomPanelExpanded }),

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
  openNewFlowDialog: (folderPath?: string, defaultTemplate?: boolean) =>
    set({
      showNewFlowDialog: true,
      newFlowDialogFolderPath: folderPath ?? '',
      newFlowDialogDefaultTemplate: defaultTemplate ?? false,
    }),

  closeNewFlowDialog: () => set({ showNewFlowDialog: false, newFlowDialogFolderPath: '' }),

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

  openPluginSettingsDialog: () => set({ showPluginSettingsDialog: true }),

  closePluginSettingsDialog: () => set({ showPluginSettingsDialog: false }),

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

  openDatasetExecutionPanel: () => set({ showDatasetExecutionPanel: true }),
  closeDatasetExecutionPanel: () => set({ showDatasetExecutionPanel: false }),

  openEnvironmentManagerDialog: () => set({ showEnvironmentManagerDialog: true }),
  closeEnvironmentManagerDialog: () => set({ showEnvironmentManagerDialog: false }),

  openShortcutsDialog: () => set({ showShortcutsDialog: true }),
  closeShortcutsDialog: () => set({ showShortcutsDialog: false }),

  openSearchOverlay: () => set({ showSearchOverlay: true }),
  closeSearchOverlay: () => set({ showSearchOverlay: false }),

  // ===== 左侧栏面板 Actions =====
  toggleFlowListCollapsed: () =>
    set((state) => ({
      isFlowListCollapsed: !state.isFlowListCollapsed,
    })),

  toggleConnectionsPanelCollapsed: () =>
    set((state) => ({
      isConnectionsPanelCollapsed: !state.isConnectionsPanelCollapsed,
    })),

  setConnectionsPanelHeight: (height) =>
    set({
      connectionsPanelHeight: Math.max(MIN_LEFT_PANEL_HEIGHT, Math.min(MAX_LEFT_PANEL_HEIGHT, height)),
    }),

  toggleVariableDrawer: () =>
    set((state) => ({
      isVariableDrawerOpen: !state.isVariableDrawerOpen,
    })),

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

  toggleMinimap: () =>
    set((state) => ({
      showMinimap: !state.showMinimap,
    })),

  // ===== 节点选择器 Actions =====
  openNodePalette: (position) =>
    set({
      isNodePaletteOpen: true,
      nodePalettePosition: position ?? null,
    }),

  closeNodePalette: () =>
    set({
      isNodePaletteOpen: false,
      nodePalettePosition: null,
    }),

  // ===== 页面导航 Actions =====
  navigateTo: (page: PageType) => set({ currentPage: page }),

  navigateToDatasetReports: () =>
    set({
      currentPage: 'dataset-reports',
      datasetReportPage: { reportId: null, rowIndex: null },
    }),

  navigateToDatasetReportDetail: (reportId: string) =>
    set({
      currentPage: 'dataset-reports',
      datasetReportPage: { reportId, rowIndex: null },
    }),

  navigateToDatasetRowDetail: (reportId: string, rowIndex: number) =>
    set({
      currentPage: 'dataset-reports',
      datasetReportPage: { reportId, rowIndex },
    }),

  navigateToCanvas: () =>
    set({
      currentPage: 'canvas',
      datasetReportPage: { reportId: null, rowIndex: null },
    }),

  // ===== 欢迎引导 Actions =====
  setWelcomeStep: (welcomeStep) => set({ welcomeStep }),

  // ===== 工具方法 =====
  resetPanels: () =>
    set({
      isFlowListPanelOpen: false,
      isHistoryPanelOpen: false,
      isBottomPanelExpanded: false,
      isConfigPanelOpen: false,
      isExecutionPanelOpen: false,
  bottomPanelTab: '' as BottomPanelTab,
      isFlowListCollapsed: false,
      isVariableDrawerOpen: false,
      showMinimap: false,
      isNodePaletteOpen: false,
      nodePalettePosition: null,
      showPluginSettingsDialog: false,
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
/** 选择插件设置对话框状态 */
export const selectShowPluginSettingsDialog = (state: UIState) =>
  state.showPluginSettingsDialog;

/** 选择底部面板 Tab */
export const selectBottomPanelTab = (state: UIState) =>
  state.bottomPanelTab;

/** 选择欢迎引导步骤 */
export const selectWelcomeStep = (state: UIState) =>
  state.welcomeStep;
