/**
 * MainLayout - 主布局组件
 * 整合所有面板：左侧流程列表/节点库、中间画布、右侧配置、底部历史
 */
import { Toaster } from '@/components/ui/sonner';

import { LeftSidebarPanel } from '@/components/sidebar';
import { FlowCanvas } from '@/components/canvas';
import { NodeConfigPanel, SettingsPanel } from '@/components/panels';
import { ExecutionReport, ExecutionHistory, ConsolePanel, ExecutionPanel } from '@/components/execution';
import { FlowCompareDialog } from '@/components/execution/FlowCompareDialog';
import { StressTestResult } from '@/components/stress-test/StressTestResult';
import { StressTestMonitor } from '@/components/stress-test/StressTestMonitor';
import { NewFlowDialog } from '@/components/dialogs/NewFlowDialog';
import { ConfirmDialog } from '@/components/dialogs/ConfirmDialog';
import { ShortcutsDialog } from '@/components/dialogs/ShortcutsDialog';
import { SearchOverlay } from '@/components/dialogs/SearchOverlay';
import { EnvironmentManagerDialog } from '@/components/panels/EnvironmentManager';
import {
  DatasetReportList,
  DatasetReportDetail,
  DatasetRowDetail,
  DatasetReportDialog,
} from '@/components/reports';

import { useUIStore, useFlowStore, useDebugStore, selectIsDebugging, selectIsDebugDataReadOnly } from '@/stores';
import { useSettingsStore } from '@/stores/settings';
import type { BottomPanelTab } from '@/stores/ui';
import { useConsoleStore } from '@/stores/console';
import { useZoomStore } from '@/stores/zoom';
import { ChevronLeft, ChevronRight, ChevronUp, ChevronDown, Trash2, Settings, GripVertical, GripHorizontal, ZoomIn, ZoomOut, Keyboard } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { useState, useCallback, useRef, useEffect } from 'react';
import { toast } from 'sonner';
import { loadReport } from '@/services/storage/reportStorage';
import { stressTestStorage } from '@/services/storage/stressTestStorage';
import { flowStorage } from '@/services/storage/flowStorage';
import type { Flow } from '@/types';
import { getLL } from '@/i18n/i18n-lazy';

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
  const isHistoryPanelOpen = useUIStore((state) => state.isHistoryPanelOpen);
  const isConfigPanelOpen = useUIStore((state) => state.isConfigPanelOpen);
  const configPanelMode = useUIStore((state) => state.configPanelMode);
  const bottomPanelTab = useUIStore((state) => state.bottomPanelTab);
  const bottomPanelHeight = useUIStore((state) => state.bottomPanelHeight);
  const configPanelWidth = useUIStore((state) => state.configPanelWidth);
  const sidebarWidth = useUIStore((state) => state.sidebarWidth);
  const toggleFlowListPanel = useUIStore((state) => state.toggleFlowListPanel);
  const toggleHistoryPanel = useUIStore((state) => state.toggleHistoryPanel);
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
  const zoom = useZoomStore((state) => state.zoom);
  const zoomIn = useZoomStore((state) => state.zoomIn);
  const zoomOut = useZoomStore((state) => state.zoomOut);
  const resetZoom = useZoomStore((state) => state.resetZoom);

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

  // 底部面板拖拽调整高度相关状态
  const [isDragging, setIsDragging] = useState(false);
  const startYRef = useRef(0);
  const startHeightRef = useRef(0);

  // 对比对话框状态
  const [compareOpen, setCompareOpen] = useState(false);
  const [compareSnapshotFlow, setCompareSnapshotFlow] = useState<Flow | null>(null);
  const [compareCurrentFlow, setCompareCurrentFlow] = useState<Flow | null>(null);
  const [compareSnapshotName, setCompareSnapshotName] = useState('');
  const [compareCurrentName, setCompareCurrentName] = useState('');

  // 底部面板内容延迟渲染：展开动画结束后再渲染内容
  const [bottomPanelContentReady, setBottomPanelContentReady] = useState(isHistoryPanelOpen);
  const bottomPanelTransitionRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // 右侧面板拖拽调整宽度相关状态
  const [isRightPanelDragging, setIsRightPanelDragging] = useState(false);
  const startXRef = useRef(0);
  const startWidthRef = useRef(0);

  // 左侧面板拖拽调整宽度相关状态
  const [isLeftPanelDragging, setIsLeftPanelDragging] = useState(false);
  const startLeftXRef = useRef(0);
  const startLeftWidthRef = useRef(0);

  // 底部面板开始拖拽
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
    startYRef.current = e.clientY;
    startHeightRef.current = bottomPanelHeight;
    document.body.style.cursor = 'row-resize';
    document.body.style.userSelect = 'none';
  }, [bottomPanelHeight]);

  // 底部面板拖拽中
  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDragging) return;
    const deltaY = startYRef.current - e.clientY;
    const newHeight = Math.max(100, Math.min(500, startHeightRef.current + deltaY));
    setBottomPanelHeight(newHeight);
  }, [isDragging, setBottomPanelHeight]);

  // 底部面板结束拖拽
  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, []);

  // 右侧面板开始拖拽
  const handleRightPanelMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsRightPanelDragging(true);
    startXRef.current = e.clientX;
    startWidthRef.current = configPanelWidth;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }, [configPanelWidth]);

  // 右侧面板拖拽中
  const handleRightPanelMouseMove = useCallback((e: MouseEvent) => {
    if (!isRightPanelDragging) return;
    const deltaX = startXRef.current - e.clientX;
    const newWidth = Math.max(250, Math.min(500, startWidthRef.current + deltaX));
    setConfigPanelWidth(newWidth);
  }, [isRightPanelDragging, setConfigPanelWidth]);

  // 右侧面板结束拖拽
  const handleRightPanelMouseUp = useCallback(() => {
    setIsRightPanelDragging(false);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, []);

  // 左侧面板开始拖拽
  const handleLeftPanelMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsLeftPanelDragging(true);
    startLeftXRef.current = e.clientX;
    startLeftWidthRef.current = sidebarWidth;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }, [sidebarWidth]);

  // 左侧面板拖拽中
  const handleLeftPanelMouseMove = useCallback((e: MouseEvent) => {
    if (!isLeftPanelDragging) return;
    const deltaX = e.clientX - startLeftXRef.current;
    const newWidth = Math.max(200, Math.min(400, startLeftWidthRef.current + deltaX));
    setSidebarWidth(newWidth);
  }, [isLeftPanelDragging, setSidebarWidth]);

  // 左侧面板结束拖拽
  const handleLeftPanelMouseUp = useCallback(() => {
    setIsLeftPanelDragging(false);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, []);

  // 使用 useEffect 管理底部面板事件监听
  useEffect(() => {
    if (!isDragging) return;

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);

    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging, handleMouseMove, handleMouseUp]);

  // 使用 useEffect 管理右侧面板事件监听
  useEffect(() => {
    if (!isRightPanelDragging) return;

    window.addEventListener('mousemove', handleRightPanelMouseMove);
    window.addEventListener('mouseup', handleRightPanelMouseUp);

    return () => {
      window.removeEventListener('mousemove', handleRightPanelMouseMove);
      window.removeEventListener('mouseup', handleRightPanelMouseUp);
    };
  }, [isRightPanelDragging, handleRightPanelMouseMove, handleRightPanelMouseUp]);

  // 使用 useEffect 管理左侧面板事件监听
  useEffect(() => {
    if (!isLeftPanelDragging) return;

    window.addEventListener('mousemove', handleLeftPanelMouseMove);
    window.addEventListener('mouseup', handleLeftPanelMouseUp);

    return () => {
      window.removeEventListener('mousemove', handleLeftPanelMouseMove);
      window.removeEventListener('mouseup', handleLeftPanelMouseUp);
    };
  }, [isLeftPanelDragging, handleLeftPanelMouseMove, handleLeftPanelMouseUp]);

  // 底部面板内容延迟渲染：展开动画结束后渲染，收起时立即移除
  useEffect(() => {
    // 清除之前的定时器
    if (bottomPanelTransitionRef.current) {
      clearTimeout(bottomPanelTransitionRef.current);
      bottomPanelTransitionRef.current = null;
    }

    if (isHistoryPanelOpen) {
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
  }, [isHistoryPanelOpen]);

  return (
    <div className="h-screen flex flex-col bg-background text-foreground overflow-hidden">
      {/* ===== 顶部工具栏 ===== */}
      <header className="h-14 shrink-0 border-b border-border flex items-center justify-between px-4 bg-background">
        {/* 左侧：Logo */}
        <div className="flex items-center gap-4" data-guide="app-logo">
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
            <div className="flex items-center gap-2 px-3 py-1 rounded-full bg-accent border border-border">
              <span className="text-sm text-muted-foreground">{flow.name}</span>
              {isDirty && (
                <span className="w-2 h-2 rounded-full bg-amber-400 animate-pulse" title={getLL().flow.unsaved()} />
              )}
            </div>
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
              size="sm"
              onClick={resetZoom}
              className="h-7 min-w-[3.5rem] px-1 text-xs text-muted-foreground"
              title={getLL().common.zoom_reset()}
            >
              {Math.round(zoom * 100)}%
            </Button>
            <Button
              variant="ghost"
              size="icon"
              onClick={zoomIn}
              className="h-7 w-7"
              title={getLL().common.zoom_in()}
            >
              <ZoomIn className="h-4 w-4" />
            </Button>
          </div>
          )}

          {/* 快捷键按钮 */}
          <Button
            variant="ghost"
            size="icon"
            onClick={() => useUIStore.getState().openShortcutsDialog()}
            title={`快捷键 (Ctrl+/)`}
            className="hidden sm:inline-flex"
          >
            <Keyboard className="h-5 w-5" />
          </Button>

          {/* 设置按钮 */}
          <Button
            variant="ghost"
            size="icon"
            onClick={openSettingsDialog}
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
            isLeftPanelDragging && '!transition-none'
          )}
          style={{ width: isFlowListPanelOpen ? sidebarWidth : 0 }}
        >
          {/* 可拖拽调整宽度的 handle - 放在面板内部右边缘 */}
          {isFlowListPanelOpen && (
            <div
              onMouseDown={handleLeftPanelMouseDown}
              className={cn(
                'absolute right-0 top-0 bottom-0 w-2 z-10',
                'bg-transparent hover:bg-border active:bg-input',
                'cursor-col-resize flex items-center justify-center',
                'transition-colors duration-150',
                isLeftPanelDragging && 'bg-input'
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

          {/* ===== 底部面板（历史/控制台） ===== */}
          <div
            className={cn(
              'shrink-0 border-t border-border bg-background overflow-hidden relative',
              'transition-all duration-300 ease-out',
              isDragging && '!transition-none'
            )}
            style={{ height: isHistoryPanelOpen ? bottomPanelHeight : 0 }}
          >
            {/* 可拖拽调整高度的 handle - 放在面板内部上边缘 */}
            <div
              onMouseDown={handleMouseDown}
              className={cn(
                'absolute left-0 right-0 top-0 h-2 z-10',
                'bg-transparent hover:bg-border active:bg-input',
                'cursor-row-resize flex items-center justify-center',
                'transition-colors duration-150',
                isDragging && 'bg-input'
              )}
              title={getLL().common.drag_resize_height()}
            >
              <GripHorizontal className="w-6 h-2.5 text-muted-foreground" />
            </div>
            {/* 内容延迟渲染：展开动画结束后再渲染，避免卡顿 */}
            {bottomPanelContentReady && (
              <div className="h-full flex flex-col pt-2">
                {/* 面板头部：Tab 切换 + 关闭按钮 */}
                <div className="flex items-center justify-between px-3 py-1.5 border-b border-border">
                    <Tabs value={bottomPanelTab} onValueChange={(v) => setBottomPanelTab(v as BottomPanelTab)} className="w-auto">
                      <TabsList className="h-7 p-0.5">
                        <TabsTrigger value="history" className="h-6 px-3 text-xs">
                          {getLL().flow.execution_history()}
                        </TabsTrigger>
                        <TabsTrigger value="running" className="h-6 px-3 text-xs">
                          {getLL().flow.running()}
                        </TabsTrigger>
                        <TabsTrigger value="console" className="h-6 px-3 text-xs">
                          {getLL().flow.console()}
                        </TabsTrigger>
                      </TabsList>
                    </Tabs>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={toggleHistoryPanel}
                      className="h-7 w-7 hover:bg-accent"
                      title={getLL().flow.collapse_panel()}
                    >
                      <ChevronDown className="w-4 h-4 text-muted-foreground" />
                    </Button>
                  </div>

                  {/* Tab 内容 */}
                  <Tabs value={bottomPanelTab} onValueChange={(v) => setBottomPanelTab(v as BottomPanelTab)} className="flex-1 min-h-0">
                    <TabsContent value="history" className="m-0 h-full overflow-hidden">
                      <ExecutionHistory onCompare={handleCompare} />
                    </TabsContent>
                    <TabsContent value="running" className="m-0 h-full overflow-hidden">
                      <ExecutionPanel />
                    </TabsContent>
                    <TabsContent value="console" className="m-0 h-full overflow-hidden flex flex-col">
                      {/* 过滤控件 */}
                      <div className="flex items-center gap-2 px-2 py-1.5 border-b border-border shrink-0">
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
                    </TabsContent>
                  </Tabs>
                </div>
            )}
          </div>

          {/* 底部历史按钮（面板收起时显示）- 在画布底部 */}
          {!isHistoryPanelOpen && (
            <Button
              variant="ghost"
              size="sm"
              onClick={toggleHistoryPanel}
              className={cn(
                'absolute bottom-0 left-1/2 -translate-x-1/2 z-10',
                'h-6 px-3 rounded-t-lg rounded-b-none',
                'bg-background hover:bg-accent border border-b-0 border-border',
                'text-xs text-muted-foreground hover:text-accent-foreground'
              )}
              title={getLL().flow.expand_bottom_panel()}
            >
              <ChevronUp className="w-3.5 h-3.5 mr-1" />
              {getLL().flow.bottom_panel()}
            </Button>
          )}
        </main>

        {/* ===== 右侧配置面板 ===== */}
        <aside
          className={cn(
            'shrink-0 border-l border-border bg-background overflow-hidden relative',
            'transition-all duration-300 ease-out',
            isConfigPanelOpen ? 'min-w-0' : 'w-0 min-w-0',
            isRightPanelDragging && '!transition-none'
          )}
          style={{ width: isConfigPanelOpen ? configPanelWidth : 0 }}
        >
          {/* 可拖拽调整宽度的 handle - 放在面板内部左边缘 */}
          {isConfigPanelOpen && (
            <div
              onMouseDown={handleRightPanelMouseDown}
              className={cn(
                'absolute left-0 top-0 bottom-0 w-2 z-10',
                'bg-transparent hover:bg-border active:bg-input',
                'cursor-col-resize flex items-center justify-center',
                'transition-colors duration-150',
                isRightPanelDragging && 'bg-input'
              )}
              title={getLL().common.drag_resize_width()}
            >
              <GripVertical className="w-2.5 h-6 text-muted-foreground" />
            </div>
          )}
          {isConfigPanelOpen && (
            <div className="h-full min-w-0 overflow-hidden pl-2" style={{ width: configPanelWidth }}>
              {configPanelMode === 'settings' ? (
                <SettingsPanel />
              ) : (
                <NodeConfigPanel readOnly={isRightPanelReadOnly} />
              )}
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

      {/* ===== 全流程搜索面板 ===== */}
      <SearchOverlayHandler />
    </div>
  );
}
