import { useState, useEffect, useMemo } from 'react';
import { getLL } from '@/i18n/i18n-lazy';

/**
 * 执行报告弹窗组件
 * 显示流程执行的完整报告，包括统计信息和节点详情
 */
import {
  CheckCircle2,
  XCircle,
  Clock,
  BarChart3,
  Timer,
  Activity,
  AlertCircle,
  AlertTriangle,
  Maximize2,
  Minimize2,
  Inbox,
  X,
  Wifi,
  ChevronDown,
  ChevronRight,
  Sparkles,
  FileDown,
  FileText,
} from 'lucide-react';
import { toast } from 'sonner';
import { isTauriEnv } from '@/lib/platform';
import { useEnvironmentStore } from '@/stores/environment';
import { getFlowReportMarkdown, exportFlowReportHtml } from '@/services/tauri';
import { save } from '@tauri-apps/plugin-dialog';
import { revealItemInDir } from '@/services/tauri/settings';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { ResizablePanel } from '@/components/ui/ResizablePanel';
import { useExecutionStore } from '@/stores';
import { useAIStore } from '@/stores/ai';
import { NodeExecutionDetails } from './NodeExecutionDetails';
import { NodeListItem } from './NodeListItem';
import { WsMessageTimeline } from './WsMessageTimeline';
import { CanvasReportViewer } from './CanvasReportViewer';
import { cn } from '@/lib/utils';
import { findNodeReportInTree } from '@/lib/reportTree';
import { Button } from '@/components/ui/button';
import { CopyButton, StagesSection } from '@/components/common';
/** 统计卡片组件 */
function StatCard({
  icon: Icon,
  label,
  value,
  iconColor,
  iconBg,
  valueColor,
}: {
  icon: React.ElementType;
  label: string;
  value: string | number;
  iconColor?: string;
  iconBg?: string;
  valueColor?: string;
}) {
  return (
    <div className="rounded-lg border border-border/50 bg-muted/20 hover:bg-muted/30 px-3 py-2 transition-colors min-w-0">
      <div className="flex items-center gap-2.5">
        <div className={cn('p-1.5 rounded-md shrink-0', iconBg || 'bg-muted/60')}>
          <Icon className={cn('w-3.5 h-3.5', iconColor || 'text-muted-foreground')} />
        </div>
        <div className="min-w-0 flex-1">
          <p className="text-[11px] text-muted-foreground leading-none truncate font-medium">{label}</p>
          <p className={cn('text-sm font-bold tabular-nums leading-tight mt-1 truncate', valueColor)}>
            {value}
          </p>
        </div>
      </div>
    </div>
  );
}

export function ExecutionReport() {
  const report = useExecutionStore((state) => state.report);
  const isReportDialogOpen = useExecutionStore((state) => state.isReportDialogOpen);
  const selectedHistoryReport = useExecutionStore((state) => state.selectedHistoryReport);
  const closeReportDialog = useExecutionStore((state) => state.closeReportDialog);

  const displayReport = selectedHistoryReport ?? report;

  const ll = getLL();

  const [selectedNodeUuid, setSelectedNodeUuid] = useState<string | null>(null);
  const [isMaximized, setIsMaximized] = useState(false);
  const [isWsStreamCollapsed, setIsWsStreamCollapsed] = useState(false);
  const [isStatsCollapsed, setIsStatsCollapsed] = useState(false);
  const [viewMode, setViewMode] = useState<'list' | 'canvas'>('list');
  const [filterStatus, setFilterStatus] = useState<'all' | 'failed'>('all');

  // 在整棵报告树（含任意层级子流程）中检索当前选中的节点报告
  const activeReport = useMemo(() => {
    if (!selectedNodeUuid) return null;
    return findNodeReportInTree(displayReport?.node_reports, selectedNodeUuid);
  }, [displayReport, selectedNodeUuid]);

  const failedNodes = useMemo(() => {
    return displayReport?.node_reports.filter((n) => n.status === 'failed') ?? [];
  }, [displayReport?.node_reports]);

  const filteredNodeReports = useMemo(() => {
    if (!displayReport?.node_reports) return [];
    if (filterStatus === 'failed') {
      return failedNodes;
    }
    return displayReport.node_reports;
  }, [displayReport?.node_reports, filterStatus, failedNodes]);

  const [canvasSelectedUuid, setCanvasSelectedUuid] = useState<string | null>(null);

  // 默认选中：优先选中首个失败节点，若全部成功则选中第一个节点（更符合人类排障直觉）
  useEffect(() => {
    if (displayReport?.node_reports && displayReport.node_reports.length > 0) {
      const firstFailed = displayReport.node_reports.find((n) => n.status === 'failed');
      const initialUuid = firstFailed?.node_uuid ?? displayReport.node_reports[0].node_uuid;
      setSelectedNodeUuid(initialUuid);
      setCanvasSelectedUuid(initialUuid);
    }
  }, [displayReport]);
  // 弹窗内按需拉取选中的节点详情（若为脱敏状态）
  useEffect(() => {
    if (selectedNodeUuid && activeReport?.is_detail_stripped) {
      const flowUuid = displayReport?.flow_uuid;
      if (flowUuid) {
        void useExecutionStore.getState().fetchNodeDetail(flowUuid, selectedNodeUuid);
      }
    }
  }, [selectedNodeUuid, activeReport?.is_detail_stripped, displayReport?.flow_uuid]);


  // 关闭弹窗时重置状态
  const handleClose = () => {
    setIsMaximized(false);
    closeReportDialog();
  };

  if (!isReportDialogOpen || !displayReport) return null;

  const isSuccess = displayReport.status === 'success';
  const isDebugMode = displayReport.execution_mode === 'debug';
  const successRate =
    displayReport.total_nodes > 0
      ? ((displayReport.statistics.success_count / displayReport.total_nodes) * 100).toFixed(0)
      : 0;
  const handleDiagnoseWithAI = () => {
    useAIStore.getState().setCopilotOpen(true);
    const failedNodePrompts = failedNodes.map(
      (nr) => `[${nr.node_name}] (${nr.node_type}: ${nr.error?.message || 'failed'})`,
    );
    const prompt =
      failedNodePrompts.length > 0
        ? ll.ai.copilot.prompt_diagnose_flow_failures({
            failedNodes: failedNodePrompts.join('; '),
          })
        : ll.ai.copilot.prompt_diagnose_flow_generic();
    useAIStore.getState().sendMessage(prompt);
  };
  const handleCopyMarkdown = async () => {
    if (!displayReport) return;
    try {
      const activeEnv = useEnvironmentStore.getState().activeEnvironmentName;
      const md = await getFlowReportMarkdown(displayReport, activeEnv ?? undefined);
      await navigator.clipboard.writeText(md);
      toast.success(ll.execution.report.copy_markdown_success());
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  };

  const handleExportHtml = async () => {
    if (!displayReport) return;
    try {
      const activeEnv = useEnvironmentStore.getState().activeEnvironmentName;
      if (isTauriEnv()) {
        const defaultFilename = `flow-report-${displayReport.flow_name.replace(/[/\\:*?"<>|]/g, '_')}-${new Date().toISOString().replace(/[:.]/g, '-')}.html`;
        const selectedPath = await save({
          defaultPath: defaultFilename,
          filters: [{ name: 'HTML Report (*.html)', extensions: ['html'] }],
        });
        if (!selectedPath) return;
        await exportFlowReportHtml(displayReport, selectedPath, activeEnv ?? undefined);
        toast.success(ll.execution.report.export_html_success(), {
          action: {
            label: getLL().flow.reveal_in_file_manager(),
            onClick: () => {
              revealItemInDir(selectedPath).catch(() => {});
            },
          },
        });
      } else {
        const htmlContent = `<!DOCTYPE html><html><head><meta charset="utf-8"><title>${displayReport.flow_name}</title></head><body><h1>${displayReport.flow_name}</h1><pre>${JSON.stringify(displayReport, null, 2)}</pre></body></html>`;
        const blob = new Blob([htmlContent], { type: 'text/html;charset=utf-8' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `flow-report-${displayReport.flow_name.replace(/[/\\:*?"<>|]/g, '_')}.html`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        toast.success(ll.execution.report.export_html_success());
      }
    } catch (e) {
      toast.error(ll.execution.report.export_html_failed());
    }
  };

  return (
    <Dialog open={isReportDialogOpen} onOpenChange={(open) => !open && handleClose()}>
      <DialogContent
        className={cn(
          "w-[90vw] max-w-[1400px] h-[85vh] flex flex-col overflow-hidden p-6 gap-4",
          isMaximized && "!fixed !inset-4 !z-[60] !w-[calc(100vw-2rem)] !h-[calc(100vh-2rem)] !max-w-none !max-h-none !translate-x-0 !translate-y-0 bg-background rounded-lg border shadow-2xl"
        )}
        hideCloseButton
      >
        <DialogHeader className="flex-shrink-0">
          <DialogTitle className="flex items-center gap-2">
            {/* 状态图标 */}
            <div
              className={cn(
                'p-2 rounded-full',
                isSuccess
                  ? 'bg-emerald-100 dark:bg-emerald-900/50'
                  : 'bg-red-100 dark:bg-red-900/50'
              )}>
              {isSuccess ? (
                <CheckCircle2 className="w-5 h-5 text-emerald-600 dark:text-emerald-400" />
              ) : (
                <XCircle className="w-5 h-5 text-red-600 dark:text-red-400" />
              )}
            </div>
            {/* 标题文字 */}
            <span>{isDebugMode ? ll.execution.report.debug_title() : ll.execution.report.title()}</span>

            {/* 调试模式标签 */}
            {isDebugMode && (
              <Badge
                variant="outline"
                className="h-8 px-3 text-xs font-medium inline-flex items-center bg-purple-50 dark:bg-purple-900/50 text-purple-700 dark:text-purple-300 border-purple-200 dark:border-purple-700"
              >
                {ll.execution.report.badge_debug()}
              </Badge>
            )}

            {/* 状态标签 */}
            <Badge
              variant="outline"
              className={cn(
                'ml-auto h-8 px-3 text-xs font-medium inline-flex items-center',
                isSuccess
                  ? 'bg-emerald-50 dark:bg-emerald-900/50 text-emerald-700 dark:text-emerald-300 border-emerald-200 dark:border-emerald-700'
                  : 'bg-red-50 dark:bg-red-900/50 text-red-700 dark:text-red-300 border-red-200 dark:border-red-700'
            )}>
              {isSuccess ? ll.execution.status.success() : ll.execution.status.failed()}
            </Badge>

            {/* AI 诊断与修复按钮 */}
            {!isSuccess && (
              <Button
                variant="outline"
                size="sm"
                className="h-8 px-3 gap-1.5 text-xs font-medium border-red-300 dark:border-red-800 bg-red-50/80 dark:bg-red-950/40 text-red-700 dark:text-red-300 hover:bg-red-100 dark:hover:bg-red-900/60"
                onClick={handleDiagnoseWithAI}
                title={ll.ai.copilot.diagnose_and_fix_flow()}
              >
                <Sparkles className="w-3.5 h-3.5 text-amber-500 fill-amber-500/20" />
                <span>{ll.ai.copilot.diagnose_and_fix_flow()}</span>
              </Button>
            )}

            {/* 视图模式切换 */}
            <div className="inline-flex h-8 items-center p-0.5 bg-muted/70 rounded-lg border border-border/40 gap-0.5">
              <button
                type="button"
                onClick={() => setViewMode('list')}
                className={cn(
                  'inline-flex items-center justify-center whitespace-nowrap rounded-md px-3 h-7 text-xs font-medium transition-all cursor-pointer',
                  viewMode === 'list'
                    ? 'bg-background text-foreground shadow-xs font-semibold'
                    : 'text-muted-foreground hover:text-foreground'
                )}
                disabled={!displayReport?.flow_snapshot && viewMode === 'canvas'}
                title={!displayReport?.flow_snapshot ? ll.execution.report.no_topology_data() : undefined}
              >
                {ll.execution.report.list_view()}
              </button>
              <button
                type="button"
                onClick={() => setViewMode('canvas')}
                className={cn(
                  'inline-flex items-center justify-center whitespace-nowrap rounded-md px-3 h-7 text-xs font-medium transition-all cursor-pointer',
                  viewMode === 'canvas'
                    ? 'bg-background text-foreground shadow-xs font-semibold'
                    : 'text-muted-foreground hover:text-foreground'
                )}
                disabled={!displayReport?.flow_snapshot}
                title={!displayReport?.flow_snapshot ? ll.execution.report.no_topology_data() : undefined}
              >
                {ll.execution.report.canvas_view()}
              </button>
            </div>

            {/* 复制 Markdown 摘要 */}
            <Button
              variant="outline"
              size="sm"
              className="h-8 px-3 gap-1.5 text-xs font-medium"
              onClick={handleCopyMarkdown}
              title={ll.execution.report.copy_markdown()}
            >
              <FileText className="w-3.5 h-3.5" />
              <span>{ll.execution.report.copy_markdown()}</span>
            </Button>

            {/* 导出 HTML 报告 */}
            <Button
              variant="outline"
              size="sm"
              className="h-8 px-3 gap-1.5 text-xs font-medium"
              onClick={handleExportHtml}
              title={ll.execution.report.export_html()}
            >
              <FileDown className="w-3.5 h-3.5" />
              <span>{ll.execution.report.export_html()}</span>
            </Button>

            {/* 复制报告 JSON */}
            <CopyButton
              value={displayReport}
              format="json"
              label={ll.execution.report.copy_all()}
              title={ll.execution.report.copy_all()}
              size="sm"
              className="h-8 px-3"
            />
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={() => setIsMaximized(!isMaximized)}
            >
              {isMaximized ? (
                <Minimize2 className="h-4 w-4" />
              ) : (
                <Maximize2 className="h-4 w-4" />
              )}
            </Button>

            {/* 关闭按钮 */}
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={handleClose}
            >
              <X className="h-4 w-4" />
            </Button>
          </DialogTitle>
        </DialogHeader>

        {/* 主容器：固定布局 */}
        <div className="flex-1 flex flex-col overflow-hidden gap-4">
          {/* 上半部分：统计信息（可折叠，控制紧凑高度，把最大视口留给节点报告） */}
          <ScrollArea className={cn("flex-shrink-0", isStatsCollapsed ? "max-h-[60px]" : "max-h-[220px]")}>
            <div className="flex flex-col gap-4 pr-4 min-w-0">
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <button
                  type="button"
                  onClick={() => setIsStatsCollapsed(!isStatsCollapsed)}
                  className="flex items-center gap-1 hover:text-foreground transition-colors"
                >
                  {isStatsCollapsed ? (
                    <ChevronRight className="w-4 h-4" />
                  ) : (
                    <ChevronDown className="w-4 h-4" />
                  )}
                  <Activity className="w-4 h-4" />
                </button>
                <span>{ll.execution.report.flow_label()}</span>
                 <span className="font-medium text-foreground">{displayReport.flow_name}</span>

                {/* 折叠时显示摘要信息 */}
                {isStatsCollapsed && (
                  <div className="flex items-center gap-3 ml-auto text-xs">
                    <span className="text-muted-foreground">{displayReport.duration_ms}ms</span>
                    <span className="text-muted-foreground">
                      {displayReport.executed_count}/{displayReport.total_nodes}
                    </span>
                    <span className={cn(
                      "font-medium",
                      isSuccess ? "text-emerald-600" : "text-red-600"
                    )}>
                      {successRate}%
                    </span>
                  </div>
                )}
              </div>

              {/* 展开时显示完整统计 */}
              {!isStatsCollapsed && (
                <>
                  {/* 统计指标紧凑单层网格 */}
                  <div className={cn(
                    "grid gap-2",
                    displayReport.statistics.avg_node_duration_ms != null
                      ? "grid-cols-2 sm:grid-cols-3 lg:grid-cols-6"
                      : "grid-cols-2 sm:grid-cols-3 lg:grid-cols-5"
                  )}>
                    <StatCard
                      icon={Timer}
                      label={ll.execution.report.total_duration()}
                      value={`${displayReport.duration_ms}ms`}
                      iconColor="text-violet-500"
                      iconBg="bg-violet-50 dark:bg-violet-950/50"
                    />
                    <StatCard
                      icon={BarChart3}
                      label={ll.execution.report.executed_nodes()}
                      value={`${displayReport.executed_count}/${displayReport.total_nodes}`}
                      iconColor="text-blue-500"
                      iconBg="bg-blue-50 dark:bg-blue-950/50"
                    />
                    <StatCard
                      icon={CheckCircle2}
                      label={ll.execution.status.success()}
                      value={displayReport.statistics.success_count}
                      iconColor="text-emerald-500"
                      iconBg="bg-emerald-50 dark:bg-emerald-950/50"
                      valueColor="text-emerald-600 dark:text-emerald-400"
                    />
                    <StatCard
                      icon={XCircle}
                      label={ll.execution.status.failed()}
                      value={displayReport.statistics.failed_count}
                      iconColor="text-red-500"
                      iconBg="bg-red-50 dark:bg-red-950/50"
                      valueColor={displayReport.statistics.failed_count > 0 ? "text-red-600 dark:text-red-400" : undefined}
                    />
                    <StatCard
                      icon={Clock}
                      label={ll.execution.report.success_rate()}
                      value={`${successRate}%`}
                      iconColor="text-teal-500"
                      iconBg="bg-teal-50 dark:bg-teal-950/50"
                      valueColor={isSuccess ? "text-emerald-600 dark:text-emerald-400" : "text-amber-600 dark:text-amber-400"}
                    />
                    {displayReport.statistics.avg_node_duration_ms != null && (
                      <StatCard
                        icon={Activity}
                        label={ll.execution.report.avg_duration()}
                        value={`${displayReport.statistics.avg_node_duration_ms!.toFixed(1)}ms`}
                        iconColor="text-indigo-500"
                        iconBg="bg-indigo-50 dark:bg-indigo-950/50"
                      />
                    )}
                  </div>
                  {/* 流程级错误 */}
                  {displayReport.flow_error && (
                    <div className="rounded-xl bg-red-50 dark:bg-red-950/50 p-4 border border-red-200/50 dark:border-red-800/50">
                      <div className="flex items-center justify-between gap-2 mb-2 text-red-600 dark:text-red-400">
                        <div className="flex items-center gap-2">
                          <AlertCircle className="w-4 h-4" />
                          <span className="font-medium">{ll.execution.report.flow_error()}</span>
                        </div>
                        <CopyButton
                          value={displayReport.flow_error.message}
                          title={ll.execution.viewers.copy_error_message()}
                          size="xs"
                          className="bg-background/80 text-red-700 dark:text-red-300 border-red-300/60"
                        />
                      </div>
                      <p className="text-red-600 dark:text-red-400 select-text cursor-text">{displayReport.flow_error.message}</p>
                    </div>
                  )}

                  {/* End 节点状态 / 流程级警告 */}
                  {/* 当有 flow_warning（如"无 End 节点"）时，只显示 flow_warning 避免重复提示 */}
                  {displayReport.flow_warning ? (
                    <div className="flex items-center gap-2 px-3 py-2 rounded-md text-sm bg-yellow-50 text-yellow-700 dark:bg-yellow-950/30 dark:text-yellow-400 border border-yellow-200 dark:border-yellow-800">
                      <AlertTriangle className="w-4 h-4 shrink-0" />
                      <span>{displayReport.flow_warning}</span>
                    </div>
                  ) : displayReport.end_node_reached !== undefined ? (
                    <div className={cn(
                      'flex items-center gap-2 px-3 py-2 rounded-md text-sm',
                      displayReport.end_node_reached
                        ? 'bg-green-50 text-green-700 dark:bg-green-950/30 dark:text-green-400 border border-green-200 dark:border-green-800'
                        : 'bg-red-50 text-red-700 dark:bg-red-950/30 dark:text-red-400 border border-red-200 dark:border-red-800'
                    )}>
                      <span className="text-lg">{displayReport.end_node_reached ? '🟢' : '🔴'}</span>
                      <span>{displayReport.end_node_reached ? ll.execution.report.end_node_reached() : ll.execution.report.end_node_not_reached()}</span>
                    </div>
                  ) : (
                    <div className="flex items-center gap-2 px-3 py-2 rounded-md text-sm bg-yellow-50 text-yellow-700 dark:bg-yellow-950/30 dark:text-yellow-400 border border-yellow-200 dark:border-yellow-800">
                      <span className="text-lg">🟡</span>
                      <span>{ll.execution.report.no_end_node_hint()}</span>
                    </div>
                  )}

                  {/* 生命周期阶段执行记录（Setup / Main / Teardown） */}
                  {displayReport.stages && (
                    <StagesSection stages={displayReport.stages} />
                  )}

                  {/* WebSocket 消息流 */}
                  {displayReport.ws_streams && displayReport.ws_streams.length > 0 && (
                    <div className="rounded-xl border border-border/50 overflow-hidden">
                      <button
                        type="button"
                        onClick={() => setIsWsStreamCollapsed(!isWsStreamCollapsed)}
                        className="w-full flex items-center justify-between px-4 py-2.5 hover:bg-muted/30 transition-colors text-left"
                      >
                        <div className="flex items-center gap-2.5">
                          {isWsStreamCollapsed ? (
                            <ChevronRight className="w-4 h-4 text-muted-foreground" />
                          ) : (
                            <ChevronDown className="w-4 h-4 text-muted-foreground" />
                          )}
                          <Wifi className="w-4 h-4 text-violet-500" />
                          <span className="text-sm font-semibold">{ll.execution.report.ws_stream_title()}</span>
                          <Badge variant="outline" className="text-xs font-normal">{ll.execution.report.ws_connections({ count: displayReport.ws_streams.length })}</Badge>
                        </div>
                      </button>
                      {!isWsStreamCollapsed && (
                        <div className="border-t border-border/50">
                          <WsMessageTimeline streams={displayReport.ws_streams} maxHeight="320px" />
                        </div>
                      )}
                    </div>
                  )}
                </>
              )}
            </div>
          </ScrollArea>

          {/* 下半部分：节点执行详情（固定高度，独立滚动） */}
          {viewMode === 'canvas' ? (
            <div className="flex-1 min-h-0">
              <CanvasReportViewer
                report={displayReport}
                selectedNodeUuid={canvasSelectedUuid}
                onNodeSelect={setCanvasSelectedUuid}
              />
            </div>
          ) : (
            <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
              <div className="flex items-center justify-between mb-2 flex-shrink-0">
                <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                  <BarChart3 className="w-4 h-4" />
                  <span>{ll.execution.report.node_details()}</span>
                </h3>

                {/* 失败节点快捷过滤 */}
                {failedNodes.length > 0 && (
                  <div className="inline-flex h-7 items-center p-0.5 bg-muted/70 rounded-md border border-border/40 text-xs">
                    <button
                      type="button"
                      onClick={() => setFilterStatus('all')}
                      className={cn(
                        'px-2 py-0.5 rounded text-[11px] font-medium transition-all cursor-pointer',
                        filterStatus === 'all'
                          ? 'bg-background text-foreground shadow-xs font-semibold'
                          : 'text-muted-foreground hover:text-foreground'
                      )}
                    >
                      {ll.execution.report.filter_all()} ({displayReport.node_reports.length})
                    </button>
                    <button
                      type="button"
                      onClick={() => setFilterStatus('failed')}
                      className={cn(
                        'px-2 py-0.5 rounded text-[11px] font-medium transition-all cursor-pointer flex items-center gap-1',
                        filterStatus === 'failed'
                          ? 'bg-red-100 text-red-700 dark:bg-red-950/60 dark:text-red-300 shadow-xs font-semibold'
                          : 'text-red-600 dark:text-red-400 hover:text-red-700'
                      )}
                    >
                      <XCircle className="w-3 h-3" />
                      <span>{ll.execution.report.filter_failed()} ({failedNodes.length})</span>
                    </button>
                  </div>
                )}
              </div>

              <div className="flex-1 min-h-0">
                <ResizablePanel
                  leftPanel={
                    filteredNodeReports.length === 0 ? (
                      <div className="flex-1 flex flex-col items-center justify-center text-muted-foreground gap-2 h-full text-xs">
                        <Inbox className="w-8 h-8 opacity-60" />
                        <span>{ll.execution.report.no_nodes()}</span>
                      </div>
                    ) : (
                      <ScrollArea className="h-full w-full">
                        <div className="space-y-1 p-0.5 pr-2">
                          {filteredNodeReports.map((nodeReport, idx) => (
                            <NodeListItem
                              key={nodeReport.node_uuid}
                              report={nodeReport}
                              indexPrefix={`${idx + 1}`}
                              isSelected={selectedNodeUuid === nodeReport.node_uuid}
                              onClick={() => setSelectedNodeUuid(nodeReport.node_uuid)}
                              selectedNodeUuid={selectedNodeUuid}
                              onSelectNode={setSelectedNodeUuid}
                            />
                          ))}
                        </div>
                      </ScrollArea>
                    )
                  }
                  rightPanel={
                    displayReport.node_reports.length === 0 ? (
                      <div className="flex-1 flex flex-col items-center justify-center text-muted-foreground gap-2 h-full">
                        <Inbox className="w-12 h-12" />
                        <span>{ll.execution.report.no_nodes()}</span>
                      </div>
                    ) : activeReport ? (
                      <ScrollArea className="h-full w-full">
                        <div className="p-1 min-w-0">
                          <NodeExecutionDetails
                            report={activeReport}
                          />
                        </div>
                      </ScrollArea>
                    ) : (
                      <div className="flex items-center justify-center h-full text-muted-foreground">
                        {ll.execution.report.select_node_hint()}
                      </div>
                    )
                  }
                  defaultWidth={320}
                  minWidth={220}
                  maxWidth={560}
                />
              </div>
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
