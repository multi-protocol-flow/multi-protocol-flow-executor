/**
 * StagesSection - 生命周期阶段执行记录
 *
 * 将 `FlowStagesReport`（Setup / Main / Teardown）渲染为可折叠面板：
 * 每段展示状态、耗时与节点执行流水（节点名、类型、耗时、错误摘要）。
 * 流程执行报告详情页与压测报告详情页共用。
 */
import { useState } from 'react';
import { CheckCircle2, XCircle, Clock, ChevronDown, ChevronRight, CircleSlash } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import { cn } from '@/lib/utils';
import type { ExecutionStatus, FlowStagesReport, NodeExecutionReport, StageExecutionReport } from '@/types';

type Stage = 'setup' | 'main' | 'teardown';

/** 阶段状态图标与配色 */
const STATUS_STYLE: Record<ExecutionStatus, { icon: typeof CheckCircle2; className: string }> = {
  success: { icon: CheckCircle2, className: 'text-emerald-500' },
  failed: { icon: XCircle, className: 'text-destructive' },
  skipped: { icon: CircleSlash, className: 'text-muted-foreground' },
  cancelled: { icon: XCircle, className: 'text-amber-500' },
};

function stageTitle(stage: Stage): string {
  const ll = getLL().stages;
  if (stage === 'setup') return ll.report_setup();
  if (stage === 'teardown') return ll.report_teardown();
  return ll.report_main();
}

function stageShortLabel(stage: Stage): string {
  const ll = getLL().stages;
  if (stage === 'setup') return ll.short_setup();
  if (stage === 'teardown') return ll.short_teardown();
  return ll.short_main();
}

/** 节点执行流水行 */
function NodeRow({ report }: { report: NodeExecutionReport }) {
  const { icon: StatusIcon, className } = STATUS_STYLE[report.status];
  return (
    <div className="flex items-center gap-2 rounded-md bg-muted/30 px-2 py-1.5 text-xs">
      <StatusIcon className={cn('h-3.5 w-3.5 shrink-0', className)} />
      <span className="truncate font-medium text-foreground" title={report.node_name}>
        {report.node_name || report.node_uuid}
      </span>
      <span className="shrink-0 rounded bg-muted px-1 py-0.5 text-[10px] text-muted-foreground">
        {report.node_type}
      </span>
      <span className="ml-auto shrink-0 tabular-nums text-muted-foreground">
        {report.duration_ms}ms
      </span>
      {report.error && (
        <span className="max-w-[45%] truncate text-destructive" title={report.error.message}>
          {report.error.message}
        </span>
      )}
    </div>
  );
}

/** 单个阶段面板 */
function StagePanel({
  stage,
  report,
  defaultOpen,
  nodeRowLimit,
}: {
  stage: Stage;
  report?: StageExecutionReport;
  defaultOpen: boolean;
  nodeRowLimit?: number;
}) {
  const [open, setOpen] = useState(defaultOpen);
  const ll = getLL().stages;

  if (!report) {
    return (
      <div
        data-testid={`stage-panel-${stage}`}
        data-configured="false"
        className="rounded-lg border border-border/60 bg-muted/10 px-3 py-2 text-xs text-muted-foreground"
      >
        <span className="font-medium text-foreground">{stageTitle(stage)}</span>
        <span className="ml-2">{ll.not_configured()}</span>
      </div>
    );
  }

  const { icon: StatusIcon, className } = STATUS_STYLE[report.status];
  const nodes = nodeRowLimit ? report.node_reports.slice(0, nodeRowLimit) : report.node_reports;

  return (
    <div className="rounded-lg border border-border/60 overflow-hidden">
      <button
        type="button"
        data-testid={`stage-panel-${stage}`}
        onClick={() => setOpen((prev) => !prev)}
        className="flex w-full items-center gap-2 px-3 py-2 text-xs cursor-pointer hover:bg-muted/30"
      >
        {open ? (
          <ChevronDown className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        )}
        <span className="font-semibold text-foreground">{stageTitle(stage)}</span>
        <StatusIcon className={cn('h-3.5 w-3.5 shrink-0', className)} />
        {report.status === 'skipped' && (
          <span className="rounded bg-muted px-1 py-0.5 text-[10px] text-muted-foreground">
            {ll.skipped()}
          </span>
        )}
        <span className="ml-auto flex items-center gap-1 tabular-nums text-muted-foreground">
          <Clock className="h-3 w-3" />
          {report.duration_ms}ms
        </span>
        <span className="shrink-0 text-muted-foreground">
          {ll.node_count({ count: report.node_reports.length })}
        </span>
      </button>

      {open && (
        <div className="flex flex-col gap-1 border-t border-border/40 px-2 py-2">
          {report.error && (
            <div className="rounded-md bg-destructive/10 px-2 py-1 text-xs text-destructive">
              {report.error.message}
            </div>
          )}
          {nodes.length === 0 ? (
            <p className="px-2 py-1 text-xs text-muted-foreground">{ll.no_nodes()}</p>
          ) : (
            nodes.map((nodeReport) => <NodeRow key={nodeReport.node_uuid} report={nodeReport} />)
          )}
        </div>
      )}
    </div>
  );
}

export interface StagesSectionProps {
  /** 各阶段执行报告 */
  stages: FlowStagesReport;
  /** 面板标题（默认使用生命周期执行记录标题） */
  title?: string;
  /** 默认展开的阶段（默认展开全部有数据的阶段） */
  defaultOpenStages?: Stage[];
  /** 每个阶段最多展示的节点流水行数（压测报告仅需概览） */
  nodeRowLimit?: number;
}

/**
 * 生命周期阶段执行记录区块
 */
export function StagesSection({
  stages,
  title,
  defaultOpenStages,
  nodeRowLimit,
}: StagesSectionProps) {
  const ll = getLL().stages;
  const openStages = defaultOpenStages ?? ['setup', 'main', 'teardown'];
  const allStages: Stage[] = ['setup', 'main', 'teardown'];

  return (
    <div className="flex flex-col gap-2" data-testid="stages-section">
      <h3 className="flex items-center gap-2 text-sm font-semibold text-foreground">
        <Clock className="h-4 w-4" />
        {title ?? ll.report_lifecycle()}
      </h3>
      {allStages.map((stage) => (
        <StagePanel
          key={stage}
          stage={stage}
          report={
            stage === 'setup' ? stages.setup : stage === 'teardown' ? stages.teardown : stages.main
          }
          defaultOpen={openStages.includes(stage)}
          nodeRowLimit={nodeRowLimit}
        />
      ))}
    </div>
  );
}

/** 阶段短标签（供报告列表等场景复用） */
export { stageShortLabel };
