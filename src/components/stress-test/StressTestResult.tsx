/**
 * 压力测试结果组件
 * 显示压力测试完成后的详细报告
 */
import {
  CheckCircle2,
  XCircle,
  Clock,
  BarChart3,
  Timer,
  Activity,
  AlertCircle,
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
import { Button } from '@/components/ui/button';
import {
  useStressTestStore,
  selectStressTestReport,
} from '@/stores/stressTest';
import { useFlowStore } from '@/stores';
import { cn } from '@/lib/utils';

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

/** 响应时间指标项 */
function MetricItem({
  label,
  value,
  unit = 'ms',
}: {
  label: string;
  value: number;
  unit?: string;
}) {
  return (
    <div className="flex items-center justify-between py-2 px-3 rounded-lg bg-muted/30">
      <span className="text-sm text-muted-foreground">{label}</span>
      <span className="text-sm font-semibold tabular-nums">
        {value.toFixed(1)} {unit}
      </span>
    </div>
  );
}

export function StressTestResult() {
  const report = useStressTestStore(selectStressTestReport);
  const isReportDialogOpen = useStressTestStore((state) => state.isReportDialogOpen);
  const closeReportDialog = useStressTestStore((state) => state.closeReportDialog);
  const unlockFlow = useFlowStore((state) => state.unlockFlow);

  // 仅在弹窗打开且有报告时显示
  if (!isReportDialogOpen || !report) {
    return null;
  }

  const isSuccess = report.statistics.successRate >= 0.99;

  // 关闭弹窗
  const handleClose = () => {
    closeReportDialog();
    unlockFlow();
  };

  return (
    <Dialog open={isReportDialogOpen} onOpenChange={(open) => !open && handleClose()}>
      <DialogContent
        className="max-w-3xl h-[85vh] flex flex-col overflow-hidden p-6 gap-4"
        hideCloseButton
      >
        <DialogHeader className="flex-shrink-0">
          <DialogTitle className="flex items-center gap-3">
            {/* 状态图标 */}
            <div
              className={cn(
                'p-2 rounded-full',
                isSuccess ? 'bg-emerald-100' : 'bg-red-100'
              )}
            >
              {isSuccess ? (
                <CheckCircle2 className="w-5 h-5 text-emerald-600" />
              ) : (
                <XCircle className="w-5 h-5 text-red-600" />
              )}
            </div>
            {/* 标题文字 */}
            <span>压测结果</span>

            {/* 状态标签 */}
            <Badge
              variant="outline"
              className={cn(
                'ml-auto font-medium',
                isSuccess
                  ? 'bg-emerald-50 text-emerald-700 border-emerald-200'
                  : 'bg-red-50 text-red-700 border-red-200'
              )}
            >
              {isSuccess ? '成功' : '有错误'}
            </Badge>

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

        {/* 主滚动容器 */}
        <ScrollArea className="flex-1">
          <div className="flex flex-col gap-5 pr-4">
            {/* 流程名称和持续时间 */}
            <div className="flex items-center gap-4 text-sm text-muted-foreground">
              <div className="flex items-center gap-2">
                <Activity className="w-4 h-4" />
                <span>流程: </span>
                <span className="font-medium text-foreground">
                  {report.flowName}
                </span>
              </div>
              <div className="flex items-center gap-2">
                <Clock className="w-4 h-4" />
                <span>持续: </span>
                <span className="font-medium text-foreground">
                  {(report.durationMs / 1000).toFixed(1)}s
                </span>
              </div>
            </div>

            {/* 统计卡片网格 */}
            <div className="grid grid-cols-3 gap-3">
              <StatCard
                icon={BarChart3}
                label="总执行次数"
                value={report.statistics.totalIterations}
                iconColor="text-blue-500"
                valueColor="text-blue-600"
              />
              <StatCard
                icon={CheckCircle2}
                label="成功率"
                value={`${(report.statistics.successRate * 100).toFixed(1)}%`}
                iconColor="text-emerald-500"
                valueColor="text-emerald-600"
              />
              <StatCard
                icon={Timer}
                label="实际 RPS"
                value={report.statistics.rps.toFixed(2)}
                iconColor="text-violet-500"
                valueColor="text-violet-600"
              />
            </div>

            {/* 响应时间分布 */}
            <div className="flex flex-col gap-3">
              <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                <Timer className="w-4 h-4" />
                响应时间分布
              </h3>
              <div className="grid grid-cols-2 gap-2">
                <MetricItem label="平均" value={report.responseTimeDistribution.avgMs} />
                <MetricItem label="最小" value={report.responseTimeDistribution.minMs} />
                <MetricItem label="最大" value={report.responseTimeDistribution.maxMs} />
                <MetricItem label="P50" value={report.responseTimeDistribution.p50Ms} />
                <MetricItem label="P90" value={report.responseTimeDistribution.p90Ms} />
                <MetricItem label="P95" value={report.responseTimeDistribution.p95Ms} />
                <MetricItem label="P99" value={report.responseTimeDistribution.p99Ms} />
              </div>
            </div>

            {/* 错误统计 */}
            {report.errorStatistics.length > 0 && (
              <div className="flex flex-col gap-3">
                <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                  <AlertCircle className="w-4 h-4 text-red-500" />
                  错误统计
                </h3>
                <div className="rounded-xl border border-border/50 overflow-hidden">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="bg-muted/50">
                        <th className="text-left py-2 px-3 font-medium text-muted-foreground">
                          错误类型
                        </th>
                        <th className="text-right py-2 px-3 font-medium text-muted-foreground">
                          数量
                        </th>
                      </tr>
                    </thead>
                    <tbody>
                      {report.errorStatistics.slice(0, 5).map((error, index) => (
                        <tr
                          key={`${error.errorKind}-${index}`}
                          className="border-t border-border/50"
                        >
                          <td className="py-2 px-3 text-red-600">
                            {error.errorKind}
                          </td>
                          <td className="py-2 px-3 text-right tabular-nums font-medium">
                            {error.count}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
                {report.errorStatistics.length > 5 && (
                  <p className="text-xs text-muted-foreground text-center">
                    显示前 5 种错误类型，共 {report.errorStatistics.length} 种
                  </p>
                )}
              </div>
            )}

            {/* 配置信息 */}
            <div className="flex items-center gap-4 text-xs text-muted-foreground">
              <span>并发用户: {report.config.concurrentUsers}</span>
              <span>爬升时间: {report.config.rampUpSeconds}s</span>
              <span>计划时长: {report.config.durationSeconds}s</span>
            </div>
          </div>
        </ScrollArea>
      </DialogContent>
    </Dialog>
  );
}
