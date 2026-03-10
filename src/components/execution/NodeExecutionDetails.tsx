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
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { NODE_TYPE_NAMES, NODE_TYPE_COLORS } from './constants';
import { RawJsonViewer } from './viewers/RawJsonViewer';
import { HttpNodeViewer } from './viewers/HttpNodeViewer';
import { WsConnectViewer } from './viewers/WsConnectViewer';
import { WsSendWaitViewer } from './viewers/WsSendWaitViewer';
import { WsCloseViewer } from './viewers/WsCloseViewer';
import { ConditionalViewer } from './viewers/ConditionalViewer';
import { GenericViewer } from './viewers/GenericViewer';

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
  
  switch (nodeType) {
    case 'http':
      return <HttpNodeViewer report={report} />;
    case 'ws_connect':
      return <WsConnectViewer report={report} />;
    case 'ws_send_wait':
      return <WsSendWaitViewer report={report} />;
    case 'ws_close':
      return <WsCloseViewer report={report} />;
    case 'conditional':
      return <ConditionalViewer report={report} />;
    default:
      return <GenericViewer report={report} />;
  }
}


export function NodeExecutionDetails({ report }: NodeExecutionDetailsProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [activeTab, setActiveTab] = useState<'structured' | 'raw'>('structured');

  const isSuccess = report.status === 'success';
  const nodeTypeLabel = NODE_TYPE_NAMES[report.node_type] || report.node_type;
  const nodeTypeColor = NODE_TYPE_COLORS[report.node_type] || '';

  return (
    <div
      className={cn(
        'border rounded-xl overflow-hidden transition-all duration-200',
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
        <div className="border-t border-border/50 bg-muted/20">
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
              结构化
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
              原始 JSON
            </button>
          </div>

          {/* Tab 内容区域 */}
          <div className="p-4 text-sm">
            {activeTab === 'structured' ? (
              <NodeViewerSelector report={report} />
            ) : (
              /* 原始 JSON 视图 */
              <div className="min-w-0">
                <RawJsonViewer data={report} />
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
