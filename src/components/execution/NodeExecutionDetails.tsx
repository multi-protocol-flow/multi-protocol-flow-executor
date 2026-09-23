/**
 * 节点执行详情组件
 * 显示单个节点的执行结果，包括输入/输出数据和错误信息
 */
import { useState } from 'react';
import {
  CheckCircle2,
  XCircle,
  Loader2,
  Ban,
  Clock,
  LayoutList,
  FileCode,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import { useFlowStore, getNodeStage } from '@/stores';
import { NODE_TYPE_NAMES, NODE_TYPE_COLORS } from './constants';
import { RawJsonViewer } from './viewers/RawJsonViewer';
import { getViewerComponent } from '@/lib/viewerRegistry';
import { ErrorSection } from './viewers/ErrorSection';
import { CopyButton } from '@/components/common';
export interface NodeExecutionDetailsProps {
  report: NodeExecutionReport;
  className?: string;
}
/**
 * 将 PascalCase 转换为 snake_case
 */
function toSnakeCase(str: string): string {
  return str.replace(/([A-Z])/g, '_$1').toLowerCase().replace(/^_/, '');
}

/**
 * 根据 node_type 选择对应的 Viewer 组件
 */
function NodeViewerSelector({ report }: NodeExecutionDetailsProps) {
  // 后端返回的 node_type 可能是 PascalCase（如 "WsConnect"），转为 snake_case 匹配
  const nodeType = toSnakeCase(report.node_type);
  // report 透传：getViewerComponent 插件分支依赖 report.plugin_data 判定
  const ViewerComponent = getViewerComponent(nodeType, report);
  return <ViewerComponent report={report} />;
}


export function NodeExecutionDetails({ report, className }: NodeExecutionDetailsProps) {
  const ll = getLL();
  const [activeTab, setActiveTab] = useState<'structured' | 'raw'>('structured');

  const status = report.status;
  const isRunning = (status as string) === 'running';
  const isSuccess = status === 'success';
  const isCancelled = status === 'cancelled';
  const nodeTypeLabel = NODE_TYPE_NAMES[report.node_type] || report.node_type;
  const nodeTypeColor = NODE_TYPE_COLORS[report.node_type] || '';
  const flow = useFlowStore((s) => s.flow);
  const stage = flow ? getNodeStage(flow, report.node_uuid) : null;
  const stageLl = getLL().stages;

  return (
    <div
      className={cn(
        'border rounded-xl overflow-hidden transition-all duration-200 min-w-0 bg-card',
        isRunning
          ? 'border-blue-300 dark:border-blue-700 shadow-xs'
          : isSuccess
            ? 'border-emerald-200/60 dark:border-emerald-800/60'
            : isCancelled
              ? 'border-amber-200/60 dark:border-amber-800/60'
              : 'border-red-200/60 dark:border-red-800/60',
        className
      )}
    >
      {/* 顶栏 Header：分层自适应排版，确保在任意窄宽度下永不重叠 */}
      <div className="w-full flex flex-col gap-2 px-3.5 py-2.5 bg-muted/40 border-b border-border/50">
        {/* 第一行：节点状态 + 名称 + 类型标签 + 耗时 + 快捷复制 */}
        <div className="flex flex-wrap items-center justify-between gap-y-1.5 gap-x-2 min-w-0 w-full">
          <div className="flex flex-wrap items-center gap-2 min-w-0 flex-1">
            {/* 状态图标 */}
            {isRunning ? (
              <Loader2 className="w-4 h-4 text-blue-500 animate-spin shrink-0" />
            ) : isSuccess ? (
              <CheckCircle2 className="w-4 h-4 text-emerald-500 shrink-0" />
            ) : isCancelled ? (
              <Ban className="w-4 h-4 text-amber-500 shrink-0" />
            ) : (
              <XCircle className="w-4 h-4 text-red-500 shrink-0" />
            )}

            {/* 节点名称（自动截断，防止窄屏撑爆） */}
            <span
              className="font-semibold text-foreground text-sm truncate select-text cursor-text min-w-0"
              title={report.node_name}
            >
              {report.node_name}
            </span>

            {/* 所属阶段标记 (Setup / Teardown) */}
            {stage && stage !== 'main' && (
              <span
                className={cn(
                  'shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium leading-none border',
                  stage === 'setup'
                    ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20'
                    : 'bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-500/20',
                )}
              >
                {stage === 'setup' ? stageLl.short_setup() : stageLl.short_teardown()}
              </span>
            )}
            {/* 节点类型标签 */}
            <Badge
              variant="outline"
              className={cn('text-xs font-normal border-0 shrink-0', nodeTypeColor)}
            >
              {nodeTypeLabel}
            </Badge>
          </div>

          {/* 右侧：耗时 + 复制节点名 */}
          <div className="flex items-center gap-1.5 shrink-0 ml-auto">
            <div className="flex items-center gap-1 text-xs text-muted-foreground font-mono">
              <Clock className="w-3.5 h-3.5 shrink-0" />
              <span>{report.duration_ms}ms</span>
            </div>

            <CopyButton
              value={report.node_name}
              title={ll.execution.viewers.copy_node_name()}
              size="xs"
              className="h-6 px-1.5 opacity-70 hover:opacity-100 transition-opacity"
            />
          </div>
        </div>

        {/* 第二行：操作控制栏（Tab 切换 + 完整报告复制） */}
        <div className="flex flex-wrap items-center justify-between gap-2 w-full pt-1.5 border-t border-border/30">
          <div className="inline-flex h-7 items-center p-0.5 bg-muted/70 rounded-lg border border-border/40 shrink-0">
            <button
              type="button"
              onClick={() => setActiveTab('structured')}
              className={cn(
                'inline-flex items-center justify-center gap-1 px-2.5 h-6 text-xs font-medium rounded-md transition-all duration-150 cursor-pointer',
                activeTab === 'structured'
                  ? 'bg-background text-foreground shadow-xs font-semibold'
                  : 'text-muted-foreground hover:text-foreground'
              )}
            >
              <LayoutList className="w-3 h-3 shrink-0" />
              <span>{ll.execution.report.tab_structured()}</span>
            </button>
            <button
              type="button"
              onClick={() => setActiveTab('raw')}
              className={cn(
                'inline-flex items-center justify-center gap-1 px-2.5 h-6 text-xs font-medium rounded-md transition-all duration-150 cursor-pointer',
                activeTab === 'raw'
                  ? 'bg-background text-foreground shadow-xs font-semibold'
                  : 'text-muted-foreground hover:text-foreground'
              )}
            >
              <FileCode className="w-3 h-3 shrink-0" />
              <span>{ll.execution.report.tab_raw_json()}</span>
            </button>
          </div>

          <CopyButton
            value={report}
            format="json"
            label={ll.execution.report.copy_node_report()}
            title={ll.execution.report.copy_node_report()}
            size="xs"
            className="h-7 px-2 shrink-0 text-xs ml-auto"
          />
        </div>
      </div>
      {/* 详情内容区（直接平铺渲染） */}
      <div className="p-4 text-sm overflow-hidden">
        {report.is_detail_stripped && (
          <div className="flex items-center gap-2 p-2.5 mb-3 bg-muted/40 border border-border/50 rounded-lg text-xs text-muted-foreground">
            <Loader2 className="w-3.5 h-3.5 animate-spin text-blue-500 shrink-0" />
            <span>{ll.execution.history.loading()}</span>
          </div>
        )}
        {activeTab === 'structured' ? (
          <div className="space-y-3">
            {/* 补充执行摘要（等待耗时、出口端口、变量变更） */}
            {(Boolean(report.wait_duration_ms && report.wait_duration_ms > 0) || Boolean(report.used_port_id) || Boolean(report.variable_changes && report.variable_changes.length > 0)) && (
              <div className="flex flex-wrap items-center gap-2 px-3 py-1.5 bg-muted/40 rounded-lg border border-border/40 text-xs">
                {report.wait_duration_ms != null && report.wait_duration_ms > 0 && (
                  <div className="flex items-center gap-1">
                    <span className="text-muted-foreground">{ll.execution.report.wait_duration()}:</span>
                    <span className="font-mono">{report.wait_duration_ms}ms</span>
                  </div>
                )}
                {report.used_port_id && (
                  <div className="flex items-center gap-1">
                    {report.wait_duration_ms != null && report.wait_duration_ms > 0 && <span className="text-muted-foreground">•</span>}
                    <span className="text-muted-foreground">{ll.execution.report.used_port()}:</span>
                    <Badge variant="outline" className="font-mono text-[11px] bg-background">
                      {report.used_port_id}
                    </Badge>
                  </div>
                )}
                {report.variable_changes && report.variable_changes.length > 0 && (
                  <div className="flex items-center gap-1">
                    {(Boolean(report.wait_duration_ms && report.wait_duration_ms > 0) || Boolean(report.used_port_id)) && <span className="text-muted-foreground">•</span>}
                    <Badge variant="outline" className="text-[11px] bg-violet-50 text-violet-700 border-violet-200 dark:bg-violet-950/40 dark:text-violet-300 dark:border-violet-800">
                      {report.variable_changes.length} {ll.execution.viewers.variable_changes()}
                    </Badge>
                  </div>
                )}
              </div>
            )}

            {/* 节点级错误展示（在 viewer 上方，覆盖所有节点类型） */}
            {report.error && report.status === 'failed' && (
              <ErrorSection
                error={report.error}
                nodeName={report.node_name}
                nodeType={report.node_type}
                nodeUuid={report.node_uuid}
              />
            )}
            <NodeViewerSelector report={report} />
          </div>
        ) : (
          /* 原始 JSON 视图 */
          <div className="min-w-0">
            <div className="overflow-x-auto">
              <RawJsonViewer data={report} />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
