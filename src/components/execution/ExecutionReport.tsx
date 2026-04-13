import { useState, useEffect } from 'react';

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
  Maximize2,
  Minimize2,
  Inbox,
  X,
  Wifi,
  ChevronDown,
  ChevronRight,
} from 'lucide-react';
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
import { NodeExecutionDetails } from './NodeExecutionDetails';
import { NodeListItem } from './NodeListItem';
import { WsMessageTimeline } from './WsMessageTimeline';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';

/** 统计卡片组件 */
function StatCard({
  icon: Icon,
  label,
  value,
  iconColor,
  valueColor,
}: {
  icon: React.ElementType;
  label: string;
  value: string | number;
  iconColor?: string;
  valueColor?: string;
}) {
  return (
    <div className="rounded-xl border border-border/50 bg-gradient-to-br from-background to-muted/30 p-4 transition-all">
      <div className="flex items-center gap-2 mb-2">
        <Icon className={cn('w-4 h-4', iconColor || 'text-muted-foreground')} />
        <span className="text-xs text-muted-foreground">{label}</span>
      </div>
      <p className={cn('text-xl font-bold tabular-nums', valueColor)}>
        {value}
      </p>
    </div>
  );
}

export function ExecutionReport() {
  const report = useExecutionStore((state) => state.report);
  const isReportDialogOpen = useExecutionStore((state) => state.isReportDialogOpen);
  const selectedHistoryReport = useExecutionStore((state) => state.selectedHistoryReport);
  const closeReportDialog = useExecutionStore((state) => state.closeReportDialog);

  const displayReport = selectedHistoryReport ?? report;

  const [selectedNodeUuid, setSelectedNodeUuid] = useState<string | null>(null);
  const [isMaximized, setIsMaximized] = useState(false);
  const [isWsStreamCollapsed, setIsWsStreamCollapsed] = useState(false);

  // 默认选中第一个节点
  useEffect(() => {
    if (displayReport?.node_reports && displayReport.node_reports.length > 0) {
      setSelectedNodeUuid(displayReport.node_reports[0].node_uuid);
    }
  }, [displayReport]);

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
          <DialogTitle className="flex items-center gap-3">
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
            <span>{isDebugMode ? '调试报告' : '执行报告'}</span>

            {/* 调试模式标签 */}
            {isDebugMode && (
              <Badge
                variant="outline"
                className="bg-purple-50 dark:bg-purple-900/50 text-purple-700 dark:text-purple-300 border-purple-200 dark:border-purple-700 font-medium"
              >
                调试
              </Badge>
            )}

            {/* 状态标签 */}
            <Badge
              variant="outline"
              className={cn(
                'ml-auto font-medium',
                isSuccess
                  ? 'bg-emerald-50 dark:bg-emerald-900/50 text-emerald-700 dark:text-emerald-300 border-emerald-200 dark:border-emerald-700'
                  : 'bg-red-50 dark:bg-red-900/50 text-red-700 dark:text-red-300 border-red-200 dark:border-red-700'
            )}>
              {isSuccess ? '成功' : '失败'}
            </Badge>

            {/* 最大化按钮 */}
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

        {/* 主滚动容器：包裹所有内容 */}
        <ScrollArea className="flex-1">
          <div className="flex flex-col gap-5 pr-4 min-w-0 overflow-hidden">
            {/* 流程名称 */}
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Activity className="w-4 h-4" />
              <span>流程: </span>
               <span className="font-medium text-foreground">{displayReport.flow_name}</span>
            </div>

            {/* 统计卡片网格 */}
            <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
              <StatCard
                icon={Timer}
                label="总耗时"
                 value={`${displayReport.duration_ms}ms`}
                iconColor="text-violet-500"
                valueColor="text-violet-600"
              />
              <StatCard
                icon={BarChart3}
                label="执行节点"
                 value={`${displayReport.executed_count}/${displayReport.total_nodes}`}
                iconColor="text-blue-500"
                valueColor="text-blue-600"
              />
              <StatCard
                icon={CheckCircle2}
                label="成功"
                 value={displayReport.statistics.success_count}
                iconColor="text-emerald-500"
                valueColor="text-emerald-600"
              />
              <StatCard
                icon={XCircle}
                label="失败"
                 value={displayReport.statistics.failed_count}
                iconColor="text-red-500"
                valueColor="text-red-600"
              />
            </div>

            {/* 额外统计信息 */}
            <div className="flex items-center gap-4 text-sm">
              <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-muted/50">
                <Clock className="w-3.5 h-3.5 text-muted-foreground" />
                <span className="text-muted-foreground">成功率:</span>
                <span className="font-semibold text-foreground">{successRate}%</span>
              </div>
              {displayReport.statistics.avg_node_duration_ms != null && (
                <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-muted/50">
                  <span className="text-muted-foreground">平均耗时:</span>
                  <span className="font-semibold text-foreground">
                    {displayReport.statistics.avg_node_duration_ms!.toFixed(1)}ms
                  </span>
                </div>
              )}
            </div>

            {/* 流程级错误 */}
            {displayReport.flow_error && (
              <div className="rounded-xl bg-red-50 dark:bg-red-950/50 p-4 border border-red-200/50 dark:border-red-800/50">
                <div className="flex items-center gap-2 mb-2 text-red-600 dark:text-red-400">
                  <AlertCircle className="w-4 h-4" />
                  <span className="font-medium">流程错误</span>
                </div>
                <p className="text-red-600 dark:text-red-400">{displayReport.flow_error.message}</p>
              </div>
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
                    <span className="text-sm font-semibold">WebSocket 消息流</span>
                    <Badge variant="outline" className="text-xs font-normal">{displayReport.ws_streams.length} 个连接</Badge>
                  </div>
                </button>
                {!isWsStreamCollapsed && (
                  <div className="border-t border-border/50">
                    <WsMessageTimeline streams={displayReport.ws_streams} maxHeight="320px" />
                  </div>
                )}
              </div>
            )}

            {/* 节点执行详情 */}
            <div className="min-h-[280px] flex flex-col overflow-hidden">
              <h3 className="text-sm font-semibold text-foreground mb-3 flex items-center gap-2">
                <BarChart3 className="w-4 h-4" />
                节点执行详情
              </h3>
              <div className="flex-1 min-h-0">
                <ResizablePanel
                  leftPanel={
                    displayReport.node_reports.length === 0 ? (
                      <div className="flex-1 flex flex-col items-center justify-center text-muted-foreground gap-2 h-full">
                        <Inbox className="w-12 h-12" />
                        <span>暂无执行节点</span>
                      </div>
                    ) : (
                      <ScrollArea className="h-full w-full">
                        <div className="space-y-1 p-1">
                          {displayReport.node_reports.map((nodeReport) => (
                            <NodeListItem
                              key={nodeReport.node_uuid}
                              report={nodeReport}
                              isSelected={selectedNodeUuid === nodeReport.node_uuid}
                              onClick={() => setSelectedNodeUuid(nodeReport.node_uuid)}
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
                        <span>暂无执行节点</span>
                      </div>
                    ) : selectedNodeUuid ? (
                      <ScrollArea className="h-full w-full">
                        <div className="p-2 min-w-0">
                          <NodeExecutionDetails
                            report={displayReport.node_reports.find(r => r.node_uuid === selectedNodeUuid)!}
                          />
                        </div>
                      </ScrollArea>
                    ) : (
                      <div className="flex items-center justify-center h-full text-muted-foreground">
                        请选择节点查看详情
                      </div>
                    )
                  }
                  defaultWidth={300}
                  minWidth={200}
                  maxWidth={500}
                />
              </div>
            </div>
          </div>
        </ScrollArea>
      </DialogContent>
    </Dialog>
  );
}
