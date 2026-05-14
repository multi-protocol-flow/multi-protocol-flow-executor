/**
 * 节点执行列表项组件
 * 用于在执行报告列表中显示单个节点的执行信息
 */
import { CheckCircle2, XCircle, Clock } from 'lucide-react';
import { cn } from '@/lib/utils';
import { getNodeMeta } from '@/lib/nodeMeta';
import type { NodeExecutionReport } from '@/types';

interface NodeListItemProps {
  /** 节点执行报告 */
  report: NodeExecutionReport;
  /** 是否选中 */
  isSelected: boolean;
  /** 点击回调 */
  onClick: () => void;
}

export function NodeListItem({ report, isSelected, onClick }: NodeListItemProps) {
  const isSuccess = report.status === 'success';
  const meta = getNodeMeta(report.node_type);
  const nodeTypeLabel = meta?.name ?? report.node_type;

  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'w-full flex items-center gap-2 p-2 text-left rounded-lg transition-colors',
        isSelected ? 'bg-accent' : 'hover:bg-muted/50'
      )}
    >
      {/* 状态图标 */}
      {isSuccess ? (
        <CheckCircle2 className="w-4 h-4 text-emerald-500 shrink-0" />
      ) : (
        <XCircle className="w-4 h-4 text-red-500 shrink-0" />
      )}

      {/* 节点名称 */}
      <span className="font-medium text-foreground truncate flex-1">
        {report.node_name}
      </span>

      {/* 类型标签 */}
      <span
        className={cn(
          'inline-flex items-center rounded px-1.5 py-0.5 text-xs font-medium leading-none shrink-0',
          meta?.badge || 'bg-muted text-muted-foreground',
        )}
      >
        {nodeTypeLabel}
      </span>

      {/* 执行时间 */}
      <div className="flex items-center gap-1 text-sm text-muted-foreground shrink-0">
        <Clock className="w-3.5 h-3.5" />
        <span>{report.duration_ms}ms</span>
      </div>
    </button>
  );
}
