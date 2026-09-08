/**
 * MainLayout - 主布局组件
 * 整合所有面板：左侧流程列表/节点库、中间画布、右侧配置、底部历史
 */
import { Toaster } from '@/components/ui/sonner';
import { SandboxBanner } from '@/components/common/SandboxBanner';

import { LeftSidebarPanel } from '@/components/sidebar';
import { FlowCanvas } from '@/components/canvas';
import { NodeConfigPanel } from '@/components/panels';
import { ExecutionReport, ExecutionHistory, ConsolePanel, ExecutionPanel } from '@/components/execution';
import { FlowCompareDialog } from '@/components/execution/FlowCompareDialog';
import { StressTestResult } from '@/components/stress-test/StressTestResult';
import { StressTestMonitor } from '@/components/stress-test/StressTestMonitor';
import { NewFlowDialog } from '@/components/dialogs/NewFlowDialog';
import { PluginSettingsDialog } from '@/components/dialogs/PluginSettingsDialog';
import { SettingsDialog } from '@/components/settings';
import { UpdateDialog } from '@/components/dialogs/UpdateDialog';
import { ConfirmDialog } from '@/components/dialogs/ConfirmDialog';
import { ShortcutsDialog } from '@/components/dialogs/ShortcutsDialog';
import { ScriptHelpDialog } from '@/components/dialogs/ScriptHelpDialog';
import { DynamicVariablesHelpDialog } from '@/components/dialogs/DynamicVariablesHelpDialog';
import { SearchOverlay } from '@/components/dialogs/SearchOverlay';
import { EnvironmentManagerDialog } from '@/components/panels/EnvironmentManager';
import {
  DatasetReportList,
  DatasetReportDetail,
  DatasetRowDetail,
  DatasetReportDialog,
} from '@/components/reports';
import { AICopilotPanel, AISpotlightDialog } from '@/components/ai';
import { useAIStore } from '@/stores/ai';

import { useUIStore, useFlowStore, useDebugStore, selectIsDebugging, selectIsDebugDataReadOnly } from '@/stores';
import { useSettingsStore } from '@/stores/settings';
import { useConsoleStore } from '@/stores/console';
import { useZoomStore } from '@/stores/zoom';
import { useUpdateStore } from '@/stores/update';
import { ChevronLeft, ChevronRight, ChevronUp, ChevronDown, Trash2, Settings, Puzzle, GripVertical, GripHorizontal, ZoomIn, ZoomOut, Keyboard, Sparkles, Copy, HelpCircle, BookOpen } from 'lucide-react';
import * as Popover from '@radix-ui/react-popover';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { useState, useCallback, useRef, useEffect } from 'react';
import { usePanelDrag } from '@/hooks';
import { toast } from 'sonner';
import { loadReport } from '@/services/storage/reportStorage';
import { stressTestStorage } from '@/services/storage/stressTestStorage';
import { flowStorage } from '@/services/storage/flowStorage';
import type { Flow } from '@/types';
import { getLL } from '@/i18n/i18n-lazy';
import { copyToClipboard, safeStringify } from '@/lib/clipboard';
/**
 * 确认对话框处理器组件
 * 从 store 读取状态并渲染 ConfirmDialog
 */
function ConfirmDialogHandler() {
  const showConfirmDialog = useUIStore((state) => state.showConfirmDialog);
  const confirmDialogConfig = useUIStore((state) => state.confirmDialogConfig);
  const hideConfirm = useUIStore((state) => state.hideConfirm);

  if (!showConfirmDialog || !confirmDialogConfig) {
    return null;
  }

  // 延迟求值：函数类型的 title/description/confirmText 在渲染时调用，确保跟随当前语言
  const resolve = (v: string | (() => string) | undefined): string | undefined =>
    typeof v === 'function' ? v() : v;

  return (
    <ConfirmDialog
      open={showConfirmDialog}
      title={resolve(confirmDialogConfig.title) ?? ''}
      description={resolve(confirmDialogConfig.description)}
      confirmText={resolve(confirmDialogConfig.confirmText)}
      onConfirm={() => {
        confirmDialogConfig.onConfirm();
        hideConfirm();
      }}
      onCancel={() => {
        confirmDialogConfig.onCancel?.();
        hideConfirm();
      }}
    />
  );
}

/**
 * 搜索面板处理器组件
 * 从 store 读取状态并渲染 SearchOverlay
 */
function SearchOverlayHandler() {
  const showSearchOverlay = useUIStore((state) => state.showSearchOverlay);
  const closeSearchOverlay = useUIStore((state) => state.closeSearchOverlay);

  return (
    <SearchOverlay
      open={showSearchOverlay}
      onOpenChange={(open) => {
        if (!open) closeSearchOverlay();
      }}
    />
  );
}

export function MainLayout() {
  const isFlowListPanelOpen = useUIStore((state) => state.isFlowListPanelOpen);
  const isBottomPanelExpanded = useUIStore((state) => state.isBottomPanelExpanded);
  const isConfigPanelOpen = useUIStore((state) => state.isConfigPanelOpen);
  const bottomPanelTab = useUIStore((state) => state.bottomPanelTab);
  const bottomPanelHeight = useUIStore((state) => state.bottomPanelHeight);
  const configPanelWidth = useUIStore((state) => state.configPanelWidth);
  const sidebarWidth = useUIStore((state) => state.sidebarWidth);
  const toggleFlowListPanel = useUIStore((state) => state.toggleFlowListPanel);
  const toggleBottomPanel = useUIStore((state) => state.toggleBottomPanel);
  const setBottomPanelExpanded = useUIStore((state) => state.setBottomPanelExpanded);
  const toggleConfigPanel = useUIStore((state) => state.toggleConfigPanel);
  const setBottomPanelTab = useUIStore((state) => state.setBottomPanelTab);
  const setBottomPanelHeight = useUIStore((state) => state.setBottomPanelHeight);
  const setConfigPanelWidth = useUIStore((state) => state.setConfigPanelWidth);
  const setSidebarWidth = useUIStore((state) => state.setSidebarWidth);
  const openSettingsDialog = useUIStore((state) => state.openSettingsDialog);
  const showConfirm = useUIStore((state) => state.showConfirm);
  const showEnvironmentManagerDialog = useUIStore((state) => state.showEnvironmentManagerDialog);

  // 页面导航
  const currentPage = useUIStore((state) => state.currentPage);
  const datasetReportPage = useUIStore((state) => state.datasetReportPage);
  const navigateToDatasetReportDetail = useUIStore((state) => state.navigateToDatasetReportDetail);
  const navigateToDatasetRowDetail = useUIStore((state) => state.navigateToDatasetRowDetail);

  // 缩放状态
  const zoomIn = useZoomStore((state) => state.zoomIn);
  const zoomOut = useZoomStore((state) => state.zoomOut);
  const isDirty = useFlowStore((state) => state.isDirty);
  const flow = useFlowStore((state) => state.flow);

  // 设置 store
  const reportDir = useSettingsStore((state) => state.reportDir);

  // Debug store values
  const isDebugging = useDebugStore(selectIsDebugging);
  const isDebugDataReadOnly = useDebugStore(selectIsDebugDataReadOnly);
  
  // 调试时右侧面板只读
  const isRightPanelReadOnly = isDebugging || isDebugDataReadOnly;

  // Console store values
  const filter = useConsoleStore((state) => state.filter);
  const clearLogs = useConsoleStore((state) => state.clearLogs);
  const setFilter = useConsoleStore((state) => state.setFilter);
  const setSearchKeyword = useConsoleStore((state) => state.setSearchKeyword);
  const getUniqueNodes = useConsoleStore((state) => state.getUniqueNodes);

  // 清空控制台（带确认）
  const handleClearConsole = useCallback(() => {
    showConfirm({
      title: getLL().flow.clear_console(),
      description: getLL().flow.clear_logs_confirm_desc(),
      confirmText: getLL().common.clear(),
      onConfirm: () => {
        clearLogs();
      },
    });
  }, [clearLogs, showConfirm]);

  // 复制控制台日志
  const handleCopyConsoleLogs = useCallback(async () => {
    const logs = useConsoleStore.getState().getFilteredLogs();
    if (logs.length === 0) {
      toast.info(getLL().viewer.console.no_logs());
      return;
    }
    const text = logs
      .map((l) => {
        const time = new Date(l.timestamp).toISOString();
        const tag = l.nodeName ? ` ${l.nodeName}` : '';
        const details = l.details ? `\n  Details: ${safeStringify(l.details, 2)}` : '';
        return `[${time}] [${l.level}]${tag} ${l.message}${details}`;
      })
      .join('\n');

    const copied = await copyToClipboard(text);
    if (copied) {
      toast.success(getLL().flow.logs_copied());
    } else {
      toast.error(getLL().flow.copy_logs_failed?.() ?? 'Failed to copy logs');
    }
  }, []);

  // 底部面板 Tab 点击：点击相同 tab 收起并取消选中，不同 tab 切换并展开
  const handleTabClick = useCallback((tab: 'console' | 'history' | 'running') => {
    if (tab === bottomPanelTab && isBottomPanelExpanded) {
      // 点击当前激活的 tab → 收起并取消选中
      setBottomPanelTab('');
      setBottomPanelExpanded(false);
    } else {
      // 点击不同 tab 或当前收起 → 切换到目标 tab 并展开
      setBottomPanelTab(tab);
      if (!isBottomPanelExpanded) {
        setBottomPanelExpanded(true);
      }
    }
  }, [bottomPanelTab, isBottomPanelExpanded, setBottomPanelTab, setBottomPanelExpanded]);

  // 对比回调：加载报告快照并打开对比对话框
  const handleCompare = useCallback(async (reportUuid: string) => {
    try {
      // 尝试加载普通报告
      const report = await loadReport(reportUuid, reportDir);
      if (report?.flow_snapshot) {
        setCompareSnapshotFlow(report.flow_snapshot);
        setCompareSnapshotName(report.flow_name);
        const currentFlow = await flowStorage.loadFlow(report.flow_uuid).catch(() => null);
        setCompareCurrentFlow(currentFlow);
        setCompareCurrentName(currentFlow?.name ?? report.flow_name);
        setCompareOpen(true);
        return;
      }
    } catch {
      // 普通报告加载失败，尝试压测报告
      console.warn('加载普通报告失败，尝试压测报告:', reportUuid);
    }
    try {
      // 通过历史列表找到对应的压测报告 filePath
      const stressReports = await stressTestStorage.listReports(reportDir);
      const stressMeta = stressReports.find((s) => s.uuid === reportUuid);
      if (stressMeta) {
        const stressReport = await stressTestStorage.loadReport(stressMeta.filePath);
        if (stressReport?.flowSnapshot) {
          setCompareSnapshotFlow(stressReport.flowSnapshot);
          setCompareSnapshotName(stressReport.flowName);
          const currentFlow = await flowStorage.loadFlow(stressReport.flowUuid).catch(() => null);
          setCompareCurrentFlow(currentFlow);
          setCompareCurrentName(currentFlow?.name ?? stressReport.flowName);
          setCompareOpen(true);
          return;
        }
      }
    } catch {
      // 压测报告加载失败
      console.warn('加载压测报告失败:', reportUuid);
    }
    toast.error(getLL().flow.report_no_snapshot());
  }, [reportDir]);

  // 对比对话框状态
  const [compareOpen, setCompareOpen] = useState(false);
  const [compareSnapshotFlow, setCompareSnapshotFlow] = useState<Flow | null>(null);
  const [compareCurrentFlow, setCompareCurrentFlow] = useState<Flow | null>(null);
  const [compareSnapshotName, setCompareSnapshotName] = useState('');
  const [compareCurrentName, setCompareCurrentName] = useState('');

  // 底部面板内容延迟渲染：展开动画结束后再渲染内容
  const [bottomPanelContentReady, setBottomPanelContentReady] = useState(isBottomPanelExpanded);
  const bottomPanelTransitionRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // 拖拽调整面板大小（底部面板最高可拉至接近顶部，方便查看全屏日志和 Agent 报告）
  const maxBottomHeight = typeof window !== 'undefined' ? Math.max(700, window.innerHeight - 80) : 1600;
  const bottomDrag = usePanelDrag({
    axis: 'vertical',
    min: 100,
    max: maxBottomHeight,
    sign: 1,
    initialSize: bottomPanelHeight,
    onResize: setBottomPanelHeight,
  });
  const rightDrag = usePanelDrag({
    axis: 'horizontal', min: 250, max: 500,
    initialSize: configPanelWidth,
    onResize: setConfigPanelWidth,
  });

  const leftDrag = usePanelDrag({
    axis: 'horizontal', min: 200, max: 400, sign: -1,
    initialSize: sidebarWidth,
    onResize: setSidebarWidth,
  });

  // 底部面板内容延迟渲染：展开动画结束后渲染，收起时立即移除
  useEffect(() => {
    // 清除之前的定时器
    if (bottomPanelTransitionRef.current) {
      clearTimeout(bottomPanelTransitionRef.current);
      bottomPanelTransitionRef.current = null;
    }

    if (isBottomPanelExpanded) {
      // 展开：等待动画结束后渲染内容
      bottomPanelTransitionRef.current = setTimeout(() => {
        setBottomPanelContentReady(true);
      }, 300);
    } else {
      // 收起：立即移除内容
      setBottomPanelContentReady(false);
    }

    return () => {
      if (bottomPanelTransitionRef.current) {
        clearTimeout(bottomPanelTransitionRef.current);
      }
    };
  }, [isBottomPanelExpanded]);
  // 启动后异步静默检查版本更新
  useEffect(() => {
    const timer = setTimeout(() => {
      const { autoCheckUpdate } = useSettingsStore.getState();
      if (autoCheckUpdate) {
        void useUpdateStore.getState().checkForUpdates(false);
      }
    }, 2000);
    return () => clearTimeout(timer);
  }, []);


  return (
    <div className="h-screen flex flex-col bg-background text-foreground overflow-hidden">
      <SandboxBanner />
      {/* ===== 顶部工具栏 ===== */}
      <header className="h-14 shrink-0 border-b border-border flex items-center justify-between px-4 bg-background">
        {/* 左侧：Logo */}
        <div className="flex items-center gap-4" data-guide="app-logo" data-testid="app-title">
          <svg viewBox="0 -3 600 136" className="h-10 w-auto" fill="none" xmlns="http://www.w3.org/2000/svg" style={{color: 'inherit'}}>
            <defs>
              <linearGradient id="hdr-grad" x1="0" y1="0" x2="100" y2="100">
                <stop offset="0%" stopColor="#818CF8"/>
                <stop offset="100%" stopColor="#22D3EE"/>
              </linearGradient>
              <linearGradient id="hdr-ring" x1="0" y1="0" x2="100" y2="100">
                <stop offset="0%" stopColor="#818CF8" stopOpacity="0.18"/>
                <stop offset="100%" stopColor="#22D3EE" stopOpacity="0.18"/>
              </linearGradient>
            </defs>
            <g transform="translate(68, 65)">
              <circle r="48" fill="url(#hdr-ring)"/>
              <line x1="0" y1="-22" x2="0" y2="-50" stroke="#818CF8" strokeWidth="4" strokeLinecap="round" opacity="0.5"/>
              <line x1="22" y1="0" x2="50" y2="0" stroke="#22D3EE" strokeWidth="4" strokeLinecap="round" opacity="0.5"/>
              <line x1="0" y1="22" x2="0" y2="50" stroke="#818CF8" strokeWidth="4" strokeLinecap="round" opacity="0.5"/>
              <line x1="-22" y1="0" x2="-50" y2="0" stroke="#22D3EE" strokeWidth="4" strokeLinecap="round" opacity="0.5"/>
              <polygon points="-6,-38 0,-48 6,-38" fill="#818CF8" opacity="0.8"/>
              <polygon points="38,-6 48,0 38,6" fill="#22D3EE" opacity="0.8"/>
              <polygon points="6,38 0,48 -6,38" fill="#818CF8" opacity="0.8"/>
              <polygon points="-38,6 -48,0 -38,-6" fill="#22D3EE" opacity="0.8"/>
              <circle cx="0" cy="-56" r="8" fill="currentColor" stroke="#818CF8" strokeWidth="4"/>
              <circle cx="56" cy="0" r="8" fill="currentColor" stroke="#22D3EE" strokeWidth="4"/>
              <circle cx="0" cy="56" r="8" fill="currentColor" stroke="#818CF8" strokeWidth="4"/>
              <circle cx="-56" cy="0" r="8" fill="currentColor" stroke="#22D3EE" strokeWidth="4"/>
              <circle r="22" fill="url(#hdr-grad)" opacity="0.15"/>
              <circle r="17" stroke="url(#hdr-grad)" strokeWidth="4" fill="currentColor"/>
              <circle r="9" fill="url(#hdr-grad)"/>
            </g>
            <text x="148" y="65" fill="currentColor" fontFamily="'Inter','SF Pro Display',system-ui,-apple-system,sans-serif" fontSize="46" fontWeight="800" letterSpacing="-0.5">MPE</text>
            <text x="148" y="96" fill="currentColor" fontFamily="system-ui,-apple-system,sans-serif" fontSize="15" fontWeight="600" letterSpacing="3" opacity="0.6">MULTI-PROTOCOL FLOW EXECUTOR</text>
          </svg>

          {/* 当前流程名称 */}
          {flow && (
            <button
              type="button"
              onClick={() => {
                useFlowStore.getState().selectNode(null);
                useUIStore.getState().setConfigPanelOpen(true);
              }}
              className="flex items-center gap-2 px-3 py-1 rounded-full bg-accent border border-border hover:bg-accent/80 hover:border-primary/40 transition-colors cursor-pointer text-left"
              title={getLL().flow.flow_properties()}
            >
              <span className="text-sm text-muted-foreground hover:text-foreground transition-colors">{flow.name}</span>
              {isDirty && (
                <span className="w-2 h-2 rounded-full bg-amber-400 animate-pulse" title={getLL().flow.unsaved()} />
              )}
            </button>
          )}
        </div>

        {/* 右侧：操作按钮 */}
        <div className="flex items-center gap-2">

          {/* 缩放控件 */}
          {currentPage === 'canvas' && (
          <div className="flex items-center gap-0.5 border border-border rounded-md px-1">
            <Button
              variant="ghost"
              size="icon"
              onClick={zoomOut}
              className="h-7 w-7"
              title={getLL().common.zoom_out()}
            >
              <ZoomOut className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              onClick={zoomIn}
              className="h-7 w-7"
              title={getLL().common.zoom_in()}
            >
              <ZoomIn className="h-4 w-4" />
            </Button>
          </div>
          )}

          {/* 帮助与手册下拉菜单 */}
          <Popover.Root>
            <Popover.Trigger asChild>
              <Button
                variant="ghost"
                size="icon"
                title={getLL().settings.header_help_title()}
                className="hidden sm:inline-flex cursor-pointer"
              >
                <HelpCircle className="h-5 w-5" />
              </Button>
            </Popover.Trigger>
            <Popover.Portal>
              <Popover.Content
                side="bottom"
                align="end"
                sideOffset={8}
                className="w-56 rounded-xl border border-border bg-popover p-1.5 text-popover-foreground shadow-xl z-50 animate-in fade-in-0 zoom-in-95 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95"
              >
                <div className="px-2 py-1 text-xs font-semibold text-muted-foreground">
                  {getLL().settings.header_help_title()}
                </div>
                <button
                  type="button"
                  onClick={() => useUIStore.getState().openShortcutsDialog()}
                  className="w-full flex items-center justify-between px-2 py-1.5 rounded-lg text-xs hover:bg-muted transition-colors text-left cursor-pointer"
                >
                  <span className="flex items-center gap-2">
                    <Keyboard className="h-4 w-4 text-muted-foreground" />
                    <span>{getLL().shortcuts.title()}</span>
                  </span>
                  <kbd className="px-1 py-0.5 text-[10px] font-mono rounded border border-border bg-muted/60 text-muted-foreground">
                    Ctrl+/
                  </kbd>
                </button>
                <button
                  type="button"
                  onClick={() => useUIStore.getState().openScriptHelpDialog()}
                  className="w-full flex items-center gap-2 px-2 py-1.5 rounded-lg text-xs hover:bg-muted transition-colors text-left cursor-pointer"
                >
                  <BookOpen className="h-4 w-4 text-violet-500" />
                  <span>{getLL().settings.open_script_manual()}</span>
                </button>
                <button
                  type="button"
                  onClick={() => useUIStore.getState().openDynamicVariablesHelpDialog()}
                  className="w-full flex items-center gap-2 px-2 py-1.5 rounded-lg text-xs hover:bg-muted transition-colors text-left cursor-pointer"
                >
                  <Sparkles className="h-4 w-4 text-emerald-500" />
                  <span>{getLL().settings.open_dynamic_vars_manual()}</span>
                </button>
                <div className="my-1 border-t border-border/60" />
                <button
                  type="button"
                  onClick={() => useUIStore.getState().openSettingsDialog('shortcuts')}
                  className="w-full flex items-center gap-2 px-2 py-1.5 rounded-lg text-xs text-muted-foreground hover:text-foreground hover:bg-muted transition-colors text-left cursor-pointer"
                >
                  <Settings className="h-4 w-4" />
                  <span>{getLL().settings.tab_shortcuts()}</span>
                </button>
              </Popover.Content>
            </Popover.Portal>
          </Popover.Root>
          {/* 插件设置按钮 */}
          <Button
            variant="ghost"
            size="icon"
            onClick={() => useUIStore.getState().openPluginSettingsDialog()}
            title={getLL().plugins.settings_title()}
            className="hidden sm:inline-flex"
          >
            <Puzzle className="h-5 w-5" />
          </Button>

          {/* AI Copilot 助手按钮 */}
          <Button
            variant="ghost"
            size="icon"
            onClick={() => useAIStore.getState().toggleCopilot()}
            title={`${getLL().ai.copilot.title()} (Ctrl+I)`}
            className={cn(
              'hidden sm:inline-flex relative',
              useAIStore((s) => s.isCopilotOpen) && 'text-primary bg-primary/10'
            )}
          >
            <Sparkles className="h-5 w-5" />
          </Button>

          {/* 设置按钮 */}
          <Button
            variant="ghost"
            size="icon"
            onClick={() => openSettingsDialog()}
            title={getLL().common.settings_label()}
          >
            <Settings className="h-5 w-5" />
          </Button>

        </div>
      </header>

      {/* ===== 主内容区 ===== */}
      {currentPage === 'dataset-reports' ? (
        /* 数据集报告页面 */
        <div className="flex-1 overflow-hidden">
          {!datasetReportPage.reportId ? (
            <DatasetReportList
              onViewReport={(reportId) => navigateToDatasetReportDetail(reportId)}
            />
          ) : datasetReportPage.rowIndex != null ? (
            <DatasetRowDetail
              reportId={datasetReportPage.reportId}
              rowIndex={datasetReportPage.rowIndex}
              onBack={() => navigateToDatasetReportDetail(datasetReportPage.reportId!)}
            />
          ) : (
            <DatasetReportDetail
              reportId={datasetReportPage.reportId}
              onViewRowDetail={(reportId, rowIndex) =>
                navigateToDatasetRowDetail(reportId, rowIndex)
              }
            />
          )}
        </div>
      ) : (
      <div className="flex-1 flex overflow-hidden relative">
        {/* ===== 左侧栏 ===== */}
        <aside
          className={cn(
            'shrink-0 border-r border-border flex flex-col bg-background overflow-hidden relative',
            'transition-all duration-300 ease-out',
            isFlowListPanelOpen ? 'min-w-0' : 'w-0 min-w-0',
            leftDrag.isDragging && '!transition-none'
          )}
          style={{ width: isFlowListPanelOpen ? sidebarWidth : 0 }}
        >
          {/* 可拖拽调整宽度的 handle - 放在面板内部右边缘 */}
          {isFlowListPanelOpen && (
            <div
              onMouseDown={leftDrag.handleMouseDown}
              className={cn(
                'absolute right-0 top-0 bottom-0 w-2 z-10',
                'bg-transparent hover:bg-border active:bg-input',
                'cursor-col-resize flex items-center justify-center',
                'transition-colors duration-150',
                leftDrag.isDragging && 'bg-input'
              )}
              title={getLL().common.drag_resize_width()}
            >
              <GripVertical className="w-2.5 h-6 text-muted-foreground" />
            </div>
          )}
          {isFlowListPanelOpen && (
            <div className="h-full min-w-0 overflow-hidden pr-2" style={{ width: sidebarWidth }}>
              <LeftSidebarPanel />
            </div>
          )}
        </aside>

        {/* 左侧折叠按钮 */}
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleFlowListPanel}
          className={cn(
            'absolute z-20 h-8 w-6 rounded-l-none rounded-r-lg top-0',
            'bg-background hover:bg-accent border border-l-0 border-border',
            'transition-colors duration-150'
          )}
          style={{ left: isFlowListPanelOpen ? sidebarWidth : 0 }}
          title={isFlowListPanelOpen ? getLL().common.toggle_sidebar_collapse() : getLL().common.toggle_sidebar_expand()}
        >
          {isFlowListPanelOpen ? (
            <ChevronLeft className="w-4 h-4 text-muted-foreground" />
          ) : (
            <ChevronRight className="w-4 h-4 text-muted-foreground" />
          )}
        </Button>

        {/* ===== 中间画布区 ===== */}
        <main className="flex-1 flex flex-col min-w-0 overflow-hidden relative">
          {/* 画布 */}
          <div className="flex-1 relative">
            <FlowCanvas />
          </div>

          {/* ===== 底部面板（Tab Bar + 可折叠内容区） ===== */}
          <div className="shrink-0 border-t border-border bg-background flex flex-col">
            {/* 拖拽调整高度的 handle - 只在展开时显示 */}
            {isBottomPanelExpanded && (
              <div
                onMouseDown={bottomDrag.handleMouseDown}
                className={cn(
                  'h-2 shrink-0',
                  'bg-transparent hover:bg-border active:bg-input',
                  'cursor-row-resize flex items-center justify-center',
                  'transition-colors duration-150',
                  bottomDrag.isDragging && 'bg-input'
                )}
                title={getLL().common.drag_resize_height()}
              >
                <GripHorizontal className="w-6 h-2.5 text-muted-foreground" />
              </div>
            )}

            {/* Tab Bar - 始终可见 */}
            <div className="flex items-center h-9 px-2 shrink-0">
              <div className="flex items-center gap-1">
                <button
                  onClick={() => handleTabClick('console')}
                  className={cn(
                    'px-3 py-1 text-xs rounded-md transition-colors',
                    bottomPanelTab === 'console'
                      ? 'bg-accent text-accent-foreground font-medium'
                      : 'text-muted-foreground hover:text-foreground hover:bg-accent/50'
                  )}
                >
                  {getLL().flow.console()}
                </button>
                <button
                  onClick={() => handleTabClick('history')}
                  className={cn(
                    'px-3 py-1 text-xs rounded-md transition-colors',
                    bottomPanelTab === 'history'
                      ? 'bg-accent text-accent-foreground font-medium'
                      : 'text-muted-foreground hover:text-foreground hover:bg-accent/50'
                  )}
                >
                  {getLL().flow.execution_history()}
                </button>
                <button
                  onClick={() => handleTabClick('running')}
                  className={cn(
                    'px-3 py-1 text-xs rounded-md transition-colors',
                    bottomPanelTab === 'running'
                      ? 'bg-accent text-accent-foreground font-medium'
                      : 'text-muted-foreground hover:text-foreground hover:bg-accent/50'
                  )}
                >
                  {getLL().flow.running()}
                </button>
              </div>
              <div className="flex-1" />
              <Button
                variant="ghost"
                size="icon"
                onClick={toggleBottomPanel}
                className="h-7 w-7 hover:bg-accent"
                title={isBottomPanelExpanded ? getLL().flow.collapse_panel() : getLL().flow.expand_bottom_panel()}
              >
                {isBottomPanelExpanded ? (
                  <ChevronDown className="w-4 h-4 text-muted-foreground" />
                ) : (
                  <ChevronUp className="w-4 h-4 text-muted-foreground" />
                )}
              </Button>
            </div>

            {/* Content Area - 可折叠 */}
            <div
              className={cn(
                'overflow-hidden transition-all duration-300 ease-out',
                bottomDrag.isDragging && '!transition-none'
              )}
              style={{ height: isBottomPanelExpanded ? bottomPanelHeight : 0 }}
            >
              {bottomPanelContentReady && bottomPanelTab && (
                <div className="h-full flex flex-col">
                  {/* 根据选中 Tab 渲染内容 */}
                  {bottomPanelTab === 'history' && (
                    <div className="flex-1 min-h-0 overflow-hidden">
                      <ExecutionHistory onCompare={handleCompare} />
                    </div>
                  )}
                  {bottomPanelTab === 'running' && (
                    <div className="flex-1 min-h-0 overflow-hidden">
                      <ExecutionPanel />
                    </div>
                  )}
                  {bottomPanelTab === 'console' && (
                    <div className="flex-1 min-h-0 overflow-hidden flex flex-col">
                      {/* 过滤控件 */}
                      <div className="flex items-center gap-2 px-2 py-1.5 border-b border-border shrink-0">
                        {/* 来源过滤 */}
                        <Select value={filter.source} onValueChange={(v) => setFilter({ source: v as 'ALL' | 'flow' | 'system' })}>
                          <SelectTrigger className="w-24 h-7 text-xs">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="ALL">{getLL().flow.log_source_all()}</SelectItem>
                            <SelectItem value="flow">{getLL().flow.log_source_flow()}</SelectItem>
                            <SelectItem value="system">{getLL().flow.log_source_system()}</SelectItem>
                          </SelectContent>
                        </Select>
                        {/* 级别过滤 */}
                        <Select value={filter.level} onValueChange={(v) => setFilter({ level: v as 'ALL' | 'INFO' | 'DEBUG' | 'WARN' | 'ERROR' })}>
                          <SelectTrigger className="w-20 h-7 text-xs">
                            <SelectValue placeholder={getLL().flow.level()} />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="ALL">{getLL().flow.all()}</SelectItem>
                            <SelectItem value="INFO">INFO</SelectItem>
                            <SelectItem value="DEBUG">DEBUG</SelectItem>
                            <SelectItem value="WARN">WARN</SelectItem>
                            <SelectItem value="ERROR">ERROR</SelectItem>
                          </SelectContent>
                        </Select>
                        {/* 节点/分类过滤 */}
                        <Select value={filter.nodeUuid} onValueChange={(v) => setFilter({ nodeUuid: v })}>
                          <SelectTrigger className="w-28 h-7 text-xs">
                            <SelectValue placeholder={getLL().flow.node()} />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="ALL">{getLL().flow.all_nodes()}</SelectItem>
                            {getUniqueNodes().map((node) => (
                              <SelectItem key={node.uuid} value={node.uuid}>
                                {node.name}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                        <Input
                          placeholder={getLL().flow.search_placeholder()}
                          value={filter.searchKeyword}
                          onChange={(e) => setSearchKeyword(e.target.value)}
                          className="h-7 w-32 text-xs"
                        />
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={handleCopyConsoleLogs}
                          className="h-7 px-2 text-xs text-muted-foreground hover:text-accent-foreground"
                          title={getLL().flow.copy_logs()}
                        >
                          <Copy className="w-3.5 h-3.5 mr-1" />
                          {getLL().flow.copy_logs()}
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={handleClearConsole}
                          className="h-7 px-2 text-xs text-muted-foreground hover:text-accent-foreground"
                          title={getLL().flow.clear_logs()}
                        >
                          <Trash2 className="w-3.5 h-3.5 mr-1" />
                          {getLL().common.clear()}
                        </Button>
                      </div>
                      {/* 控制台内容 */}
                      <div className="flex-1 min-h-0 overflow-hidden">
                        <ConsolePanel />
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>
        </main>

        {/* ===== 右侧配置面板 ===== */}
        <aside
          className={cn(
            'shrink-0 border-l border-border bg-background overflow-hidden relative',
            'transition-all duration-300 ease-out',
            isConfigPanelOpen ? 'min-w-0' : 'w-0 min-w-0',
            rightDrag.isDragging && '!transition-none'
          )}
          style={{ width: isConfigPanelOpen ? configPanelWidth : 0 }}
        >
          {/* 可拖拽调整宽度的 handle - 放在面板内部左边缘 */}
          {isConfigPanelOpen && (
            <div
              onMouseDown={rightDrag.handleMouseDown}
              className={cn(
                'absolute left-0 top-0 bottom-0 w-2 z-10',
                'bg-transparent hover:bg-border active:bg-input',
                'cursor-col-resize flex items-center justify-center',
                'transition-colors duration-150',
                rightDrag.isDragging && 'bg-input'
              )}
              title={getLL().common.drag_resize_width()}
            >
              <GripVertical className="w-2.5 h-6 text-muted-foreground" />
            </div>
          )}
          {isConfigPanelOpen && (
            <div className="h-full min-w-0 overflow-hidden pl-2" style={{ width: configPanelWidth }}>
              <NodeConfigPanel readOnly={isRightPanelReadOnly} />
            </div>
          )}
        </aside>

        {/* 右侧折叠按钮 */}
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleConfigPanel}
          className={cn(
            'absolute z-20 h-8 w-6 rounded-r-none rounded-l-lg top-0',
            'bg-background hover:bg-accent border border-r-0 border-border',
            'transition-colors duration-150'
          )}
          style={{ right: isConfigPanelOpen ? configPanelWidth : 0 }}
          title={isConfigPanelOpen ? getLL().common.toggle_config_collapse() : getLL().common.toggle_config_expand()}
        >
          {isConfigPanelOpen ? (
            <ChevronRight className="w-4 h-4 text-muted-foreground" />
          ) : (
            <ChevronLeft className="w-4 h-4 text-muted-foreground" />
          )}
        </Button>

      </div>
      )}

      {/* ===== 执行报告弹窗 ===== */}
      <ExecutionReport />

      {/* ===== 压测报告弹窗 ===== */}
      <StressTestResult />

      {/* ===== 数据集报告弹窗 ===== */}
      <DatasetReportDialog />

      {/* ===== 压测实时监控面板 ===== */}
      <StressTestMonitor />

      {/* ===== 新建流程对话框 ===== */}
      <NewFlowDialog />

      {/* ===== 流程对比对话框 ===== */}
      {compareSnapshotFlow && (
        <FlowCompareDialog
          open={compareOpen}
          onOpenChange={setCompareOpen}
          snapshotFlow={compareSnapshotFlow}
          currentFlow={compareCurrentFlow}
          snapshotName={compareSnapshotName}
          currentName={compareCurrentName}
        />
      )}

      {/* ===== Toast 提示 ===== */}
      <Toaster />

      {/* ===== 环境管理对话框（必须在 ReactFlow 外部渲染） ===== */}
      {showEnvironmentManagerDialog && <EnvironmentManagerDialog />}

      {/* ===== 通用确认对话框 ===== */}
      <ConfirmDialogHandler />

      {/* ===== 快捷键面板 ===== */}
      <ShortcutsDialog />
      {/* ===== 脚本函数 API 参考对话框 ===== */}
      <ScriptHelpDialog
        open={useUIStore((s) => s.showScriptHelpDialog)}
        onOpenChange={(open) => {
          if (!open) useUIStore.getState().closeScriptHelpDialog();
        }}
      />

      {/* ===== 动态变量手册对话框 ===== */}
      <DynamicVariablesHelpDialog
        open={useUIStore((s) => s.showDynamicVariablesHelpDialog)}
        onOpenChange={(open) => {
          if (!open) useUIStore.getState().closeDynamicVariablesHelpDialog();
        }}
      />

      {/* ===== 插件设置对话框 ===== */}
      <PluginSettingsDialog />

      {/* ===== 全局设置对话框 ===== */}
      <SettingsDialog />

      {/* ===== 版本更新提示对话框 ===== */}
      <UpdateDialog />
      {/* ===== 全流程搜索面板 ===== */}

      {/* ===== AI 助手侧边栏与 Spotlight 弹窗 ===== */}
      <AICopilotPanel />
      <AISpotlightDialog />
      <SearchOverlayHandler />
    </div>
  );
}
