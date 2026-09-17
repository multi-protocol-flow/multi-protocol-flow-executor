/**
 * GenericViewer - 通用节点执行详情查看器（兜底）
 * 用于 Entry、End 及其他未实现专用 Viewer 的节点类型
 * 简洁展示输入数据、输出数据和变量变更
 */
import { useState, memo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  ArrowDownToLine,
  ArrowUpFromLine,
  Layers,
  Sparkles,
  Terminal,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { RawJsonViewer } from './RawJsonViewer';
import { VariableChangesCard } from './VariableChangesCard';
import { getLL } from '@/i18n/i18n-lazy';
import { useExecutionStore } from '@/stores';
import { CopyButton } from '@/components/common';
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
  extra?: React.ReactNode;
  children: React.ReactNode;
}

function CollapsibleSection({ title, icon, defaultOpen = false, extra, children }: CollapsibleSectionProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  return (
    <div className="border border-border rounded-lg overflow-hidden">
      <div className="flex items-center w-full bg-muted/30 hover:bg-muted/50 transition-colors">
        <button
          type="button"
          onClick={() => setIsOpen(!isOpen)}
          className="flex-1 flex items-center gap-2 p-3 text-left"
          aria-expanded={isOpen}
        >
          <div className="text-muted-foreground">
            {isOpen ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
          </div>
          <div className="text-muted-foreground">{icon}</div>
          <span className="font-medium text-foreground text-sm">{title}</span>
        </button>
        {extra && <div className="pr-3">{extra}</div>}
      </div>
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

export const GenericViewer = memo(function GenericViewer({ report }: GenericViewerProps) {
  const ll = getLL();
  const { node_name, node_type, resolved_config, output_data, variable_changes } = report;
  const nodeStreamData = useExecutionStore((s) => s?.nodeStreamMap?.[report.node_uuid]);
  const hasStreamData =
    nodeStreamData &&
    (Boolean(nodeStreamData.fullContent) ||
      Boolean(nodeStreamData.fullReasoning) ||
      nodeStreamData.chunks.length > 0);
  const isRunning = (report.status as string) === 'running';
  return (
    <div className="space-y-3">
      {hasStreamData && (
        <CollapsibleSection
          title={ll.execution.viewers.realtime_stream()}
          icon={<Sparkles className="w-4 h-4 text-purple-500 animate-pulse" />}
          defaultOpen={true}
        >
          <div className="space-y-3">
            {nodeStreamData.fullReasoning ? (
              <div className="bg-muted/40 border border-border/60 rounded-md p-3">
                <div className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-1.5 flex items-center gap-1.5">
                  <Terminal className="w-3.5 h-3.5 text-purple-500" />
                  <span>{ll.execution.viewers.thinking_process()}</span>
                </div>
                <div className="text-xs text-muted-foreground/90 font-mono whitespace-pre-wrap leading-relaxed max-h-96 overflow-y-auto [contain:content]">
                  {nodeStreamData.fullReasoning}
                  {isRunning && !nodeStreamData.fullContent && (
                    <span className="inline-block w-1.5 h-3.5 bg-purple-500 ml-1 animate-pulse align-middle" />
                  )}
                </div>
              </div>
            ) : null}
            {nodeStreamData.fullContent ? (
              <div className="bg-background border border-border rounded-md p-3">
                <div className="text-xs font-semibold text-foreground/80 mb-1.5 flex items-center justify-between">
                  <span>{ll.execution.viewers.stream_content()}</span>
                  <div className="flex items-center gap-2">
                    <span className="text-[10px] text-muted-foreground font-mono">
                      {ll.execution.viewers.chars_count({ count: nodeStreamData.fullContent.length })}
                    </span>
                    <CopyButton value={nodeStreamData.fullContent} size="xs" />
                  </div>
                </div>
                <div className="text-xs text-foreground font-sans whitespace-pre-wrap leading-relaxed select-text cursor-text">
                  {nodeStreamData.fullContent}
                  {isRunning && (
                    <span className="inline-block w-1.5 h-3.5 bg-blue-500 ml-1 animate-pulse align-middle" />
                  )}
                </div>
              </div>
            ) : null}
          </div>
        </CollapsibleSection>
      )}
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
        extra={
          resolved_config !== undefined && resolved_config !== null ? (
            <CopyButton
              value={resolved_config}
              format="json"
              label={ll.execution.viewers.copy_json()}
              title={ll.execution.viewers.copy_json()}
              size="xs"
            />
          ) : null
        }
      >
        <DataSection data={resolved_config} emptyText={ll.execution.viewers.no_input_data()} />
      </CollapsibleSection>

      <CollapsibleSection
        title={ll.execution.viewers.output_data()}
        icon={<ArrowDownToLine className="w-4 h-4" />}
        defaultOpen={true}
        extra={
          output_data !== undefined && output_data !== null ? (
            <CopyButton
              value={output_data}
              format="json"
              label={ll.execution.viewers.copy_json()}
              title={ll.execution.viewers.copy_json()}
              size="xs"
            />
          ) : null
        }
      >
        <DataSection data={output_data} emptyText={ll.execution.viewers.no_output_data()} />
      </CollapsibleSection>

      {report.plugin_data !== undefined && report.plugin_data !== null && (
        <CollapsibleSection
          title={ll.execution.viewers.plugin_data()}
          icon={<Sparkles className="w-4 h-4 text-purple-500" />}
          defaultOpen={true}
          extra={
            <CopyButton
              value={report.plugin_data}
              format="json"
              label={ll.execution.viewers.copy_json()}
              title={ll.execution.viewers.copy_json()}
              size="xs"
            />
          }
        >
          <DataSection data={report.plugin_data} />
        </CollapsibleSection>
      )}
      <VariableChangesCard changes={variable_changes} defaultOpen={true} />
    </div>
  );
});
