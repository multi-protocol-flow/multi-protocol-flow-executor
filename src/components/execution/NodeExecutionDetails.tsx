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
  ArrowRightLeft,
  AlertTriangle,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

interface NodeExecutionDetailsProps {
  report: NodeExecutionReport;
}

/** 节点类型中文名映射 */
const NODE_TYPE_NAMES: Record<string, string> = {
  entry: '入口',
  http: 'HTTP 请求',
  conditional: '条件分支',
  end: '终止',
};

/** 节点类型颜色映射 */
const NODE_TYPE_COLORS: Record<string, string> = {
  entry: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-300',
  http: 'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300',
  conditional: 'bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300',
  end: 'bg-slate-100 text-slate-700 dark:bg-slate-900/40 dark:text-slate-300',
};

export function NodeExecutionDetails({ report }: NodeExecutionDetailsProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  const isSuccess = report.status === 'success';
  const nodeTypeLabel = NODE_TYPE_NAMES[report.node_type] || report.node_type;
  const nodeTypeColor = NODE_TYPE_COLORS[report.node_type] || '';

  return (
    <div
      className={cn(
        'border rounded-xl overflow-hidden transition-all duration-200',
        isSuccess
          ? 'border-emerald-200/60 dark:border-emerald-800/40'
          : 'border-red-200/60 dark:border-red-800/40'
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
        <div className="border-t border-border/50 p-4 space-y-3 text-sm bg-muted/20">
          {/* 输入数据 */}
          {report.input_data !== undefined && (
            <div>
              <div className="flex items-center gap-1.5 mb-1.5 text-muted-foreground">
                <ArrowRightLeft className="w-3.5 h-3.5" />
                <span className="font-medium">输入数据</span>
              </div>
              <pre className="p-3 bg-slate-900 dark:bg-slate-950 rounded-lg text-xs overflow-x-auto text-slate-100">
                {JSON.stringify(report.input_data, null, 2)}
              </pre>
            </div>
          )}

          {/* 输出数据 */}
          {report.output_data !== undefined && (
            <div>
              <div className="flex items-center gap-1.5 mb-1.5 text-muted-foreground">
                <ArrowRightLeft className="w-3.5 h-3.5 rotate-180" />
                <span className="font-medium">输出数据</span>
              </div>
              <pre className="p-3 bg-slate-900 dark:bg-slate-950 rounded-lg text-xs overflow-x-auto text-slate-100">
                {JSON.stringify(report.output_data, null, 2)}
              </pre>
            </div>
          )}

          {/* 使用的端口 */}
          {report.used_port_id && (
            <div className="flex items-center gap-2">
              <span className="text-muted-foreground font-medium">输出端口:</span>
              <Badge variant="secondary" className="font-mono text-xs">
                {report.used_port_id}
              </Badge>
            </div>
          )}

          {/* 错误信息 */}
          {report.error && (
            <div className="rounded-lg bg-red-50 dark:bg-red-950/30 p-3 border border-red-200/50 dark:border-red-800/30">
              <div className="flex items-center gap-1.5 mb-1 text-red-600 dark:text-red-400">
                <AlertTriangle className="w-3.5 h-3.5" />
                <span className="font-medium">错误信息</span>
              </div>
              <p className="text-red-600 dark:text-red-400 text-sm">
                {report.error.message}
              </p>
              {report.error.stack_trace && (
                <pre className="mt-2 p-2 bg-red-100 dark:bg-red-900/30 rounded text-xs overflow-x-auto text-red-700 dark:text-red-300">
                  {report.error.stack_trace}
                </pre>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
