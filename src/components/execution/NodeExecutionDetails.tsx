/**
 * 节点执行详情组件
 * 显示单个节点的执行结果，包括输入/输出数据和错误信息
 */
import { useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
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
import { NODE_TYPE_NAMES, NODE_TYPE_COLORS } from './constants';
import { RawJsonViewer } from './viewers/RawJsonViewer';
import { getViewerComponent } from '@/lib/viewerRegistry';
import { ErrorSection } from './viewers/ErrorSection';
import { CopyButton } from '@/components/common';
interface NodeExecutionDetailsProps {
  report: NodeExecutionReport;
  defaultExpanded?: boolean;
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


export function NodeExecutionDetails({ report, defaultExpanded = false }: NodeExecutionDetailsProps) {
  const ll = getLL();
  const [isExpanded, setIsExpanded] = useState(defaultExpanded);
  const [activeTab, setActiveTab] = useState<'structured' | 'raw'>('structured');

  const status = report.status;
  const isRunning = (status as string) === 'running';
  const isSuccess = status === 'success';
  const isCancelled = status === 'cancelled';
  const nodeTypeLabel = NODE_TYPE_NAMES[report.node_type] || report.node_type;
  const nodeTypeColor = NODE_TYPE_COLORS[report.node_type] || '';

  return (
    <div
      className={cn(
        'border rounded-xl overflow-hidden transition-all duration-200 min-w-0',
        isRunning
          ? 'border-blue-300 dark:border-blue-700 shadow-xs'
          : isSuccess
            ? 'border-emerald-200/60 dark:border-emerald-800/60'
            : isCancelled
              ? 'border-amber-200/60 dark:border-amber-800/60'
              : 'border-red-200/60 dark:border-red-800/60'
      )}
    >
      {/* 标题栏 */}
      <div
        className={cn(
          'w-full flex items-center justify-between p-3 group/header',
          'hover:bg-muted/50 transition-colors duration-150',
          'text-left'
        )}
      >
        <button
          type="button"
          onClick={() => setIsExpanded(!isExpanded)}
          className="flex-1 flex items-center gap-2.5 text-left min-w-0"
        >
          {/* 展开图标 */}
          <div className="text-muted-foreground shrink-0">
            {isExpanded ? (
              <ChevronDown className="w-4 h-4" />
            ) : (
              <ChevronRight className="w-4 h-4" />
            )}
          </div>

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

          {/* 节点名称 */}
          <span className="font-medium text-foreground truncate select-text cursor-text">{report.node_name}</span>

          {/* 节点类型标签 */}
          <Badge
            variant="outline"
            className={cn('text-xs font-normal border-0 shrink-0', nodeTypeColor)}
          >
            {nodeTypeLabel}
          </Badge>
        </button>

        <div className="flex items-center gap-2 shrink-0 ml-2">
          {/* 复制节点名按钮 */}
          <CopyButton
            value={report.node_name}
            title={ll.execution.viewers.copy_node_name()}
            size="xs"
            className="h-6 px-1.5 opacity-70 hover:opacity-100 transition-opacity"
          />

          {/* 耗时 */}
          <button
            type="button"
            onClick={() => setIsExpanded(!isExpanded)}
            className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground cursor-pointer"
          >
            <Clock className="w-3.5 h-3.5" />
            <span>{report.duration_ms}ms</span>
          </button>
        </div>
      </div>

      {/* 展开内容 */}
      {isExpanded && (
        <div className="border-t border-border/50 bg-muted/20 overflow-hidden">
          {/* Tab 切换栏 */}
          {/* Tab 切换栏 + 快捷操作 */}
          <div className="flex items-center justify-between px-4 pt-3 pb-1 border-b border-border/40 bg-muted/30">
            <div className="inline-flex h-8 items-center p-0.5 bg-muted/70 rounded-lg border border-border/40">
              <button
                type="button"
                onClick={() => setActiveTab('structured')}
                className={cn(
                  'inline-flex items-center justify-center gap-1.5 px-3 py-1 text-xs font-medium rounded-md transition-all duration-150 cursor-pointer',
                  activeTab === 'structured'
                    ? 'bg-background text-foreground shadow-xs font-semibold'
                    : 'text-muted-foreground hover:text-foreground'
                )}
              >
                <LayoutList className="w-3.5 h-3.5" />
                <span>{ll.execution.report.tab_structured()}</span>
              </button>
              <button
                type="button"
                onClick={() => setActiveTab('raw')}
                className={cn(
                  'inline-flex items-center justify-center gap-1.5 px-3 py-1 text-xs font-medium rounded-md transition-all duration-150 cursor-pointer',
                  activeTab === 'raw'
                    ? 'bg-background text-foreground shadow-xs font-semibold'
                    : 'text-muted-foreground hover:text-foreground'
                )}
              >
                <FileCode className="w-3.5 h-3.5" />
                <span>{ll.execution.report.tab_raw_json()}</span>
              </button>
            </div>
            <CopyButton
              value={report}
              format="json"
              label={ll.execution.report.copy_node_report()}
              title={ll.execution.report.copy_node_report()}
              size="xs"
            />
          </div>
          {/* Tab 内容区域 */}
          <div className="p-4 text-sm overflow-hidden">
            {activeTab === 'structured' ? (
              <div className="space-y-3">
                {/* 节点执行摘要条 */}
                <div className="flex flex-wrap items-center gap-2 px-3 py-2 bg-muted/40 rounded-lg border border-border/40 text-xs">
                  <div className="flex items-center gap-1">
                    <span className="font-semibold text-foreground select-text">{report.node_name}</span>
                    <CopyButton
                      value={report.node_name}
                      title={ll.execution.viewers.copy_node_name()}
                      size="xs"
                      className="h-5 p-0.5"
                    />
                  </div>
                  <span className="text-muted-foreground">•</span>
                  <span className="font-semibold text-muted-foreground">{ll.execution.report.status_summary()}:</span>
                  <Badge
                    variant="outline"
                    className={cn(
                      'font-mono text-xs',
                      isSuccess
                        ? 'text-emerald-700 dark:text-emerald-300 border-emerald-300 dark:border-emerald-700 bg-emerald-50 dark:bg-emerald-950/40'
                        : isCancelled
                          ? 'text-amber-700 dark:text-amber-300 border-amber-300 dark:border-amber-700 bg-amber-50 dark:bg-amber-950/40'
                          : 'text-red-700 dark:text-red-300 border-red-300 dark:border-red-700 bg-red-50 dark:bg-red-950/40'
                    )}
                  >
                    {report.status}
                  </Badge>
                  <span className="text-muted-foreground">•</span>
                  <span className="text-muted-foreground">{ll.execution.report.total_duration()}:</span>
                  <span className="font-mono font-medium">{report.duration_ms}ms</span>
                  {report.wait_duration_ms != null && report.wait_duration_ms > 0 && (
                    <>
                      <span className="text-muted-foreground">•</span>
                      <span className="text-muted-foreground">{ll.execution.report.wait_duration()}:</span>
                      <span className="font-mono">{report.wait_duration_ms}ms</span>
                    </>
                  )}
                  {report.used_port_id && (
                    <>
                      <span className="text-muted-foreground">•</span>
                      <span className="text-muted-foreground">{ll.execution.report.used_port()}:</span>
                      <Badge variant="outline" className="font-mono text-[11px] bg-background">
                        {report.used_port_id}
                      </Badge>
                    </>
                  )}
                  {report.variable_changes && report.variable_changes.length > 0 && (
                    <>
                      <span className="text-muted-foreground">•</span>
                      <Badge variant="outline" className="text-[11px] bg-violet-50 text-violet-700 border-violet-200 dark:bg-violet-950/40 dark:text-violet-300 dark:border-violet-800">
                        {report.variable_changes.length} {ll.execution.viewers.variable_changes()}
                      </Badge>
                    </>
                  )}
                </div>

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
      )}
    </div>
  );
}
