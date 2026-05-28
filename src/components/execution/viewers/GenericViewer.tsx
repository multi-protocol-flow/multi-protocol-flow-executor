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
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { RawJsonViewer } from './RawJsonViewer';
import { VariableChangesCard } from './VariableChangesCard';
import { getLL } from '@/i18n/i18n-lazy';

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

function DataSection({ data, emptyText }: DataSectionProps) {
  const ll = getLL();
  if (data === null || data === undefined) {
    return <p className="text-sm text-muted-foreground italic">{emptyText ?? ll.execution.viewers.no_data()}</p>;
  }

  return (
    <div className="bg-muted rounded p-2">
      <RawJsonViewer data={data} />
    </div>
  );
}

export function GenericViewer({ report }: GenericViewerProps) {
  const ll = getLL();
  const { node_name, node_type, resolved_config, output_data, variable_changes } = report;

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.execution.viewers.node_info()}
        icon={<Layers className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-start gap-2">
            <span className="text-xs font-medium text-muted-foreground shrink-0 min-w-12">{ll.execution.viewers.name_label()}</span>
            <span className="text-sm text-foreground">{node_name}</span>
          </div>
          <div className="flex items-start gap-2">
            <span className="text-xs font-medium text-muted-foreground shrink-0 min-w-12">{ll.execution.viewers.type_label()}</span>
            <code className="text-sm font-mono text-foreground bg-muted px-2 py-0.5 rounded">
              {node_type}
            </code>
          </div>
        </div>
      </CollapsibleSection>

      <CollapsibleSection
        title={ll.execution.viewers.input_data()}
        icon={<ArrowUpFromLine className="w-4 h-4" />}
        defaultOpen={true}
      >
        <DataSection data={resolved_config} emptyText={ll.execution.viewers.no_input_data()} />
      </CollapsibleSection>

      <CollapsibleSection
        title={ll.execution.viewers.output_data()}
        icon={<ArrowDownToLine className="w-4 h-4" />}
        defaultOpen={true}
      >
        <DataSection data={output_data} emptyText={ll.execution.viewers.no_output_data()} />
      </CollapsibleSection>

      <VariableChangesCard changes={variable_changes} defaultOpen={true} />
    </div>
  );
}
