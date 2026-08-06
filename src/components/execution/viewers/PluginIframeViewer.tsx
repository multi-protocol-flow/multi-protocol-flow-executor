/**
 * PluginIframeViewer — 插件节点执行结果查看器（最小实现）
 *
 * 渲染 `report.plugin_data`（插件执行载荷）为格式化 JSON，
 * 并预留 iframe 位：插件自定义结果面板（iframe）将在此渲染（TODO: 后续 PR 接入）。
 */
import { memo } from 'react';
import type { NodeExecutionReport } from '@/types';
import { RawJsonViewer } from './RawJsonViewer';
import { getLL } from '@/i18n/i18n-lazy';

interface PluginIframeViewerProps {
  /** 节点执行报告 */
  report: NodeExecutionReport;
}

export const PluginIframeViewer = memo(function PluginIframeViewer({ report }: PluginIframeViewerProps) {
  const ll = getLL();
  const { plugin_data } = report;

  return (
    <div className="space-y-3">
      {/* 预留 iframe 位：插件自定义结果面板将在此渲染（TODO: 后续 PR 接入） */}
      <div className="w-full rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-6 text-center">
        <p className="text-xs text-muted-foreground">
          {ll.execution.viewers.plugin_viewer_placeholder()}
        </p>
      </div>
      <div className="border border-border rounded-lg overflow-hidden">
        <div className="p-3 bg-muted/30 text-sm font-medium text-foreground">
          {ll.execution.viewers.plugin_data()}
        </div>
        <div className="p-3 border-t border-border bg-background">
          {plugin_data === undefined ? (
            <p className="text-sm text-muted-foreground italic">{ll.execution.viewers.no_data()}</p>
          ) : (
            <div className="bg-muted rounded p-2">
              <RawJsonViewer data={plugin_data} />
            </div>
          )}
        </div>
      </div>
    </div>
  );
});
