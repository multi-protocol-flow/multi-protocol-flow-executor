/**
 * PluginIframeViewer — 插件节点执行结果查看器（iframe 报告页）
 *
 * 当节点元数据声明 `viewer`（报告页前端，如 grpc 插件的 viewer.html）时，
 * 渲染沙箱 iframe（sandbox 仅 `allow-scripts`，同配置面板 D2）并把
 * `node_report` 经 init postMessage 传入；插件 viewer 渲染报告并可实时
 * 消费 `stream` 消息（gRPC 流式消息/错误，从宿主 debug store 按节点
 * uuid 过滤后转发）。
 *
 * 未声明 viewer 的插件节点回退到最小实现：`plugin_data` 格式化 JSON。
 */
import { memo, useEffect, useMemo, useRef, useState } from 'react';
import type { NodeExecutionReport } from '@/types';
import { RawJsonViewer } from './RawJsonViewer';
import { getLL } from '@/i18n/i18n-lazy';
import { useDebugStore } from '@/stores/debug';
import { getCachedNodeMeta } from '@/lib/NodeRegistry';

interface PluginIframeViewerProps {
  /** 节点执行报告 */
  report: NodeExecutionReport;
}

export const PluginIframeViewer = memo(function PluginIframeViewer({ report }: PluginIframeViewerProps) {
  const ll = getLL();
  const { plugin_data } = report;

  // 节点元数据 viewer 页（插件 describe 的 `viewer` 前端，宿主归一化生成 URL）
  const viewerUrl = useMemo(
    () => getCachedNodeMeta(report.node_type)?.viewer?.url ?? null,
    [report.node_type]
  );

  if (!viewerUrl) {
    return (
      <div className="space-y-3">
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
  }

  return <PluginReportIframe report={report} src={viewerUrl} />;
});

/** 报告 iframe：init 携带 node_report，转发实时 stream 消息。 */
function PluginReportIframe({ report, src }: { report: NodeExecutionReport; src: string }) {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [ready, setReady] = useState(false);

  const grpcStreamMessages = useDebugStore((s) => s.grpcStreamMessages);
  const grpcStreamErrors = useDebugStore((s) => s.grpcStreamErrors);
  const nodeUuid = report.node_uuid;

  // 与当前节点相关的实时流消息（GrpcViewer 同款过滤：按 node_uuid）
  const realtimeMessages = useMemo(() => {
    const messages: Array<{
      call_id: string;
      index: number;
      data: string;
      received_at_ms: number;
      size_bytes: number;
      is_last: boolean;
    }> = [];
    for (const [callId, msgs] of Object.entries(grpcStreamMessages)) {
      const nodeMsgs = msgs.filter((m) => m.node_uuid === nodeUuid);
      for (const m of nodeMsgs) {
        messages.push({ ...m, call_id: callId });
      }
    }
    messages.sort((a, b) => a.index - b.index);
    return messages;
  }, [grpcStreamMessages, nodeUuid]);

  const realtimeError = useMemo(() => {
    for (const err of Object.values(grpcStreamErrors)) {
      if (err.node_uuid === nodeUuid) return err;
    }
    return null;
  }, [grpcStreamErrors, nodeUuid]);

  // init 载荷（viewer 页协议：node_report + nodeType + locale）
  useEffect(() => {
    if (!ready || !iframeRef.current) return;
    iframeRef.current.contentWindow?.postMessage(
      {
        type: 'init',
        payload: {
          config: {},
          variables: {},
          host_api: {},
          nodeType: report.node_type,
          locale: 'en-US',
          node_report: report,
        },
      },
      '*'
    );
  }, [ready, report]);

  // 实时流消息转发：`{type:"stream", payload:{call_id, kind, data}}`
  useEffect(() => {
    if (!ready || !iframeRef.current) return;
    const target = iframeRef.current.contentWindow;
    for (const m of realtimeMessages) {
      let data: unknown = null;
      try {
        data = JSON.parse(m.data);
      } catch {
        data = m.data;
      }
      target?.postMessage(
        { type: 'stream', payload: { call_id: m.call_id, kind: 'message', data } },
        '*'
      );
    }
    if (realtimeError) {
      target?.postMessage(
        {
          type: 'stream',
          payload: { call_id: realtimeError.call_id, kind: 'error', data: { message: realtimeError.message } },
        },
        '*'
      );
    }
  }, [realtimeMessages, realtimeError, ready]);

  return (
    <div className="border border-border rounded-lg overflow-hidden">
      <iframe
        ref={iframeRef}
        src={src}
        title="Plugin report viewer"
        sandbox="allow-scripts"
        onLoad={() => setReady(true)}
        className="w-full min-h-[320px] bg-background"
      />
    </div>
  );
}
