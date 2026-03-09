/**
 * GenericViewer - 通用节点执行详情查看器（兜底）
 * 用于 Entry、End 及其他未实现专用 Viewer 的节点类型
 * 简洁展示输入数据、输出数据和变量变更
 */
import { useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  ArrowDownToLine,
  ArrowUpFromLine,
  Layers,
  Variable,
} from 'lucide-react';
import type { NodeExecutionReport, VariableChange } from '@/types';
import { RawJsonViewer } from './RawJsonViewer';
import { cn } from '@/lib/utils';

interface GenericViewerProps {
  /** 节点执行报告 */
  report: NodeExecutionReport;
}

/**
 * 折叠面板组件
 */
interface CollapsibleSectionProps {
  title: string;
  icon: React.ReactNode;
  defaultOpen?: boolean;
  children: React.ReactNode;
}

function CollapsibleSection({ title, icon, defaultOpen = false, children }: CollapsibleSectionProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  return (
    <div className="border border-border rounded-lg overflow-hidden">
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className="w-full flex items-center gap-2 p-3 bg-muted/30 hover:bg-muted/50 transition-colors text-left"
        aria-expanded={isOpen}
      >
        <div className="text-muted-foreground">
          {isOpen ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
        </div>
        <div className="text-muted-foreground">{icon}</div>
        <span className="font-medium text-foreground text-sm">{title}</span>
      </button>
      {isOpen && <div className="p-3 border-t border-border bg-background">{children}</div>}
    </div>
  );
}

/**
 * 数据展示区域
 */
interface DataSectionProps {
  data: unknown;
  emptyText?: string;
}

function DataSection({ data, emptyText = '无数据' }: DataSectionProps) {
  if (data === null || data === undefined) {
    return <p className="text-sm text-muted-foreground italic">{emptyText}</p>;
  }

  return (
    <div className="bg-slate-50 rounded p-2">
      <RawJsonViewer data={data} />
    </div>
  );
}

/**
 * 单个变量变更展示
 */
interface VariableChangeItemProps {
  change: VariableChange;
}

function VariableChangeItem({ change }: VariableChangeItemProps) {
  const { key, old_value, new_value } = change;

  // 格式化值用于展示
  const formatValue = (value: unknown): string => {
    if (value === null || value === undefined) {
      return 'null';
    }
    if (typeof value === 'object') {
      return JSON.stringify(value);
    }
    return String(value);
  };

  // 判断是否为新创建的变量（没有旧值）
  const isNewVariable = old_value === undefined || old_value === null;

  return (
    <div className="border-b border-border last:border-b-0 py-2 last:pb-0 first:pt-0">
      <div className="flex items-start gap-2">
        {/* 变量名 */}
        <span className="text-sm font-mono font-semibold text-slate-700 min-w-0 shrink-0">
          {key}
        </span>
        <span className="text-muted-foreground">=</span>
      </div>
      
      <div className="mt-1.5 space-y-1.5 pl-2">
        {/* 旧值（如果不是新变量） */}
        {!isNewVariable && (
          <div className="flex items-start gap-2">
            <span className="text-xs text-red-600 font-mono shrink-0">-</span>
            <div className="flex-1 bg-red-50 rounded px-2 py-1 border-l-2 border-red-400">
              <code className="text-sm font-mono text-red-700 line-through break-all">
                {formatValue(old_value)}
              </code>
            </div>
          </div>
        )}
        
        {/* 新值 */}
        <div className="flex items-start gap-2">
          <span className="text-xs text-green-600 font-mono shrink-0">+</span>
          <div className={cn(
            "flex-1 rounded px-2 py-1 border-l-2",
            isNewVariable ? "bg-blue-50 border-blue-400" : "bg-green-50 border-green-400"
          )}>
            <code className={cn(
              "text-sm font-mono break-all",
              isNewVariable ? "text-blue-700" : "text-green-700"
            )}>
              {formatValue(new_value)}
            </code>
          </div>
        </div>
      </div>
    </div>
  );
}

/**
 * 变量变更列表
 */
interface VariableChangesProps {
  changes: VariableChange[];
}

function VariableChanges({ changes }: VariableChangesProps) {
  if (!changes || changes.length === 0) {
    return <p className="text-sm text-muted-foreground italic">无变量变更</p>;
  }

  return (
    <div className="space-y-0">
      {changes.map((change, index) => (
        <VariableChangeItem key={`${change.key}-${index}`} change={change} />
      ))}
    </div>
  );
}

export function GenericViewer({ report }: GenericViewerProps) {
  const { node_name, node_type, input_data, output_data, variable_changes } = report;

  return (
    <div className="space-y-3">
      {/* 节点信息 */}
      <CollapsibleSection
        title="节点信息"
        icon={<Layers className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-start gap-2">
            <span className="text-xs font-medium text-muted-foreground shrink-0 min-w-12">名称</span>
            <span className="text-sm text-foreground">{node_name}</span>
          </div>
          <div className="flex items-start gap-2">
            <span className="text-xs font-medium text-muted-foreground shrink-0 min-w-12">类型</span>
            <code className="text-sm font-mono text-slate-700 bg-slate-100 px-2 py-0.5 rounded">
              {node_type}
            </code>
          </div>
        </div>
      </CollapsibleSection>

      {/* 输入数据 */}
      <CollapsibleSection
        title="输入数据"
        icon={<ArrowUpFromLine className="w-4 h-4" />}
        defaultOpen={true}
      >
        <DataSection data={input_data} emptyText="无输入数据" />
      </CollapsibleSection>

      {/* 输出数据 */}
      <CollapsibleSection
        title="输出数据"
        icon={<ArrowDownToLine className="w-4 h-4" />}
        defaultOpen={true}
      >
        <DataSection data={output_data} emptyText="无输出数据" />
      </CollapsibleSection>

      {/* 变量变更 */}
      {variable_changes && variable_changes.length > 0 && (
        <CollapsibleSection
          title={`变量变更 (${variable_changes.length})`}
          icon={<Variable className="w-4 h-4" />}
          defaultOpen={true}
        >
          <VariableChanges changes={variable_changes} />
        </CollapsibleSection>
      )}
    </div>
  );
}
