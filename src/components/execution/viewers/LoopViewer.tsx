/**
 * Loop 节点执行结果查看器
 *
 * `flow:loop` 在顶层流程中算**一个节点**，但报告按迭代分组：
 * - `report.iteration_summary` 给出每一轮的序号/状态/耗时/等待（永不裁剪）
 * - `report.sub_flow_report.node_reports` 是保留下来的 body 节点报告，按
 *   `iteration_index` 归组，经 `NodeExecutionDetails` 复用各节点类型的 Viewer
 * - `report.truncated_iterations` 说明有多少轮只保留摘要
 */
import { memo, useMemo, useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  RotateCw,
  Clock,
  CheckCircle2,
  XCircle,
  Ban,
  ListTree,
} from 'lucide-react';
import type { NodeExecutionReport, IterationSummary, ExecutionStatus } from '@/types';
import { ErrorSection } from './ErrorSection';
import { NodeExecutionDetails } from '../NodeExecutionDetails';
import { useCurrentLocale } from '@/hooks/useLocale';
import { getLL } from '@/i18n/i18n-lazy';
import { cn } from '@/lib/utils';

interface LoopViewerProps {
  report: NodeExecutionReport;
}

/** 终止原因 → i18n 文案 */
function breakReasonLabel(reason: string | undefined): string {
  const ll = getLL().execution.report;
  switch (reason) {
    case 'exhausted':
      return ll.loop_reason_exhausted();
    case 'until':
      return ll.loop_reason_until();
    case 'budget':
      return ll.loop_reason_budget();
    case 'timeout':
      return ll.loop_reason_timeout();
    case 'depth':
      return ll.loop_reason_depth();
    case 'error':
      return ll.loop_reason_error();
    case 'cancelled':
      return ll.loop_reason_cancelled();
    default:
      return reason ?? '-';
  }
}

/** 状态图标（与 SubFlowViewer 保持一致的配色） */
function StatusIcon({ status }: { status: ExecutionStatus }) {
  if (status === 'success') return <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600" />;
  if (status === 'cancelled') return <Ban className="w-3.5 h-3.5 text-amber-600" />;
  return <XCircle className="w-3.5 h-3.5 text-rose-600" />;
}

interface IterationRowProps {
  summary: IterationSummary;
  reports: NodeExecutionReport[];
}

function IterationRow({ summary, reports }: IterationRowProps) {
  const ll = getLL().execution.report;
  // 失败轮默认展开，成功轮默认收起
  const [isOpen, setIsOpen] = useState(summary.status !== 'success');

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      <button
        type="button"
        onClick={() => setIsOpen((v) => !v)}
        className="w-full flex flex-wrap items-center gap-2 p-2.5 bg-muted/40 hover:bg-muted/60 transition-colors text-left"
      >
        {isOpen ? (
          <ChevronDown className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
        ) : (
          <ChevronRight className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
        )}
        <StatusIcon status={summary.status} />
        <span className="text-xs font-medium text-foreground">
          {ll.loop_iteration_label({ index: summary.index })}
        </span>
        <span className="text-[11px] text-muted-foreground">{summary.duration_ms} ms</span>
        {summary.wait_ms > 0 && (
          <span className="text-[11px] text-muted-foreground">
            {ll.loop_wait_ms()}: {summary.wait_ms} ms
          </span>
        )}
        <span className="ml-auto text-[11px] text-muted-foreground">
          {reports.length > 0 ? reports.length : '-'}
        </span>
      </button>
      {isOpen && (
        <div className="p-2.5 bg-background border-t border-border/60 space-y-2">
          {summary.error && (
            <ErrorSection
              error={summary.error}
              nodeName={report_name(summary)}
              nodeType="flow:loop"
            />
          )}
          {reports.length === 0 ? (
            <p className="text-xs text-muted-foreground">{ll.loop_truncated_hint()}</p>
          ) : (
            reports.map((r) => <NodeExecutionDetails key={`${r.node_uuid}-${r.start_time}`} report={r} />)
          )}
        </div>
      )}
    </div>
  );
}

/** 错误区块标题用的节点名 */
function report_name(summary: IterationSummary): string {
  return `#${summary.index}`;
}

/**
 * Loop 节点执行结果查看器
 */
export const LoopViewer = memo(function LoopViewer({ report }: LoopViewerProps) {
  useCurrentLocale();
  const ll = getLL().execution.report;

  const output = report.output_data as
    | { iterations?: number; break_reason?: string }
    | undefined;
  const summaries = report.iteration_summary ?? [];
  const bodyReports = report.sub_flow_report?.node_reports ?? [];

  const grouped = useMemo(() => {
    const map = new Map<number, NodeExecutionReport[]>();
    for (const r of bodyReports) {
      const index = r.iteration_index ?? -1;
      const list = map.get(index);
      if (list) list.push(r);
      else map.set(index, [r]);
    }
    return map;
  }, [bodyReports]);

  const isFailed = report.status === 'failed';
  const iterations = output?.iterations ?? summaries.length;
  const breakReason = output?.break_reason;

  return (
    <div className={cn('space-y-3 p-3', isFailed && 'rounded-lg border border-rose-300/60 p-3')}>
      {/* 头部摘要 */}
      <div className="flex flex-wrap items-center gap-3 text-xs">
        <span className="inline-flex items-center gap-1.5 font-medium text-sky-600">
          <RotateCw className="w-3.5 h-3.5" />
          {report.node_name}
        </span>
        <span className="text-muted-foreground">
          {ll.loop_iterations()}: <span className="text-foreground font-medium">{iterations}</span>
        </span>
        <span className="text-muted-foreground">
          {ll.loop_break_reason()}:{' '}
          <span className={cn('font-medium', isFailed ? 'text-rose-600' : 'text-emerald-600')}>
            {breakReasonLabel(breakReason)}
          </span>
        </span>
        <span className="inline-flex items-center gap-1 text-muted-foreground">
          <Clock className="w-3.5 h-3.5" />
          {report.duration_ms} ms
        </span>
        {(report.truncated_iterations ?? 0) > 0 && (
          <span className="text-[11px] text-amber-600">
            {ll.loop_truncated({ count: report.truncated_iterations ?? 0 })}
          </span>
        )}
      </div>

      {report.error && (
        <ErrorSection
          error={report.error}
          nodeName={report.node_name}
          nodeType={report.node_type}
        />
      )}

      {/* 迭代列表 */}
      <div className="space-y-2">
        <div className="flex items-center gap-1.5 text-xs font-medium text-foreground">
          <ListTree className="w-3.5 h-3.5 text-muted-foreground" />
          {ll.node_details()}
        </div>
        {summaries.length === 0 ? (
          <p className="text-xs text-muted-foreground">{ll.no_nodes()}</p>
        ) : (
          summaries.map((summary) => (
            <IterationRow
              key={summary.index}
              summary={summary}
              reports={grouped.get(summary.index) ?? []}
            />
          ))
        )}
      </div>
    </div>
  );
});
