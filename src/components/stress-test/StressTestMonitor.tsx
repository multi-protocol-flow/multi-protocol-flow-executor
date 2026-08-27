/**
 * StressTestMonitor - 压力测试实时监控面板
 * 显示压测进度、实时指标和停止按钮
 */
import { useState, useMemo, useCallback } from 'react';
import { Activity, Gauge, Timer, Square, X, FileBarChart } from 'lucide-react';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  useStressTestStore,
  selectIsStressTestRunning,
  selectStressTestProgress,
  selectStressTestStatus,
  selectStressTestProgressHistory,
  selectStressTestReport,
} from '@/stores/stressTest';
import type { NodeMetrics } from '@/types/stressTest';
import { StressTestCharts } from './StressTestCharts';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';

/**
 * 指标卡片组件
 */
function MetricCard({
  icon: Icon,
  label,
  value,
  subValue,
  iconColor,
  className,
}: {
  icon: React.ElementType;
  label: string;
  value: string | number;
  subValue?: string;
  iconColor?: string;
  className?: string;
}) {
  return (
    <Card className={cn('border-border/50 bg-gradient-to-br from-background to-muted/30', className)}>
      <CardContent className="p-4">
        <div className="flex items-center gap-2 mb-2">
          <Icon className={cn('w-4 h-4', iconColor || 'text-muted-foreground')} />
          <span className="text-xs text-muted-foreground">{label}</span>
        </div>
        <p className="text-xl font-bold tabular-nums">{value}</p>
        {subValue && (
          <p className="text-xs text-muted-foreground mt-1">{subValue}</p>
        )}
      </CardContent>
    </Card>
  );
}

/**
 * 压力测试监控面板
 * 运行时显示实时指标，结束后保持显示最终数据供用户查看
 */
export function StressTestMonitor() {
  const isRunning = useStressTestStore(selectIsStressTestRunning);
  const progress = useStressTestStore(selectStressTestProgress);
  const status = useStressTestStore(selectStressTestStatus);
  const progressHistory = useStressTestStore(selectStressTestProgressHistory);
  const report = useStressTestStore(selectStressTestReport);
  const stopStressTest = useStressTestStore((state) => state.stopStressTest);
  const openReportDialog = useStressTestStore((state) => state.openReportDialog);
  const reset = useStressTestStore((state) => state.reset);

  // 已完成（有数据可供查看）
  const isCompleted = status === 'stopped' && (progressHistory.length > 0 || report !== null);

  // 完成后的汇总数据（progress 在 ended 后被置为 null，需要从 report 读取）
  const displayTotalIterations = isCompleted && report
    ? report.statistics.totalIterations
    : (progress?.totalIterations ?? 0);
  const displayRps = isCompleted && report
    ? report.statistics.rps.toFixed(2)
    : (progress?.currentRps?.toFixed(2) ?? '0.00');
  const displayAvgLatency = isCompleted && report
    ? report.responseTimeDistribution.avgMs.toFixed(0)
    : (progress?.avgLatencyMs?.toFixed(0) ?? 0);
  const displaySuccessDetail = isCompleted && report
    ? `${getLL().flow.stress_monitor_exec_detail({ success: String(report.statistics.successful), fail: String(report.statistics.failed) })}`
    : (progress ? `${getLL().flow.stress_monitor_exec_detail({ success: String(progress.successfulIterations), fail: String(progress.failedIterations) })}` : undefined);

  // 节点类型筛选状态
  const [selectedTypes, setSelectedTypes] = useState<string[]>([]);

  // 从最新的 progressHistory 获取 nodeMetrics，或从当前 progress
  const nodeMetrics: NodeMetrics[] = useMemo(() => {
    if (progressHistory.length > 0) {
      return progressHistory[progressHistory.length - 1].nodeMetrics ?? [];
    }
    return progress?.nodeMetrics ?? [];
  }, [progressHistory, progress]);

  // 所有节点类型
  const allNodeTypes = useMemo(() => {
    return [...new Set(nodeMetrics.map((n) => n.nodeType))];
  }, [nodeMetrics]);

  // 切换节点类型筛选
  const toggleType = useCallback((type: string) => {
    setSelectedTypes((prev) =>
      prev.includes(type) ? prev.filter((t) => t !== type) : [...prev, type]
    );
  }, []);

  // 筛选后的节点指标
  const filteredMetrics = useMemo(() => {
    if (selectedTypes.length === 0) return nodeMetrics;
    return nodeMetrics.filter((n) => selectedTypes.includes(n.nodeType));
  }, [nodeMetrics, selectedTypes]);

  // 仅在运行时或刚完成时渲染，用户手动关闭后消失
  if (!isRunning && !isCompleted) {
    return null;
  }

  // 关闭面板
  const handleClose = () => {
    reset();
  };

  // 获取状态显示文本
  const getStatusText = () => {
    switch (status) {
      case 'rampUp':
        return getLL().flow.stress_monitor_status_rampup();
      case 'running':
        return getLL().flow.stress_monitor_status_running();
      case 'stopped':
        return getLL().flow.stress_monitor_status_completed();
      default:
        return '';
    }
  };

  // 获取状态 Badge 样式
  const getStatusBadgeVariant = () => {
    switch (status) {
      case 'rampUp':
        return 'bg-amber-50 text-amber-700 border-amber-200';
      case 'running':
        return 'bg-emerald-50 text-emerald-700 border-emerald-200';
      case 'stopped':
        return 'bg-blue-50 text-blue-700 border-blue-200';
      default:
        return '';
    }
  };

  // 处理停止压测
  const handleStopStressTest = async () => {
    try {
      await stopStressTest();
    } catch (error) {
      console.error('停止压测失败:', error);
    }
  };

  return (
    <Card className="fixed top-16 right-16 w-[420px] max-h-[calc(100vh-140px)] flex flex-col overflow-hidden z-50 shadow-xl">
      <CardContent className="p-4 flex-1 min-h-0 flex flex-col">
        {/* 头部：状态和时间 */}
        <div className="flex items-center justify-between mb-4 shrink-0">
          <div className="flex items-center gap-3">
            <Activity className="w-5 h-5 text-blue-500" />
            <span className="font-semibold">{getLL().flow.stress_monitor_title()}</span>
            <Badge
              variant="outline"
              className={cn('font-medium', getStatusBadgeVariant())}
            >
              {getStatusText()}
            </Badge>
          </div>
          <div className="flex items-center gap-1.5 text-sm text-muted-foreground">
            <Timer className="w-4 h-4" />
            <span className="font-medium tabular-nums">
              {getLL().flow.stress_monitor_elapsed({ seconds: String(progress?.elapsedSeconds ?? 0) })}
            </span>
          </div>
        </div>

        {/* 可滚动内容区域 */}
        <div className="flex-1 overflow-y-auto min-h-0">
        {/* 指标卡片网格 */}
        <div className="grid grid-cols-3 gap-3 mb-4">
          <MetricCard
            icon={Activity}
            label={getLL().flow.stress_monitor_exec_count()}
            value={displayTotalIterations}
            subValue={displaySuccessDetail}
            iconColor="text-blue-500"
          />
          <MetricCard
            icon={Gauge}
            label={getLL().flow.stress_monitor_current_rps()}
            value={displayRps}
            iconColor="text-violet-500"
          />
          <MetricCard
            icon={Timer}
            label={getLL().flow.stress_monitor_avg_response()}
            value={displayAvgLatency}
            subValue="ms"
            iconColor="text-emerald-500"
          />
        </div>

        {/* 实时图表 */}
        {(progressHistory.length > 0 || (isCompleted && report && report.statistics.totalIterations > 0)) && (
          <div className="mb-4">
            <StressTestCharts progressHistory={progressHistory} report={report} />
          </div>
        )}

        {/* 节点级实时指标 */}
        {nodeMetrics.length > 0 && (
          <div className="mb-4">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm font-semibold text-foreground">{getLL().flow.stress_monitor_node_metrics()}</span>
              <span className="text-xs text-muted-foreground">
                {selectedTypes.length > 0
                  ? `${filteredMetrics.length}/${nodeMetrics.length}`
                  : getLL().flow.stress_monitor_node_count({ count: nodeMetrics.length })}
              </span>
            </div>

            {/* 节点类型筛选 */}
            {allNodeTypes.length > 1 && (
              <div className="flex flex-wrap gap-1 mt-1 mb-2">
                {allNodeTypes.map((type) => (
                  <Badge
                    key={type}
                    variant={selectedTypes.includes(type) ? 'default' : 'outline'}
                    className="cursor-pointer text-xs"
                    onClick={() => toggleType(type)}
                  >
                    {type}
                  </Badge>
                ))}
              </div>
            )}

            {/* 节点指标表格 */}
            <ScrollArea className="max-h-[280px]">
              <div className="rounded-xl border border-border/50 overflow-x-auto">
                <table className="w-full min-w-max whitespace-nowrap text-xs">
                  <thead>
                    <tr className="bg-muted/50">
                      <th className="sticky left-0 z-20 bg-muted text-left py-1.5 px-2 font-medium text-muted-foreground">
                         {getLL().flow.stress_result_node_name()}
                      </th>
                      <th className="sticky left-[120px] z-10 bg-muted text-left py-1.5 px-2 font-medium text-muted-foreground border-r border-border/50">
                         {getLL().flow.stress_result_node_type()}
                      </th>
                      <th className="text-right py-1.5 px-2 font-medium text-muted-foreground">
                         {getLL().flow.stress_result_exec_count()}
                      </th>
                      <th className="text-right py-1.5 px-2 font-medium text-muted-foreground">
                         {getLL().flow.stress_result_success_rate()}
                      </th>
                      <th className="text-right py-1.5 px-2 font-medium text-muted-foreground">
                         {getLL().flow.stress_result_min()}
                      </th>
                      <th className="text-right py-1.5 px-2 font-medium text-muted-foreground">
                         {getLL().flow.stress_result_avg()}
                      </th>
                      <th className="text-right py-1.5 px-2 font-medium text-muted-foreground">
                         {getLL().flow.stress_result_max()}
                      </th>
                      <th className="text-right py-1.5 px-2 font-medium text-muted-foreground">
                        P50
                      </th>
                      <th className="text-right py-1.5 px-2 font-medium text-muted-foreground">
                        P90
                      </th>
                      <th className="text-right py-1.5 px-2 font-medium text-muted-foreground">
                        P95
                      </th>
                      <th className="text-right py-1.5 px-2 font-medium text-muted-foreground">
                        P99
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredMetrics.map((node) => (
                      <tr
                        key={node.nodeUuid}
                        className="border-t border-border/50"
                      >
                        <td className="sticky left-0 z-20 bg-background py-1.5 px-2 font-medium truncate w-[120px]">
                          {node.nodeName}
                        </td>
                        <td className="sticky left-[120px] z-10 bg-background py-1.5 px-2 text-muted-foreground border-r border-border/50">
                          {node.nodeType}
                        </td>
                        <td className="py-1.5 px-2 text-right tabular-nums">
                          {node.totalExecutions}
                        </td>
                        <td className={cn(
                          'py-1.5 px-2 text-right tabular-nums',
                          node.successRate >= 0.99 ? 'text-emerald-600' : 'text-red-600'
                        )}>
                          {(node.successRate * 100).toFixed(1)}%
                        </td>
                        <td className="py-1.5 px-2 text-right tabular-nums">
                          {node.minLatencyMs}ms
                        </td>
                        <td className="py-1.5 px-2 text-right tabular-nums">
                          {node.avgLatencyMs.toFixed(0)}ms
                        </td>
                        <td className="py-1.5 px-2 text-right tabular-nums">
                          {node.maxLatencyMs}ms
                        </td>
                        <td className="py-1.5 px-2 text-right tabular-nums">
                          {node.p50LatencyMs}ms
                        </td>
                        <td className="py-1.5 px-2 text-right tabular-nums">
                          {node.p90LatencyMs}ms
                        </td>
                        <td className="py-1.5 px-2 text-right tabular-nums">
                          {node.p95LatencyMs}ms
                        </td>
                        <td className="py-1.5 px-2 text-right tabular-nums">
                          {node.p99LatencyMs}ms
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </ScrollArea>
          </div>
        )}

        </div>{/* 可滚动内容区域结束 */}

        {/* 底部按钮 */}
        <div className="flex justify-end gap-2 shrink-0 pt-4 border-t border-border mt-4">
          {isRunning ? (
            <Button
              variant="destructive"
              size="sm"
              onClick={handleStopStressTest}
              className="gap-1.5"
            >
              <Square className="w-4 h-4" />
               {getLL().flow.stress_monitor_stop()}
            </Button>
          ) : (
            <>
              <Button
                variant="outline"
                size="sm"
                onClick={handleClose}
                className="gap-1.5"
              >
                <X className="w-4 h-4" />
                {getLL().flow.stress_monitor_close()}
              </Button>
              {report && (
                <Button
                  variant="default"
                  size="sm"
                  onClick={openReportDialog}
                  className="gap-1.5"
                >
                  <FileBarChart className="w-4 h-4" />
                  {getLL().flow.stress_monitor_view_report()}
                </Button>
              )}
            </>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
