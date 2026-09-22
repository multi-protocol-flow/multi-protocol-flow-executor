/**
 * 节点执行列表项组件
 * 用于在执行报告列表中显示单个节点的执行信息，支持 SubFlow 嵌套子节点树状展开
 * （任意层级：子流程内可再调用子流程）
 */
import { useState, useEffect } from 'react';
import {
  CheckCircle2,
  XCircle,
  Clock,
  Loader2,
  Ban,
  ChevronRight,
  ChevronDown,
  Workflow,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { getNodeMeta } from '@/lib/nodeMeta';
import { containsNodeReport } from '@/lib/reportTree';
import { useFlowStore, getNodeStage } from '@/stores';
import { getLL } from '@/i18n/i18n-lazy';
import type { NodeExecutionReport } from '@/types';

interface NodeListItemProps {
  /** 节点执行报告 */
  report: NodeExecutionReport;
  /** 是否选中（此顶层节点本身） */
  isSelected: boolean;
  /** 点击此顶层节点回调 */
  onClick: () => void;
  /** 编号前缀（如 "1" 或 "2"） */
  indexPrefix?: string;
  /** 当前选中的任意节点 UUID（含子节点） */
  selectedNodeUuid?: string | null;
  /** 选中任意节点回调（含子节点） */
  onSelectNode?: (nodeUuid: string) => void;
}

interface NodeTreeRowProps {
  report: NodeExecutionReport;
  /** 嵌套层级，顶层为 0 */
  depth: number;
  indexPrefix: string;
  isSelected: boolean;
  selectedNodeUuid?: string | null;
  onSelect: () => void;
  /** 选中任意层级节点回调（子节点使用） */
  onSelectNode?: (nodeUuid: string) => void;
}

/** 节点状态图标 */
function StatusIcon({
  status,
  size,
}: {
  status: NodeExecutionReport['status'];
  size: 'sm' | 'xs';
}) {
  const className = size === 'sm' ? 'w-3.5 h-3.5' : 'w-3 h-3';
  if ((status as string) === 'running') {
    return <Loader2 className={cn(className, 'text-blue-500 animate-spin shrink-0')} />;
  }
  if (status === 'success') {
    return <CheckCircle2 className={cn(className, 'text-emerald-500 shrink-0')} />;
  }
  if (status === 'cancelled') {
    return <Ban className={cn(className, 'text-amber-500 shrink-0')} />;
  }
  return <XCircle className={cn(className, 'text-rose-500 shrink-0')} />;
}

/**
 * 递归渲染单个节点行及其嵌套子流程节点
 *
 * 每层自持展开状态；任意层级都按同一套信息密度展示（名称 / 类型 / 耗时 / 状态）。
 */
function NodeTreeRow({
  report,
  depth,
  indexPrefix,
  isSelected,
  selectedNodeUuid,
  onSelect,
  onSelectNode,
}: NodeTreeRowProps) {
  const flow = useFlowStore((s) => s.flow);
  const stage = flow ? getNodeStage(flow, report.node_uuid) : null;
  const stageLl = getLL().stages;
  const subNodes = report.sub_flow_report?.node_reports;
  const hasSubNodes = Boolean(subNodes && subNodes.length > 0);
  const isDescendantSelected = containsNodeReport(subNodes, selectedNodeUuid);
  const [isExpanded, setIsExpanded] = useState<boolean>(isDescendantSelected);
  // 选中的是更深层子节点时自动展开
  useEffect(() => {
    if (isDescendantSelected) {
      setIsExpanded(true);
    }
  }, [isDescendantSelected]);

  const meta = getNodeMeta(report.node_type);
  const isTopLevel = depth === 0;

  return (
    <div className="w-full flex flex-col space-y-0.5">
      <button
        type="button"
        onClick={onSelect}
        className={cn(
          'w-full flex items-center gap-1.5 text-left rounded-lg transition-colors cursor-pointer',
          isTopLevel ? 'px-2 py-1.5' : 'px-1.5 py-1 rounded-md',
          isSelected ? (isTopLevel ? 'bg-accent font-medium' : 'bg-accent/80 font-medium') : 'hover:bg-muted/50'
        )}
      >
        {/* 折叠/展开 Chevron 按钮（仅当有子节点时呈现，避免普通节点左侧产生冗余占位白边） */}
        {hasSubNodes && (
          <span
            role="button"
            tabIndex={0}
            onClick={(e) => {
              e.stopPropagation();
              setIsExpanded(!isExpanded);
            }}
            className="p-0.5 -ml-1 text-muted-foreground hover:text-foreground rounded hover:bg-muted/80 transition-colors shrink-0"
          >
            {isExpanded ? (
              <ChevronDown className={isTopLevel ? 'w-3.5 h-3.5' : 'w-3 h-3'} />
            ) : (
              <ChevronRight className={isTopLevel ? 'w-3.5 h-3.5' : 'w-3 h-3'} />
            )}
          </span>
        )}

        {/* 状态图标 */}
        <StatusIcon status={report.status} size={isTopLevel ? 'sm' : 'xs'} />

        {/* 编号 */}
        <span
          className={cn(
            'font-mono text-muted-foreground shrink-0',
            isTopLevel ? 'text-[11px] w-4' : 'text-[10px] w-6'
          )}
        >
          {indexPrefix}.
        </span>

        {/* 子流程标记：让嵌套调用一眼可辨 */}
        {hasSubNodes && (
          <Workflow
            className={cn('shrink-0 text-violet-500', isTopLevel ? 'w-3 h-3' : 'w-2.5 h-2.5')}
          />
        )}

        {/* 节点名称 */}
        <span
          className={cn(
            'text-foreground truncate flex-1 min-w-0',
            isTopLevel ? 'text-xs' : 'text-[11px]'
          )}
          title={report.node_name}
        >
          {report.node_name}
        </span>

        {/* 所属阶段标记 (Setup / Teardown) */}
        {stage && stage !== 'main' && (
          <span
            className={cn(
              'shrink-0 rounded px-1 py-0.5 font-medium leading-none border',
              isTopLevel ? 'text-[9px]' : 'text-[8px]',
              stage === 'setup'
                ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20'
                : 'bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-500/20',
            )}
          >
            {stage === 'setup' ? stageLl.short_setup() : stageLl.short_teardown()}
          </span>
        )}

        {/* 类型标签 */}
        <span
          className={cn(
            'inline-flex items-center rounded font-medium leading-none shrink-0',
            isTopLevel ? 'px-1 py-0.5 text-[10px]' : 'px-1 py-0.5 text-[9px]',
            meta?.badge || 'bg-muted text-muted-foreground'
          )}
        >
          {meta?.name ?? report.node_type}
        </span>

        {/* 执行时间 */}
        <div
          className={cn(
            'flex items-center gap-1 text-muted-foreground shrink-0 font-mono',
            isTopLevel ? 'text-[11px]' : 'text-[10px]'
          )}
        >
          <Clock className={isTopLevel ? 'w-3 h-3' : 'w-2.5 h-2.5'} />
          <span>{report.duration_ms}ms</span>
        </div>
      </button>

      {/* 展开的嵌套子节点列表 */}
      {hasSubNodes && isExpanded && subNodes && (
        <div
          className={cn(
            'border-l-2 border-violet-500/20 ml-2.5 my-0.5 space-y-0.5',
            isTopLevel ? 'pl-6' : 'pl-3'
          )}
        >
          {subNodes.map((subNode, subIdx) => (
            <NodeTreeRow
              key={subNode.node_uuid}
              report={subNode}
              depth={depth + 1}
              indexPrefix={`${indexPrefix}.${subIdx + 1}`}
              isSelected={selectedNodeUuid === subNode.node_uuid}
              selectedNodeUuid={selectedNodeUuid}
              onSelect={() => onSelectNode?.(subNode.node_uuid)}
              onSelectNode={onSelectNode}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export function NodeListItem({
  report,
  isSelected,
  onClick,
  indexPrefix,
  selectedNodeUuid,
  onSelectNode,
}: NodeListItemProps) {
  return (
    <NodeTreeRow
      report={report}
      depth={0}
      indexPrefix={indexPrefix ?? '1'}
      isSelected={isSelected}
      selectedNodeUuid={selectedNodeUuid}
      onSelect={onClick}
      onSelectNode={onSelectNode}
    />
  );
}
