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
  const reset = useExecutionStore((state) => state.reset);

  const [selectedNodeUuid, setSelectedNodeUuid] = useState<string | null>(null);
  const [isMaximized, setIsMaximized] = useState(false);

  // 默认选中第一个节点
  useEffect(() => {
    if (report?.node_reports && report.node_reports.length > 0) {
      setSelectedNodeUuid(report.node_reports[0].node_uuid);
    }
  }, [report]);

  // 关闭弹窗时重置状态
  const handleClose = () => {
    setIsMaximized(false);
    reset();
  };

  if (!report) return null;

  const isSuccess = report.status === 'success';
  const successRate =
    report.total_nodes > 0
      ? ((report.statistics.success_count / report.total_nodes) * 100).toFixed(0)
      : 0;

  return (
    <Dialog open={!!report} onOpenChange={(open) => !open && handleClose()}>
      <DialogContent
        className={cn(
          "max-w-3xl max-h-[85vh] flex flex-col",
          isMaximized && "!fixed !inset-4 !z-[60] !w-[calc(100vw-2rem)] !h-[calc(100vh-2rem)] !max-w-none !max-h-none !translate-x-0 !translate-y-0 bg-background rounded-lg border shadow-2xl"
        )}
        hideCloseButton
      >
        <DialogHeader>
          <DialogTitle className="flex items-center gap-3">
            {/* 状态图标 */}
            <div
              className={cn(
                'p-2 rounded-full',
                isSuccess
                  ? 'bg-emerald-100'
                  : 'bg-red-100'
            )}>
              {isSuccess ? (
                <CheckCircle2 className="w-5 h-5 text-emerald-600" />
              ) : (
                <XCircle className="w-5 h-5 text-red-600" />
              )}
            </div>
            {/* 标题文字 */}
            <span>执行报告</span>

            {/* 状态标签 */}
            <Badge
              variant="outline"
              className={cn(
                'ml-auto font-medium',
                isSuccess
                  ? 'bg-emerald-50 text-emerald-700 border-emerald-200'
                  : 'bg-red-50 text-red-700 border-red-200'
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

        <div className="flex-1 overflow-hidden flex flex-col gap-5">
          {/* 流程名称 */}
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Activity className="w-4 h-4" />
            <span>流程: </span>
            <span className="font-medium text-foreground">{report.flow_name}</span>
          </div>

          {/* 统计卡片网格 */}
          <div className="grid grid-cols-4 gap-3">
            <StatCard
              icon={Timer}
              label="总耗时"
              value={`${report.duration_ms}ms`}
              iconColor="text-violet-500"
              valueColor="text-violet-600"
            />
            <StatCard
              icon={BarChart3}
              label="执行节点"
              value={`${report.executed_count}/${report.total_nodes}`}
              iconColor="text-blue-500"
              valueColor="text-blue-600"
            />
            <StatCard
              icon={CheckCircle2}
              label="成功"
              value={report.statistics.success_count}
              iconColor="text-emerald-500"
              valueColor="text-emerald-600"
            />
            <StatCard
              icon={XCircle}
              label="失败"
              value={report.statistics.failed_count}
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
            {report.statistics.avg_node_duration_ms !== undefined && (
              <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-muted/50">
                <span className="text-muted-foreground">平均耗时:</span>
                <span className="font-semibold text-foreground">
                  {report.statistics.avg_node_duration_ms.toFixed(1)}ms
                </span>
              </div>
            )}
          </div>

          {/* 流程级错误 */}
          {report.flow_error && (
            <div className="rounded-xl bg-red-50 p-4 border border-red-200/50">
              <div className="flex items-center gap-2 mb-2 text-red-600">
                <AlertCircle className="w-4 h-4" />
                <span className="font-medium">流程错误</span>
              </div>
              <p className="text-red-600">{report.flow_error.message}</p>
            </div>
          )}
          {/* 节点执行详情 */}
          <div className="flex-1 min-h-0 flex flex-col">
            <h3 className="text-sm font-semibold text-foreground mb-3 flex items-center gap-2 flex-shrink-0">
              <BarChart3 className="w-4 h-4" />
              节点执行详情
            </h3>
            <div className="flex-1 min-h-0">
              <ResizablePanel
                leftPanel={
                  report.node_reports.length === 0 ? (
                    <div className="flex-1 flex flex-col items-center justify-center text-muted-foreground gap-2 h-full">
                      <Inbox className="w-12 h-12" />
                      <span>暂无执行节点</span>
                    </div>
                  ) : (
                    <ScrollArea className="h-full">
                      <div className="space-y-1 p-1">
                        {report.node_reports.map((nodeReport) => (
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
                  report.node_reports.length === 0 ? (
                    <div className="flex-1 flex flex-col items-center justify-center text-muted-foreground gap-2 h-full">
                      <Inbox className="w-12 h-12" />
                      <span>暂无执行节点</span>
                    </div>
                  ) : selectedNodeUuid ? (
                    <ScrollArea className="h-full">
                      <div className="p-2">
                        <NodeExecutionDetails
                          report={report.node_reports.find(r => r.node_uuid === selectedNodeUuid)!}
                        />
                      </div>
                    </ScrollArea>
                  ) : (
                    <div className="flex items-center justify-center h-full text-muted-foreground">
                      请选择节点查看详情
                    </div>
                  )
                }
                defaultWidth={280}
                minWidth={180}
                maxWidth={400}
              />
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
