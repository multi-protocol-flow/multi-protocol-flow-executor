/**
 * 数据集报告列表页
 * 显示所有数据集执行报告摘要，支持查看详情和删除
 */
import { useState, useEffect, useCallback } from 'react';
import {
  CheckCircle2,
  XCircle,
  AlertTriangle,
  Clock,
  Trash2,
  Loader2,
  FileSpreadsheet,
  ArrowRight,
} from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
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
import { useUIStore } from '@/stores';
import {
  listDatasetReports,
  deleteDatasetReport,
} from '@/services/tauri';
import type { DatasetReportSummary } from '@/types/dataset';
import { cn } from '@/lib/utils';

/** 格式化时间戳 */
function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp);
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

interface DatasetReportListProps {
  /** 查看报告详情 */
  onViewReport: (reportId: string) => void;
}

export function DatasetReportList({ onViewReport }: DatasetReportListProps) {
  const ll = getLL();
  const [reports, setReports] = useState<DatasetReportSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const reportDir = useSettingsStore((state) => state.reportDir);
  const showConfirm = useUIStore((state) => state.showConfirm);

  /** 加载报告列表 */
  const loadReports = useCallback(async () => {
    if (!reportDir) return;
    setLoading(true);
    try {
      const list = await listDatasetReports(reportDir);
      // 按执行时间降序排列（最新在前）
      setReports(list.sort((a, b) => b.executed_at - a.executed_at));
    } catch (error) {
      console.error('加载数据集报告列表失败:', error);
      toast.error(ll.execution.toast.load_report_list_failed(), {
        description: error instanceof Error ? error.message : ll.execution.datasetReport.unknown_error(),
      });
    } finally {
      setLoading(false);
    }
  }, [reportDir]);

  useEffect(() => {
    loadReports();
  }, [loadReports]);

  /** 删除报告 */
  const handleDelete = useCallback(
    (report: DatasetReportSummary) => {
      showConfirm({
        title: ll.execution.datasetReport.delete_title(),
        description: ll.execution.datasetReport.delete_confirm(),
        confirmText: ll.common.DELETE(),
        onConfirm: async () => {
          try {
            await deleteDatasetReport(report.report_id, reportDir);
            setReports((prev) => prev.filter((r) => r.report_id !== report.report_id));
            toast.success(ll.execution.toast.report_deleted());
          } catch (error) {
            console.error('删除报告失败:', error);
            toast.error(ll.execution.toast.delete_report_failed(), {
              description: error instanceof Error ? error.message : ll.execution.datasetReport.unknown_error(),
            });
          }
        },
      });
    },
    [reportDir, showConfirm],
  );

  // 加载中
  if (loading) {
    return (
      <div className="flex flex-col items-center justify-center h-full py-16 text-muted-foreground">
        <Loader2 className="w-10 h-10 mb-3 animate-spin" />
        <p className="text-sm">{ll.execution.datasetReport.loading()}</p>
      </div>
    );
  }

  // 空状态
  if (reports.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full py-16 text-muted-foreground">
        <FileSpreadsheet className="w-12 h-12 mb-3 opacity-40" />
        <p className="text-sm font-medium">{ll.execution.datasetReport.no_reports()}</p>
        <p className="text-xs mt-1 opacity-70">{ll.execution.datasetReport.empty_desc()}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* 标题栏 */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-border shrink-0">
        <div>
          <h2 className="text-lg font-semibold text-foreground">{ll.execution.datasetReport.title()}</h2>
          <p className="text-xs text-muted-foreground mt-0.5">
            {ll.execution.datasetReport.report_count({ count: reports.length })}
          </p>
        </div>
<Button variant="outline" size="sm" onClick={loadReports}>
           {ll.execution.datasetReport.refresh()}
         </Button>
      </div>

      {/* 表格 */}
      <ScrollArea className="flex-1">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[180px]">{ll.execution.datasetReport.label_flow_name()}</TableHead>
              <TableHead className="w-[150px]">{ll.execution.datasetReport.label_dataset_name()}</TableHead>
              <TableHead className="w-[160px]">{ll.execution.datasetReport.label_time()}</TableHead>
              <TableHead className="w-[80px] text-center">{ll.execution.datasetReport.label_total_rows()}</TableHead>
              <TableHead className="w-[80px] text-center">
                <span className="inline-flex items-center gap-1">
                  <CheckCircle2 className="w-3.5 h-3.5 text-green-600" />
                  {ll.execution.datasetReport.success()}
                </span>
              </TableHead>
              <TableHead className="w-[80px] text-center">
                <span className="inline-flex items-center gap-1">
                  <XCircle className="w-3.5 h-3.5 text-red-600" />
                  {ll.execution.datasetReport.fail()}
                </span>
              </TableHead>
              <TableHead className="w-[80px] text-center">
                <span className="inline-flex items-center gap-1">
                  <AlertTriangle className="w-3.5 h-3.5 text-yellow-600" />
                  {ll.execution.datasetReport.error()}
                </span>
              </TableHead>
              <TableHead className="w-[100px] text-right">{ll.execution.datasetReport.actions()}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {reports.map((report) => (
                <TableRow
                  key={report.report_id}
                  className={cn(
                    'cursor-pointer transition-colors',
                    'hover:bg-accent',
                  )}
                  onClick={() => onViewReport(report.report_id)}
                >
                  <TableCell className="font-medium">{report.flow_name}</TableCell>
                  <TableCell className="text-muted-foreground">
                    {report.dataset_name}
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1.5 text-muted-foreground">
                      <Clock className="w-3.5 h-3.5" />
                      <span className="text-xs">{formatTimestamp(report.executed_at)}</span>
                    </div>
                  </TableCell>
                  <TableCell className="text-center tabular-nums">
                    {report.total_rows}
                  </TableCell>
                  <TableCell className="text-center tabular-nums">
                    <span className={cn(report.success_count > 0 && 'text-green-600 font-medium')}>
                      {report.success_count}
                    </span>
                  </TableCell>
                  <TableCell className="text-center tabular-nums">
                    <span className={cn(report.fail_count > 0 && 'text-red-600 font-medium')}>
                      {report.fail_count}
                    </span>
                  </TableCell>
                  <TableCell className="text-center tabular-nums">
                    <span className={cn(report.error_count > 0 && 'text-yellow-600 font-medium')}>
                      {report.error_count}
                    </span>
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-1">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDelete(report);
                        }}
                        className="h-7 text-xs text-muted-foreground hover:text-red-500"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-7 text-xs"
                        onClick={(e) => {
                          e.stopPropagation();
                          onViewReport(report.report_id);
                        }}
                      >
{ll.execution.datasetReport.detail()}
                         <ArrowRight className="w-3.5 h-3.5 ml-1" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollArea>
    </div>
  );
}
