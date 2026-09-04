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
  Copy,
  Check,
  LayoutList,
  FileCode,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';
import { NODE_TYPE_NAMES, NODE_TYPE_COLORS } from './constants';
import { RawJsonViewer } from './viewers/RawJsonViewer';
import { getViewerComponent } from '@/lib/viewerRegistry';
import { ErrorSection } from './viewers/ErrorSection';

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
  const [copied, setCopied] = useState(false);

  const handleCopyRawJson = async () => {
    try {
      await navigator.clipboard.writeText(JSON.stringify(report, null, 2));
      setCopied(true);
      toast.success(ll.execution.report.copied_to_clipboard());
      setTimeout(() => setCopied(false), 2000);
    } catch {
      toast.error(ll.execution.report.copy_failed());
    }
  };

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
      <button
        type="button"
        onClick={() => setIsExpanded(!isExpanded)}
        className={cn(
          'w-full flex items-center justify-between p-3',
          'hover:bg-muted/50 transition-colors duration-150',
          'text-left'
        )}
      >
        <div className="flex items-center gap-2.5">
          {/* 展开图标 */}
          <div className="text-muted-foreground">
            {isExpanded ? (
              <ChevronDown className="w-4 h-4" />
            ) : (
              <ChevronRight className="w-4 h-4" />
            )}
          </div>

          {/* 状态图标 */}
          {isRunning ? (
            <Loader2 className="w-4 h-4 text-blue-500 animate-spin" />
          ) : isSuccess ? (
            <CheckCircle2 className="w-4 h-4 text-emerald-500" />
          ) : isCancelled ? (
            <Ban className="w-4 h-4 text-amber-500" />
          ) : (
            <XCircle className="w-4 h-4 text-red-500" />
          )}

          {/* 节点名称 */}
          <span className="font-medium text-foreground">{report.node_name}</span>

          {/* 节点类型标签 */}
          <Badge
            variant="outline"
            className={cn('text-xs font-normal border-0', nodeTypeColor)}
          >
            {nodeTypeLabel}
          </Badge>
        </div>

        {/* 耗时 */}
        <div className="flex items-center gap-1.5 text-sm text-muted-foreground">
          <Clock className="w-3.5 h-3.5" />
          <span>{report.duration_ms}ms</span>
        </div>
      </button>

      {/* 展开内容 */}
      {isExpanded && (
        <div className="border-t border-border/50 bg-muted/20 overflow-hidden">
          {/* Tab 切换栏 */}
          <div className="flex items-center px-4 pt-3 pb-1 border-b border-border/40 bg-muted/30">
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
          </div>
          {/* Tab 内容区域 */}
          <div className="p-4 text-sm overflow-hidden">
            {activeTab === 'structured' ? (
              <div className="space-y-3">
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
                <div className="flex items-center justify-end mb-2">
                  <button
                    type="button"
                    onClick={handleCopyRawJson}
                    className="inline-flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium rounded-md border border-border hover:bg-muted/50 transition-colors text-muted-foreground hover:text-foreground"
                  >
                    {copied ? (
                      <Check className="w-3.5 h-3.5 text-emerald-500" />
                    ) : (
                      <Copy className="w-3.5 h-3.5" />
                    )}
                    {copied ? ll.execution.report.copied() : ll.execution.report.copy_all()}
                  </button>
                </div>
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
