/**
 * UI 状态管理
 */
import { create } from 'zustand';
import type { Flow, FlowMetadata } from '@/types';
import { startLayoutTransition } from '@/lib/layoutTransition';

/** 面板类型 */
export type PanelType = 'flowList' | 'history' | 'config' | 'execution';

/** 右侧配置面板模式 */
export type ConfigPanelMode = 'node' | 'settings' | 'debug';
/** 底部面板 Tab 类型（空字符串表示未选中） */
export type BottomPanelTab = '' | 'history' | 'console' | 'running';

/** 流程标签栏显示模式（单行横向滚动 / 多行折行平铺） */
export type FlowTabDisplayMode = 'scroll' | 'wrap';

/** 页面导航类型 */
export type PageType = 'canvas' | 'dataset-reports';


/** 环境管理器上下文模式：从侧边栏进入为纯全局环境管理，从画布进入为流程变量兜底与环境级联对比 */
export type EnvironmentManagerContext = 'sidebar' | 'canvas';
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

/** 流程 UUID 冲突状态定义 */
export interface FlowConflictState {
  incomingFlow: Flow;
  incomingFilePath?: string;
  conflictingTabUuid: string;
  conflictingTabName: string;
  conflictingFilePath?: string;
  conflictType: 'flow_uuid' | 'node_uuid' | 'both';
  duplicateNodeCount?: number;
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
  /** 定位流程的目标信息（包含 uuid 或 filePath，以及更新时间戳） */
  locateFlowTarget: { uuid?: string; filePath?: string; timestamp: number } | null;

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
  /** 设置对话框激活选项卡 */
  settingsDialogTab?: 'general' | 'storage' | 'ai' | 'engine' | 'network' | 'about' | 'shortcuts';
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
  /** 环境管理器当前上下文（'sidebar' 或 'canvas'） */
  environmentManagerContext: EnvironmentManagerContext;
  /** 环境管理器打开时预置的待删除环境名称（用于从侧边栏直接发起删除） */
  environmentManagerPendingDelete: string | null;
  /** 是否显示流程导出固化对话框 */
  showExportFlowDialog: boolean;
  /** 流程导出固化目标流程 */
  exportFlowTarget: Flow | FlowMetadata | null;
  /** 是否显示快捷键面板 */
  showShortcutsDialog: boolean;
  /** 是否显示搜索面板 */
  showSearchOverlay: boolean;
  /** 是否显示脚本函数帮助对话框 */
  showScriptHelpDialog: boolean;
  /** 是否显示动态变量帮助对话框 */
  showDynamicVariablesHelpDialog: boolean;

  /** 流程 UUID / 节点 UUID 冲突状态（非 null 时展示决策弹窗） */
  flowConflict: FlowConflictState | null;
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
  /** 流程标签栏显示模式 */
  flowTabDisplayMode: FlowTabDisplayMode;

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
  openSettingsDialog: (tab?: 'general' | 'storage' | 'ai' | 'engine' | 'network' | 'about' | 'shortcuts') => void;
  /** 关闭设置对话框 */
  closeSettingsDialog: () => void;
  /** 设置设置对话框激活选项卡 */
  setSettingsDialogTab: (tab: 'general' | 'storage' | 'ai' | 'engine' | 'network' | 'about' | 'shortcuts') => void;
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
  /** 打开环境管理对话框（可预置待删除环境） */
  openEnvironmentManagerDialog: (
    context?: EnvironmentManagerContext,
    pendingDeleteEnv?: string,
  ) => void;
  /** 关闭环境管理对话框 */
  closeEnvironmentManagerDialog: () => void;
  /** 打开导出流程对话框 */
  openExportFlowDialog: (target: Flow | FlowMetadata) => void;
  /** 关闭导出流程对话框 */
  closeExportFlowDialog: () => void;
  /** 打开快捷键面板 */
  openShortcutsDialog: () => void;
  /** 关闭快捷键面板 */
  closeShortcutsDialog: () => void;
  /** 打开搜索面板 */
  openSearchOverlay: () => void;
  /** 关闭搜索面板 */
  closeSearchOverlay: () => void;
  /** 打开脚本函数帮助对话框 */
  openScriptHelpDialog: () => void;
  /** 关闭脚本函数帮助对话框 */
  closeScriptHelpDialog: () => void;
  /** 打开动态变量帮助对话框 */
  openDynamicVariablesHelpDialog: () => void;
  /** 关闭动态变量帮助对话框 */
  closeDynamicVariablesHelpDialog: () => void;

  /** 打开流程冲突决策对话框 */
  openFlowConflictDialog: (flowConflict: FlowConflictState) => void;
  /** 关闭流程冲突决策对话框 */
  closeFlowConflictDialog: () => void;
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

  /** 在流程列表中定位并高亮流程（若未指定目标则定位当前活跃流程） */
  revealFlowInList: (target?: { uuid?: string; filePath?: string }) => void;
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
  /** 设置流程标签栏显示模式 */
  setFlowTabDisplayMode: (mode: FlowTabDisplayMode) => void;

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
const MAX_BOTTOM_PANEL_HEIGHT = 2400;
const MIN_LEFT_PANEL_HEIGHT = 100;
const MAX_LEFT_PANEL_HEIGHT = 400;

const TAB_DISPLAY_MODE_KEY = 'mpe_flow_tab_display_mode';

function loadTabDisplayMode(): FlowTabDisplayMode {
  try {
    const saved = localStorage.getItem(TAB_DISPLAY_MODE_KEY);
    if (saved === 'scroll' || saved === 'wrap') {
      return saved;
    }
  } catch {
    // 忽略
  }
  return 'scroll';
}

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

  locateFlowTarget: null,
  // 模态框状态
  showNewFlowDialog: false,
  newFlowDialogFolderPath: '',
  newFlowDialogDefaultTemplate: false,
  showDeleteConfirmDialog: false,
  pendingDeleteFlowUuid: null,
  showSettingsDialog: false,
  settingsDialogTab: 'general' as 'general' | 'storage' | 'ai' | 'engine' | 'network' | 'about' | 'shortcuts',
  showPluginSettingsDialog: false,
  configPanelMode: 'node' as ConfigPanelMode,
  showStressTestConfigDialog: false,
  showDatasetExecutionPanel: false,
  showEnvironmentManagerDialog: false,
  environmentManagerContext: 'sidebar' as EnvironmentManagerContext,
  environmentManagerPendingDelete: null as string | null,
  showExportFlowDialog: false,
  exportFlowTarget: null as Flow | FlowMetadata | null,
  showShortcutsDialog: false,
  showSearchOverlay: false,
  showScriptHelpDialog: false,
  showDynamicVariablesHelpDialog: false,

  flowConflict: null as FlowConflictState | null,
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
  flowTabDisplayMode: loadTabDisplayMode(),
};

export const useUIStore = create<UIState>((set, get) => ({
  ...initialState,

  // ===== 面板 Actions =====
  toggleFlowListPanel: () => {
    startLayoutTransition(220);
    set((state) => ({
      isFlowListPanelOpen: !state.isFlowListPanelOpen,
    }));
  },

  toggleHistoryPanel: () =>
    set((state) => ({
      isHistoryPanelOpen: !state.isHistoryPanelOpen,
    })),

  toggleBottomPanel: () => {
    startLayoutTransition(220);
    set((state) => ({
      isBottomPanelExpanded: !state.isBottomPanelExpanded,
    }));
  },

  setBottomPanelExpanded: (isBottomPanelExpanded) => {
    startLayoutTransition(220);
    set({ isBottomPanelExpanded });
  },

  toggleConfigPanel: () => {
    startLayoutTransition(220);
    set((state) => ({
      isConfigPanelOpen: !state.isConfigPanelOpen,
    }));
  },

  toggleExecutionPanel: () =>
    set((state) => ({
      isExecutionPanelOpen: !state.isExecutionPanelOpen,
    })),

  setConfigPanelOpen: (isConfigPanelOpen) => {
    startLayoutTransition(220);
    set({ isConfigPanelOpen });
  },

  setExecutionPanelOpen: (isExecutionPanelOpen) => set({ isExecutionPanelOpen }),

  setFlowListPanelOpen: (isFlowListPanelOpen) => {
    startLayoutTransition(220);
    set({ isFlowListPanelOpen });
  },

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

  openSettingsDialog: (tab?: 'general' | 'storage' | 'ai' | 'engine' | 'network' | 'about' | 'shortcuts') =>
    set({
      showSettingsDialog: true,
      settingsDialogTab: (typeof tab === 'string' && ['general', 'storage', 'ai', 'engine', 'network', 'about', 'shortcuts'].includes(tab)) ? tab : 'general',
    }),

  closeSettingsDialog: () => set({ showSettingsDialog: false }),
  openFlowConflictDialog: (flowConflict) => set({ flowConflict }),
  closeFlowConflictDialog: () => set({ flowConflict: null }),

  setSettingsDialogTab: (settingsDialogTab) => set({ settingsDialogTab }),
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

  openEnvironmentManagerDialog: (context = 'sidebar', pendingDeleteEnv) =>
    set({
      showEnvironmentManagerDialog: true,
      environmentManagerContext: context,
      environmentManagerPendingDelete: pendingDeleteEnv ?? null,
    }),
  closeEnvironmentManagerDialog: () =>
    set({ showEnvironmentManagerDialog: false, environmentManagerPendingDelete: null }),
  openExportFlowDialog: (target) =>
    set({ showExportFlowDialog: true, exportFlowTarget: target }),
  closeExportFlowDialog: () =>
    set({ showExportFlowDialog: false, exportFlowTarget: null }),

  openShortcutsDialog: () => set({ showShortcutsDialog: true }),
  closeShortcutsDialog: () => set({ showShortcutsDialog: false }),

  openSearchOverlay: () => set({ showSearchOverlay: true }),
  closeSearchOverlay: () => set({ showSearchOverlay: false }),
  openScriptHelpDialog: () => set({ showScriptHelpDialog: true }),
  closeScriptHelpDialog: () => set({ showScriptHelpDialog: false }),

  openDynamicVariablesHelpDialog: () => set({ showDynamicVariablesHelpDialog: true }),
  closeDynamicVariablesHelpDialog: () => set({ showDynamicVariablesHelpDialog: false }),

  // ===== 左侧栏面板 Actions =====
  toggleFlowListCollapsed: () =>
    set((state) => ({
      isFlowListCollapsed: !state.isFlowListCollapsed,
    })),

  revealFlowInList: (target) => {
    const state = get();
    if (!state.isFlowListPanelOpen) {
      startLayoutTransition(220);
    }
    set({
      isFlowListPanelOpen: true,
      isFlowListCollapsed: false,
      locateFlowTarget: {
        uuid: target?.uuid,
        filePath: target?.filePath,
        timestamp: Date.now(),
      },
    });
  },
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

  setFlowTabDisplayMode: (flowTabDisplayMode) => {
    try {
      localStorage.setItem(TAB_DISPLAY_MODE_KEY, flowTabDisplayMode);
    } catch {
      // 忽略
    }
    set({ flowTabDisplayMode });
  },

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
      flowTabDisplayMode: loadTabDisplayMode(),
      flowConflict: null,
      locateFlowTarget: null,
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

/** 选择流程标签栏显示模式 */
export const selectFlowTabDisplayMode = (state: UIState) =>
  state.flowTabDisplayMode;

/** 选择流程冲突状态 */
export const selectFlowConflict = (state: UIState) =>
  state.flowConflict;
/** 选择是否展示流程冲突对话框 */
export const selectShowFlowConflictDialog = (state: UIState) =>
  state.flowConflict !== null;

/** 选择环境管理器上下文模式 */
export const selectEnvironmentManagerContext = (state: UIState) =>
  state.environmentManagerContext;
/** 选择导出流程对话框是否显示 */
export const selectShowExportFlowDialog = (state: UIState) =>
  state.showExportFlowDialog;
/** 选择导出流程目标对象 */
export const selectExportFlowTarget = (state: UIState) =>
  state.exportFlowTarget;
/** 选择定位流程目标 */
export const selectLocateFlowTarget = (state: UIState) =>
  state.locateFlowTarget;
