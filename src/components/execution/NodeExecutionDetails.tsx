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
  Clock,
  Copy,
  Check,
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
  const ViewerComponent = getViewerComponent(nodeType);
  return <ViewerComponent report={report} />;
}


export function NodeExecutionDetails({ report }: NodeExecutionDetailsProps) {
  const ll = getLL();
  const [isExpanded, setIsExpanded] = useState(false);
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

  const isSuccess = report.status === 'success';
  const nodeTypeLabel = NODE_TYPE_NAMES[report.node_type] || report.node_type;
  const nodeTypeColor = NODE_TYPE_COLORS[report.node_type] || '';

  return (
    <div
      className={cn(
        'border rounded-xl overflow-hidden transition-all duration-200 min-w-0',
        isSuccess
          ? 'border-emerald-200/60'
          : 'border-red-200/60'
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
          {isSuccess ? (
            <CheckCircle2 className="w-4 h-4 text-emerald-500" />
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
          <div className="flex border-b border-border/50">
            <button
              type="button"
              onClick={() => setActiveTab('structured')}
              className={cn(
                'px-4 py-2 text-sm font-medium transition-colors',
                'border-b-2 -mb-px',
                activeTab === 'structured'
                  ? 'border-foreground text-foreground'
                  : 'border-transparent text-muted-foreground hover:text-foreground'
              )}
            >
              {ll.execution.report.tab_structured()}
            </button>
            <button
              type="button"
              onClick={() => setActiveTab('raw')}
              className={cn(
                'px-4 py-2 text-sm font-medium transition-colors',
                'border-b-2 -mb-px',
                activeTab === 'raw'
                  ? 'border-foreground text-foreground'
                  : 'border-transparent text-muted-foreground hover:text-foreground'
              )}
            >
              {ll.execution.report.tab_raw_json()}
            </button>
          </div>

          {/* Tab 内容区域 */}
          <div className="p-4 text-sm overflow-hidden">
            {activeTab === 'structured' ? (
              <div className="space-y-3">
                {/* 节点级错误展示（在 viewer 上方，覆盖所有节点类型） */}
                {report.error && report.status === 'failed' && (
                  <ErrorSection error={report.error} />
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
