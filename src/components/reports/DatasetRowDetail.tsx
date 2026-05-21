/**
 * 数据集行详情页
 * 顶部上下文栏 + 复用 ExecutionReport 组件展示流程执行报告
 */
import { useState, useEffect } from 'react';
import {
  ArrowLeft,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  Loader2,
} from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useSettingsStore } from '@/stores/settings';
import { useExecutionStore } from '@/stores/execution';
import { loadDatasetReport, loadDatasetRowDetail } from '@/services/tauri';
import type { DatasetExecutionReport, RowSummary, RowStatus } from '@/types/dataset';
import type { FlowExecutionReport } from '@/types/report';
import { cn } from '@/lib/utils';

/** 行状态配置 */
function getStatusConfig(status: RowStatus, ll: ReturnType<typeof getLL>) {
  switch (status) {
    case 'success':
      return {
        bg: 'bg-green-100 dark:bg-green-900/50',
        text: 'text-green-700 dark:text-green-300',
        label: ll.execution.datasetReport.success(),
        icon: CheckCircle2,
      };
    case 'fail':
      return {
        bg: 'bg-red-100 dark:bg-red-900/50',
        text: 'text-red-700 dark:text-red-300',
        label: ll.execution.datasetReport.fail(),
        icon: XCircle,
      };
    case 'error':
      return {
        bg: 'bg-yellow-100 dark:bg-yellow-900/50',
        text: 'text-yellow-700 dark:text-yellow-300',
        label: ll.execution.datasetReport.error(),
        icon: AlertTriangle,
      };
  }
}

/** 格式化耗时 */
function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}

interface DatasetRowDetailProps {
  /** 报告 ID */
  reportId: string;
  /** 行索引 */
  rowIndex: number;
  /** 返回报告详情回调 */
  onBack: () => void;
}

export function DatasetRowDetail({
  reportId,
  rowIndex,
  onBack,
}: DatasetRowDetailProps) {
  const ll = getLL();
  const [rowSummary, setRowSummary] = useState<RowSummary | null>(null);
  const [loading, setLoading] = useState(true);

  const reportDir = useSettingsStore((state) => state.reportDir);
  const openReportDialog = useExecutionStore((state) => state.openReportDialog);

  // 加载行摘要 + 行详情，然后复用 ExecutionReport 弹窗
  useEffect(() => {
    if (!reportDir || !reportId) return;

    let cancelled = false;

    const load = async () => {
      setLoading(true);
      try {
        // 先加载报告获取行摘要
        const report: DatasetExecutionReport = await loadDatasetReport(
          reportId,
          reportDir,
        );
        if (cancelled) return;

        const summary = report.row_summaries.find(
          (r) => r.row_index === rowIndex,
        );
        if (summary) {
          setRowSummary(summary);
        }

        // 加载行详情（FlowExecutionReport）
        const detail = await loadDatasetRowDetail(reportId, rowIndex, reportDir);
        if (cancelled) return;

        if (detail) {
          // 将行详情转换为 FlowExecutionReport 并通过 execution store 打开报告弹窗
          const flowReport = detail as FlowExecutionReport;
          openReportDialog(flowReport);
        }
      } catch (error) {
        if (!cancelled) {
          console.error('加载行详情失败:', error);
          toast.error(ll.execution.toast.load_row_detail_failed(), {
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
  }, [reportId, rowIndex, reportDir, openReportDialog]);

  if (loading) {
    return (
      <div className="flex flex-col items-center justify-center h-full py-16 text-muted-foreground">
        <Loader2 className="w-10 h-10 mb-3 animate-spin" />
        <p className="text-sm">{ll.execution.datasetReport.loading_row()}</p>
      </div>
    );
  }

  if (!rowSummary) {
    return (
      <div className="flex flex-col items-center justify-center h-full py-16 text-muted-foreground">
        <p className="text-sm">{ll.execution.datasetReport.row_not_exist()}</p>
        <Button variant="outline" size="sm" className="mt-4" onClick={onBack}>
          {ll.execution.datasetReport.back()}
        </Button>
      </div>
    );
  }

  const statusConfig = getStatusConfig(rowSummary.status, ll);
  const StatusIcon = statusConfig.icon;

  // 构建输入数据摘要（显示前几个字段）
  const inputEntries = Object.entries(rowSummary.input_data);
  const displayEntries = inputEntries.slice(0, 5);

  return (
    <div className="flex flex-col h-full">
      {/* 顶部上下文栏 */}
      <div className="flex items-center gap-3 px-6 py-3 border-b border-border shrink-0">
        <Button variant="ghost" size="sm" onClick={onBack}>
          <ArrowLeft className="w-4 h-4 mr-1" />
          {ll.execution.datasetReport.back()}
        </Button>
        <div className="h-4 w-px bg-border" />
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-sm font-medium text-foreground">
            {ll.execution.datasetReport.row_n({ n: rowSummary.row_index + 1 })}
          </span>
          <Badge
            variant="outline"
            className={cn(
              'text-xs font-medium',
              statusConfig.bg,
              statusConfig.text,
            )}
          >
            <StatusIcon className="w-3 h-3 mr-1" />
            {statusConfig.label}
          </Badge>
          <span className="text-xs text-muted-foreground tabular-nums">
            {formatDuration(rowSummary.duration_ms)}
          </span>
        </div>

        {/* 输入数据摘要 */}
        <div className="h-4 w-px bg-border" />
        <div className="flex items-center gap-2 text-xs text-muted-foreground overflow-hidden">
          {displayEntries.map(([key, value]) => (
            <span key={key} className="whitespace-nowrap">
              <span className="text-foreground/70">{key}:</span>{' '}
              {String(value)}
            </span>
          ))}
          {inputEntries.length > 5 && (
            <span className="text-muted-foreground/60">
              {ll.execution.datasetReport.n_more({ n: inputEntries.length - 5 })}
            </span>
          )}
        </div>
      </div>

      {/* 行详情错误信息 */}
      {rowSummary.error_message && (
        <div className="px-6 py-3 bg-red-50 dark:bg-red-950/30 border-b border-red-200/50 dark:border-red-800/50">
          <div className="flex items-start gap-2">
            <XCircle className="w-4 h-4 text-red-500 mt-0.5 shrink-0" />
            <div>
              <p className="text-sm font-medium text-red-600 dark:text-red-400">
                {ll.execution.datasetReport.error_message()}
              </p>
              <p className="text-xs text-red-600/80 dark:text-red-400/80 mt-0.5">
                {rowSummary.error_message}
              </p>
            </div>
          </div>
        </div>
      )}

      {/* 提示：详情在弹窗中展示 */}
      <div className="flex-1 flex flex-col items-center justify-center text-muted-foreground gap-3 py-8">
        <p className="text-sm">
          {ll.execution.datasetReport.detail_in_popup()}
        </p>
        <Button variant="outline" size="sm" onClick={onBack}>
          {ll.execution.datasetReport.back_to_report()}
        </Button>
      </div>
    </div>
  );
}
