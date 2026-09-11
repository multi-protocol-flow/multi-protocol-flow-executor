/**
 * SubFlow 节点执行结果查看器
 *
 * 子流程在顶层流程中算**一个节点**，但报告需要可下钻：这里以与普通节点报告一致的
 * 结构展示子流程的调用配置、输入/输出变量、执行统计与子节点执行明细。
 * 子节点通过 `NodeExecutionDetails` 复用各自类型的结构化 Viewers，
 * 因此任意层级的嵌套子流程都能继续下钻。
 */
import { useState, memo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Workflow,
  Clock,
  Layers,
  CheckCircle2,
  XCircle,
  Ban,
  Database,
  ExternalLink,
  ArrowRightLeft,
  AlertTriangle,
  ListTree,
  SlidersHorizontal,
  LogOut,
} from 'lucide-react';
import type { NodeExecutionReport, VariableMapping } from '@/types';
import { RawJsonViewer } from './RawJsonViewer';
import { VariableChangesCard } from './VariableChangesCard';
import { ErrorSection } from './ErrorSection';
import { NodeExecutionDetails } from '../NodeExecutionDetails';
import { useFlowStore } from '@/stores';
import { useCurrentLocale } from '@/hooks/useLocale';
import { flowStorage } from '@/services/storage';
import { getLL } from '@/i18n/i18n-lazy';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';

interface SubFlowViewerProps {
  report: NodeExecutionReport;
}

/** 折叠区块（与其它 Viewer 保持一致的本地实现） */
interface CollapsibleSectionProps {
  title: string;
  icon?: React.ReactNode;
  badge?: React.ReactNode;
  defaultOpen?: boolean;
  children: React.ReactNode;
}

function CollapsibleSection({
  title,
  icon,
  badge,
  defaultOpen = false,
  children,
}: CollapsibleSectionProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  return (
    <div className="border border-border rounded-lg overflow-hidden">
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className="w-full flex items-center gap-2 p-3 bg-muted/40 hover:bg-muted/60 transition-colors text-left"
      >
        {isOpen ? (
          <ChevronDown className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
        ) : (
          <ChevronRight className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
        )}
        {icon}
        <span className="font-medium text-xs text-foreground flex-1">{title}</span>
        {badge}
      </button>
      {isOpen && <div className="p-3 bg-background border-t border-border/60">{children}</div>}
    </div>
  );
}

/** 统计指标卡片 */
function StatCard({
  label,
  value,
  valueClassName,
}: {
  label: string;
  value: string | number;
  valueClassName?: string;
}) {
  return (
    <div className="rounded-lg border border-border/60 bg-muted/20 px-3 py-2">
      <div className="text-[11px] text-muted-foreground truncate">{label}</div>
      <div className={cn('text-sm font-semibold tabular-nums text-foreground', valueClassName)}>
        {value}
      </div>
    </div>
  );
}

/** 变量映射表格（输入覆盖 / 导出映射共用） */
function MappingTable({ mappings }: { mappings: VariableMapping[] }) {
  return (
    <div className="rounded border border-border/60 overflow-hidden">
      {mappings.map((mapping, idx) => (
        <div
          key={`${mapping.source}-${mapping.target}-${idx}`}
          className={cn(
            'flex items-center gap-2 px-2 py-1.5 text-xs',
            idx > 0 && 'border-t border-border/40'
          )}
        >
          <code className="font-mono text-[11px] text-foreground truncate flex-1 min-w-0">
            {mapping.source || '—'}
          </code>
          <ArrowRightLeft className="w-3 h-3 text-muted-foreground shrink-0" />
          <code className="font-mono text-[11px] text-violet-600 dark:text-violet-400 truncate flex-1 min-w-0">
            {mapping.target || '—'}
          </code>
        </div>
      ))}
    </div>
  );
}

/** 调用配置的运行时结构（`SubFlowPlugin::build_report_input` 输出） */
interface SubFlowInvocationInfo {
  target_flow_id?: string | null;
  target_flow_path?: string | null;
  inherit_all_variables?: boolean;
  export_all_variables?: boolean;
  variable_mappings?: VariableMapping[];
  output_mappings?: VariableMapping[];
  /** 兼容旧版报告：仅记录映射条目数量 */
  variable_mappings_count?: number;
  output_mappings_count?: number;
}

function asInvocationInfo(value: unknown): SubFlowInvocationInfo | null {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return null;
  return value as SubFlowInvocationInfo;
}

function asMappings(value: unknown): VariableMapping[] {
  if (!Array.isArray(value)) return [];
  return value.filter(
    (item): item is VariableMapping =>
      typeof item === 'object' && item !== null && 'source' in item && 'target' in item
  );
}

export const SubFlowViewer = memo(function SubFlowViewer({ report }: SubFlowViewerProps) {
  // 订阅 locale：viewer 被 memo 包裹，locale 切换时需主动触发重渲染以刷新文案
  useCurrentLocale();
  const ll = getLL();
  const setFlow = useFlowStore((state) => state.setFlow);
  const subReport = report.sub_flow_report;
  const outputObj = (report.output_data && typeof report.output_data === 'object'
    ? report.output_data
    : {}) as Record<string, unknown>;
  const invocation = asInvocationInfo(report.input_data);

  const flowName = subReport?.flow_name || (outputObj.sub_flow_name as string) || report.node_name;
  const flowUuid = subReport?.flow_uuid || (outputObj.sub_flow_uuid as string) || '';
  const executedCount =
    subReport?.executed_count ??
    (typeof outputObj.executed_count === 'number' ? outputObj.executed_count : 0);
  const durationMs =
    subReport?.duration_ms ??
    (typeof outputObj.duration_ms === 'number' ? outputObj.duration_ms : report.duration_ms);
  const finalVariables = subReport?.final_variables ?? outputObj.final_variables;
  const initialVariables = subReport?.initial_variables;
  const statistics = subReport?.statistics;
  const skippedNodes = subReport?.skipped_nodes ?? [];
  const childReports = subReport?.node_reports ?? [];

  const status = subReport?.status ?? report.status;
  const isSuccess = status === 'success';
  const isCancelled = status === 'cancelled';

  const inputMappings = asMappings(invocation?.variable_mappings);
  const outputMappings = asMappings(invocation?.output_mappings);
  const inputMappingCount = invocation?.variable_mappings
    ? inputMappings.length
    : (invocation?.variable_mappings_count ?? 0);
  const outputMappingCount = invocation?.output_mappings
    ? outputMappings.length
    : (invocation?.output_mappings_count ?? 0);

  const hasFinalVariables =
    Boolean(finalVariables) &&
    typeof finalVariables === 'object' &&
    Object.keys(finalVariables as object).length > 0;
  const hasInitialVariables =
    Boolean(initialVariables) &&
    typeof initialVariables === 'object' &&
    Object.keys(initialVariables as object).length > 0;

  const handleOpenTargetFlow = async () => {
    if (!flowUuid) return;
    try {
      const flow = await flowStorage.loadFlow(flowUuid);
      if (flow) {
        setFlow(flow);
        toast.success(flow.name);
      }
    } catch (err) {
      toast.error(String(err));
    }
  };

  return (
    <div className="space-y-4">
      {/* 1. 子流程执行概览 */}
      <div className="p-4 rounded-xl border border-violet-500/30 bg-violet-500/5 space-y-3">
        <div className="flex items-start justify-between gap-2">
          <div className="flex items-center gap-2.5 min-w-0">
            <div className="p-2 rounded-lg bg-violet-500 text-white shadow-sm shrink-0">
              <Workflow className="w-5 h-5" />
            </div>
            <div className="min-w-0">
              <h3 className="font-semibold text-base text-foreground flex items-center gap-2">
                <span className="truncate" title={flowName}>
                  {flowName}
                </span>
                {flowUuid && (
                  <button
                    type="button"
                    onClick={handleOpenTargetFlow}
                    title={ll.panels.subflow.open_target_flow()}
                    className="shrink-0 text-violet-600 hover:text-violet-700 hover:bg-violet-100 dark:hover:bg-violet-950 p-1 rounded transition-colors"
                  >
                    <ExternalLink className="w-3.5 h-3.5" />
                  </button>
                )}
              </h3>
              {flowUuid && <p className="text-xs font-mono text-muted-foreground truncate">{flowUuid}</p>}
            </div>
          </div>

          <span
            className={cn(
              'inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium shrink-0',
              isSuccess
                ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-950/50 dark:text-emerald-400'
                : isCancelled
                  ? 'bg-amber-100 text-amber-700 dark:bg-amber-950/50 dark:text-amber-400'
                  : 'bg-rose-100 text-rose-700 dark:bg-rose-950/50 dark:text-rose-400'
            )}
          >
            {isSuccess ? (
              <>
                <CheckCircle2 className="w-3 h-3" />
                Success
              </>
            ) : isCancelled ? (
              <>
                <Ban className="w-3 h-3" />
                Cancelled
              </>
            ) : (
              <>
                <XCircle className="w-3 h-3" />
                Failed
              </>
            )}
          </span>
        </div>

        {/* 执行统计：与顶层报告一致的指标口径 */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 pt-2 border-t border-violet-500/20">
          <StatCard label={ll.execution.report.sub_flow_total_nodes()} value={subReport?.total_nodes ?? '—'} />
          <StatCard label={ll.execution.report.executed_nodes()} value={executedCount} />
          <StatCard
            label={ll.execution.report.sub_flow_success_nodes()}
            value={statistics?.success_count ?? '—'}
            valueClassName="text-emerald-600 dark:text-emerald-400"
          />
          <StatCard
            label={ll.execution.report.sub_flow_failed_nodes()}
            value={statistics?.failed_count ?? '—'}
            valueClassName={
              (statistics?.failed_count ?? 0) > 0 ? 'text-rose-600 dark:text-rose-400' : undefined
            }
          />
        </div>

        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <Clock className="w-3.5 h-3.5 text-violet-500" />
          <span>{ll.execution.report.total_duration()}:</span>
          <span className="font-medium text-foreground tabular-nums">{durationMs} ms</span>
        </div>
      </div>

      {/* 2. 子流程级错误 / 警告 */}
      {subReport?.flow_error && (
        <ErrorSection
          error={subReport.flow_error}
          nodeName={flowName}
          nodeType="sub_flow"
          nodeUuid={report.node_uuid}
        />
      )}
      {subReport?.flow_warning && (
        <div className="flex items-start gap-2 p-3 rounded-lg border border-amber-300/60 bg-amber-50 dark:border-amber-800/60 dark:bg-amber-950/30 text-xs text-amber-800 dark:text-amber-300">
          <AlertTriangle className="w-3.5 h-3.5 shrink-0 mt-0.5" />
          <div className="min-w-0">
            <div className="font-medium">{ll.execution.report.flow_warning()}</div>
            <div className="break-words opacity-90">{subReport.flow_warning}</div>
          </div>
        </div>
      )}
      {subReport?.end_node_reached === false && (
        <div className="flex items-center gap-2 p-3 rounded-lg border border-amber-300/60 bg-amber-50 dark:border-amber-800/60 dark:bg-amber-950/30 text-xs text-amber-800 dark:text-amber-300">
          <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
          {ll.execution.report.sub_flow_not_reached_end()}
        </div>
      )}

      {/* 3. 调用配置（目标流程引用 + 变量映射策略） */}
      {invocation && (
        <CollapsibleSection
          title={ll.execution.report.sub_flow_invocation()}
          icon={<SlidersHorizontal className="w-3.5 h-3.5 text-violet-500" />}
          defaultOpen
        >
          <div className="space-y-3 text-xs">
            <div className="flex items-center gap-2">
              <span className="text-muted-foreground shrink-0">
                {ll.panels.subflow.target_flow()}:
              </span>
              <code className="font-mono text-[11px] text-foreground truncate">
                {invocation.target_flow_path ||
                  invocation.target_flow_id ||
                  (flowUuid ? flowUuid : '—')}
              </code>
            </div>

            <div className="flex flex-wrap gap-2">
              <span
                className={cn(
                  'inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-medium',
                  invocation.inherit_all_variables
                    ? 'bg-violet-100 text-violet-700 dark:bg-violet-950/50 dark:text-violet-300'
                    : 'bg-muted text-muted-foreground'
                )}
              >
                {invocation.inherit_all_variables ? '✓ ' : '✗ '}
                {ll.panels.subflow.inherit_all_variables()}
              </span>
              <span
                className={cn(
                  'inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-medium',
                  invocation.export_all_variables
                    ? 'bg-violet-100 text-violet-700 dark:bg-violet-950/50 dark:text-violet-300'
                    : 'bg-muted text-muted-foreground'
                )}
              >
                {invocation.export_all_variables ? '✓ ' : '✗ '}
                {ll.panels.subflow.export_all_variables()}
              </span>
            </div>

            {inputMappingCount > 0 && (
              <div className="space-y-1.5">
                <div className="font-medium text-muted-foreground">
                  {ll.panels.subflow.input_mappings()} ({inputMappingCount})
                </div>
                {inputMappings.length > 0 ? (
                  <MappingTable mappings={inputMappings} />
                ) : (
                  <div className="text-muted-foreground italic">
                    {ll.panels.subflow.no_input_mappings()}
                  </div>
                )}
              </div>
            )}

            {outputMappingCount > 0 && (
              <div className="space-y-1.5">
                <div className="font-medium text-muted-foreground">
                  {ll.panels.subflow.output_mappings()} ({outputMappingCount})
                </div>
                {outputMappings.length > 0 ? (
                  <MappingTable mappings={outputMappings} />
                ) : (
                  <div className="text-muted-foreground italic">
                    {ll.panels.subflow.no_output_mappings()}
                  </div>
                )}
              </div>
            )}
          </div>
        </CollapsibleSection>
      )}

      {/* 4. 子流程输入变量（继承快照 + 输入覆盖后的实际入参） */}
      {hasInitialVariables && (
        <CollapsibleSection
          title={ll.execution.report.sub_flow_input_variables()}
          icon={<Database className="w-3.5 h-3.5 text-violet-500" />}
        >
          <RawJsonViewer data={initialVariables} />
        </CollapsibleSection>
      )}

      {/* 5. 子流程输出变量 */}
      {hasFinalVariables && (
        <CollapsibleSection
          title={ll.execution.report.sub_flow_output_variables()}
          icon={<Database className="w-3.5 h-3.5 text-violet-500" />}
          defaultOpen
        >
          <RawJsonViewer data={finalVariables} />
        </CollapsibleSection>
      )}

      {/* 6. 回写到父流程的变量变更 */}
      {report.variable_changes && report.variable_changes.length > 0 && (
        <div className="space-y-1.5">
          <div className="flex items-center gap-1.5 px-1 text-xs font-medium text-muted-foreground">
            <LogOut className="w-3.5 h-3.5 text-violet-500" />
            {ll.panels.subflow.export_all_variables()}
          </div>
          <VariableChangesCard changes={report.variable_changes} defaultOpen />
        </div>
      )}

      {/* 7. 跳过的节点 */}
      {skippedNodes.length > 0 && (
        <CollapsibleSection
          title={ll.execution.report.sub_flow_skipped_nodes({ count: skippedNodes.length })}
          icon={<Ban className="w-3.5 h-3.5 text-amber-500" />}
        >
          <div className="space-y-1.5 text-xs">
            {skippedNodes.map((skipped) => (
              <div key={skipped.node_uuid} className="flex items-start gap-2">
                <code className="font-mono text-[11px] text-foreground shrink-0">
                  {skipped.node_name}
                </code>
                <span className="text-muted-foreground break-words">{skipped.reason}</span>
              </div>
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 8. 子节点执行明细 — 复用普通节点的结构化 Viewer，支持任意层级嵌套 */}
      <div className="space-y-2">
        <div className="flex items-center gap-1.5 px-1 text-xs font-medium text-muted-foreground">
          <ListTree className="w-3.5 h-3.5 text-violet-500" />
          {ll.execution.report.sub_flow_child_nodes({ count: childReports.length })}
        </div>
        {childReports.length === 0 ? (
          <div className="p-4 rounded-lg border border-border text-center text-xs text-muted-foreground">
            {ll.execution.report.no_nodes()}
          </div>
        ) : (
          <div className="space-y-2">
            {childReports.map((childReport) => (
              <NodeExecutionDetails key={childReport.node_uuid} report={childReport} />
            ))}
          </div>
        )}
      </div>

      {/* 兜底：仅有 output_data、缺少结构化子报告时，仍保留原始执行结果 */}
      {!subReport && report.output_data !== undefined && report.output_data !== null && (
        <CollapsibleSection
          title={ll.execution.viewers.general_output()}
          icon={<Layers className="w-3.5 h-3.5 text-violet-500" />}
        >
          <RawJsonViewer data={report.output_data} />
        </CollapsibleSection>
      )}
    </div>
  );
});
