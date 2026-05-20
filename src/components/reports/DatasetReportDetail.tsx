/**
 * 数据集报告详情页
 * 显示汇总统计 + 可过滤的行数据表格
 */
import { useState, useEffect, useMemo } from 'react';
import {
  CheckCircle2,
  XCircle,
  AlertTriangle,
  Clock,
  BarChart3,
  Timer,
  Loader2,
  ExternalLink,
} from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { useSettingsStore } from '@/stores/settings';
import { loadDatasetReport } from '@/services/tauri';
import type { DatasetExecutionReport, RowStatus } from '@/types/dataset';
import { cn } from '@/lib/utils';

/** 格式化耗时 */
function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}

/** 状态过滤标签 */
type StatusFilter = 'all' | 'success' | 'fail' | 'error';

/** 统计卡片 */
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
      <p className={cn('text-xl font-bold tabular-nums', valueColor)}>{value}</p>
    </div>
  );
}

/** 行状态标签颜色映射 */
function getStatusStyle(status: RowStatus, ll: ReturnType<typeof getLL>) {
  switch (status) {
    case 'success':
      return {
        bg: 'bg-green-100 dark:bg-green-900/50',
        text: 'text-green-700 dark:text-green-300',
        border: 'border-green-200 dark:border-green-700',
        label: ll.execution.datasetReport.success(),
        icon: CheckCircle2,
      };
    case 'fail':
      return {
        bg: 'bg-red-100 dark:bg-red-900/50',
        text: 'text-red-700 dark:text-red-300',
        border: 'border-red-200 dark:border-red-700',
        label: ll.execution.datasetReport.fail(),
        icon: XCircle,
      };
    case 'error':
      return {
        bg: 'bg-yellow-100 dark:bg-yellow-900/50',
        text: 'text-yellow-700 dark:text-yellow-300',
        border: 'border-yellow-200 dark:border-yellow-700',
        label: ll.execution.datasetReport.error(),
        icon: AlertTriangle,
      };
  }
}

interface DatasetReportDetailProps {
  /** 报告 ID */
  reportId: string;
  /** 查看行详情回调 */
  onViewRowDetail: (reportId: string, rowIndex: number) => void;
}

export function DatasetReportDetail({
  reportId,
  onViewRowDetail,
}: DatasetReportDetailProps) {
  const ll = getLL();
  const [report, setReport] = useState<DatasetExecutionReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');

  const reportDir = useSettingsStore((state) => state.reportDir);

  // 加载报告详情
  useEffect(() => {
    if (!reportDir || !reportId) return;

    let cancelled = false;
    const load = async () => {
      setLoading(true);
      try {
        const data = await loadDatasetReport(reportId, reportDir);
        if (!cancelled) {
          setReport(data);
        }
      } catch (error) {
        if (!cancelled) {
          console.error('加载报告详情失败:', error);
          toast.error(ll.execution.toast.load_report_detail_failed(), {
            description: error instanceof Error ? error.message : ll.execution.datasetReport.unknown_error(),
          });
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };
    load();
    return () => {
      cancelled = true;
    };
  }, [reportId, reportDir]);

  /** 从行摘要中提取 CSV 列头（取第一行的 input_data 键） */
  const csvColumns = useMemo(() => {
    if (!report || report.row_summaries.length === 0) return [];
    const firstRow = report.row_summaries[0];
    return Object.keys(firstRow.input_data);
  }, [report]);

  /** 过滤后的行数据 */
  const filteredRows = useMemo(() => {
    if (!report) return [];
    if (statusFilter === 'all') return report.row_summaries;
    return report.row_summaries.filter((row) => row.status === statusFilter);
  }, [report, statusFilter]);

  /** 过滤标签计数 */
  const filterCounts = useMemo(() => {
    if (!report) return { all: 0, success: 0, fail: 0, error: 0 };
    return {
      all: report.row_summaries.length,
      success: report.success_count,
      fail: report.fail_count,
      error: report.error_count,
    };
  }, [report]);

  if (loading) {
    return (
      <div className="flex flex-col items-center justify-center h-full py-16 text-muted-foreground">
        <Loader2 className="w-10 h-10 mb-3 animate-spin" />
        <p className="text-sm">{ll.execution.datasetReport.loading_detail()}</p>
      </div>
    );
  }

  if (!report) {
    return (
      <div className="flex flex-col items-center justify-center h-full py-16 text-muted-foreground">
        <p className="text-sm">{ll.execution.datasetReport.not_found()}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* 顶部信息栏 */}
      <div className="flex items-center gap-3 px-6 py-3 border-b border-border shrink-0">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold text-foreground truncate">
            {report.flow_name}
          </h2>
          <p className="text-xs text-muted-foreground">{report.dataset_name}</p>
        </div>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-6 space-y-5">
          {/* 统计卡片网格 */}
          <div className="grid grid-cols-2 lg:grid-cols-4 xl:grid-cols-6 gap-3">
            <StatCard
              icon={CheckCircle2}
              label={ll.execution.datasetReport.success()}
              value={report.success_count}
              iconColor="text-green-500"
              valueColor="text-green-600"
            />
            <StatCard
              icon={XCircle}
              label={ll.execution.datasetReport.fail()}
              value={report.fail_count}
              iconColor="text-red-500"
              valueColor="text-red-600"
            />
            <StatCard
              icon={AlertTriangle}
              label={ll.execution.datasetReport.error()}
              value={report.error_count}
              iconColor="text-yellow-500"
              valueColor="text-yellow-600"
            />
            <StatCard
              icon={Timer}
              label={ll.execution.datasetReport.avg_duration()}
              value={formatDuration(report.avg_duration_ms)}
              iconColor="text-violet-500"
              valueColor="text-violet-600"
            />
            <StatCard
              icon={BarChart3}
              label={ll.execution.datasetReport.p95()}
              value={formatDuration(report.p95_duration_ms)}
              iconColor="text-blue-500"
              valueColor="text-blue-600"
            />
            <StatCard
              icon={Clock}
              label={ll.execution.datasetReport.total_duration()}
              value={formatDuration(report.total_duration_ms)}
              iconColor="text-orange-500"
              valueColor="text-orange-600"
            />
          </div>

          {/* 状态过滤标签 */}
          <div className="flex items-center gap-2">
            {(
              [
                { key: 'all', label: ll.execution.datasetReport.filter_all() },
                { key: 'success', label: ll.execution.datasetReport.filter_pass() },
                { key: 'fail', label: ll.execution.datasetReport.filter_fail() },
                { key: 'error', label: ll.execution.datasetReport.filter_error() },
              ] as const
            ).map(({ key, label }) => (
              <Button
                key={key}
                variant={statusFilter === key ? 'default' : 'outline'}
                size="sm"
                className="h-8 text-xs"
                onClick={() => setStatusFilter(key)}
              >
                {label}
                <Badge variant="secondary" className="ml-1.5 px-1.5 py-0 text-[10px]">
                  {filterCounts[key]}
                </Badge>
              </Button>
            ))}
          </div>

          {/* 行数据表格 */}
          <div className="rounded-xl border border-border overflow-hidden">
            <Table>
              <TableHeader>
                <TableRow className="bg-muted/30">
                  <TableHead className="w-[60px] text-center">#</TableHead>
                  {csvColumns.map((col) => (
                    <TableHead key={col} className="max-w-[200px]">
                      {col}
                    </TableHead>
                  ))}
                  <TableHead className="w-[80px] text-center">{ll.execution.datasetReport.label_status()}</TableHead>
                  <TableHead className="w-[100px] text-right">{ll.execution.datasetReport.label_duration()}</TableHead>
                  <TableHead className="w-[200px]">{ll.execution.datasetReport.label_error()}</TableHead>
                  <TableHead className="w-[60px] text-center">{ll.execution.datasetReport.detail()}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredRows.length === 0 ? (
                  <TableRow>
                    <TableCell
                      colSpan={csvColumns.length + 5}
                      className="text-center py-8 text-muted-foreground"
                    >
                      {ll.execution.datasetReport.no_matching_rows()}
                    </TableCell>
                  </TableRow>
                ) : (
                  filteredRows.map((row) => {
                    const style = getStatusStyle(row.status, ll);
                    const StatusIcon = style.icon;
                    const canDrillDown =
                      row.status !== 'success' && row.has_detail_report;
                    return (
                      <TableRow
                        key={row.row_index}
                        className={cn(
                          canDrillDown && 'cursor-pointer',
                          row.status === 'fail' && 'bg-red-50/50 dark:bg-red-950/20',
                          row.status === 'error' && 'bg-yellow-50/50 dark:bg-yellow-950/20',
                        )}
                        onClick={() => {
                          if (canDrillDown || row.has_detail_report) {
                            onViewRowDetail(reportId, row.row_index);
                          }
                        }}
                      >
                        {/* 行号 */}
                        <TableCell className="text-center tabular-nums text-muted-foreground">
                          {row.row_index + 1}
                        </TableCell>

                        {/* CSV 数据列 */}
                        {csvColumns.map((col) => (
                          <TableCell
                            key={col}
                            className="max-w-[200px] truncate text-xs"
                            title={String(row.input_data[col] ?? '')}
                          >
                            {String(row.input_data[col] ?? '')}
                          </TableCell>
                        ))}

                        {/* 状态 */}
                        <TableCell className="text-center">
                          <Badge
                            variant="outline"
                            className={cn(
                              'text-xs font-medium',
                              style.bg,
                              style.text,
                              style.border,
                            )}
                          >
                            <StatusIcon className="w-3 h-3 mr-1" />
                            {style.label}
                          </Badge>
                        </TableCell>

                        {/* 耗时 */}
                        <TableCell className="text-right tabular-nums text-xs">
                          {formatDuration(row.duration_ms)}
                        </TableCell>

                        {/* 错误信息 */}
                        <TableCell className="text-xs text-muted-foreground max-w-[200px] truncate">
                          {row.error_message ?? '-'}
                        </TableCell>

                        {/* 详情链接 */}
                        <TableCell className="text-center">
                          {row.has_detail_report ? (
                            <ExternalLink className="w-3.5 h-3.5 text-muted-foreground mx-auto" />
                          ) : (
                            <span className="text-muted-foreground/40">-</span>
                          )}
                        </TableCell>
                      </TableRow>
                    );
                  })
                )}
              </TableBody>
            </Table>
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
