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
import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  ArrowDownToLine,
  ArrowUpFromLine,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { GenericViewer } from './GenericViewer';
import { RawJsonViewer } from './RawJsonViewer';
import { VariableChangesCard } from './VariableChangesCard';
import { useDebugStore } from '@/stores/debug';
import { useExecutionStore } from '@/stores';
import { useCurrentLocale } from '@/hooks/useLocale';
import { getCachedNodeMeta } from '@/lib/NodeRegistry';
import { getLL } from '@/i18n/i18n-lazy';
import { createPluginRequestHandler } from '@/lib/pluginRequestActions';
import { CopyButton } from '@/components/common';
import { copyToClipboard } from '@/lib/clipboard';
import { toast } from 'sonner';
interface PluginIframeViewerProps {
  /** 节点执行报告 */
  report: NodeExecutionReport;
}

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
      <div className="flex items-center justify-between p-3 bg-muted/30 hover:bg-muted/50 transition-colors">
        <button
          type="button"
          onClick={() => setIsOpen(!isOpen)}
          className="flex-1 flex items-center gap-2 text-left"
        >
          <div className="text-muted-foreground">{icon}</div>
          <span className="text-sm font-medium text-foreground">{title}</span>
        </button>
        <div className="flex items-center gap-2">
          {extra}
          <button
            type="button"
            onClick={() => setIsOpen(!isOpen)}
            className="text-muted-foreground hover:text-foreground"
            aria-label={isOpen ? "Collapse" : "Expand"}
          >
            {isOpen ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
          </button>
        </div>
      </div>
      {isOpen && (
        <div className="p-3 border-t border-border bg-background">
          {children}
        </div>
      )}
    </div>
  );
}

export const PluginIframeViewer = memo(function PluginIframeViewer({ report }: PluginIframeViewerProps) {
  const ll = getLL();
  const { resolved_config, input_data, output_data, variable_changes } = report;
  const inputPayload = resolved_config ?? input_data;

  // 节点元数据 viewer 页（插件 describe 的 `viewer` 前端，宿主归一化生成 URL）
  const viewerUrl = useMemo(
    () => getCachedNodeMeta(report.node_type)?.viewer?.url ?? null,
    [report.node_type]
  );

  if (!viewerUrl) {
    return <GenericViewer report={report} />;
  }

  return (
    <div className="space-y-3">
      {/* 1. 插件专属视图 */}
      <PluginReportIframe key={`${report.node_uuid}-${viewerUrl}`} report={report} src={viewerUrl} />

      {/* 2. 输入数据（折叠展示，保持全节点一致性） */}
      {inputPayload !== undefined && (
        <CollapsibleSection
          title={ll.execution.viewers.input_data()}
          icon={<ArrowUpFromLine className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <CopyButton
              value={inputPayload}
              format="json"
              label={ll.execution.viewers.copy_json()}
              title={ll.execution.viewers.copy_json()}
              size="xs"
            />
          }
        >
          <div className="bg-muted rounded p-2">
            <RawJsonViewer data={inputPayload} />
          </div>
        </CollapsibleSection>
      )}

      {/* 3. 输出数据（折叠展示） */}
      {output_data !== undefined && output_data !== null && (
        <CollapsibleSection
          title={ll.execution.viewers.output_data()}
          icon={<ArrowDownToLine className="w-4 h-4" />}
          defaultOpen={true}
          extra={
            <CopyButton
              value={output_data}
              format="json"
              label={ll.execution.viewers.copy_json()}
              title={ll.execution.viewers.copy_json()}
              size="xs"
            />
          }
        >
          <div className="bg-muted rounded p-2">
            <RawJsonViewer data={output_data} />
          </div>
        </CollapsibleSection>
      )}
      {/* 4. 变量变更 */}
      <VariableChangesCard changes={variable_changes} defaultOpen={true} />
    </div>
  );
});

/** 报告 iframe：init 携带 node_report，转发实时 stream 消息。 */
function PluginReportIframe({ report, src }: { report: NodeExecutionReport; src: string }) {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [ready, setReady] = useState(false);
  const [iframeHeight, setIframeHeight] = useState<number>(360);
  const iframeReadyRef = useRef(false);
  const locale = useCurrentLocale();
  const requestHandler = useMemo(
    () => createPluginRequestHandler(report.plugin_data || report.output_data, report.node_type),
    [report.plugin_data, report.output_data, report.node_type]
  );
  const grpcStreamMessages = useDebugStore((s) => s.grpcStreamMessages);
  const grpcStreamErrors = useDebugStore((s) => s.grpcStreamErrors);
  const nodeUuid = report.node_uuid;
  const nodeStreamData = useExecutionStore((s) => s?.nodeStreamMap?.[nodeUuid]);
  const currentExecutionId = useExecutionStore((s) => s?.report?.uuid);
  const lastSentChunkIndexRef = useRef(0);
  const lastSentGrpcIndexRef = useRef(0);
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

  const sendInit = useCallback(() => {
    if (!iframeRef.current?.contentWindow) return;
    const execId =
      (report as unknown as { execution_id?: string }).execution_id ||
      currentExecutionId ||
      report.node_uuid;
    iframeRef.current.contentWindow.postMessage(
      {
        type: 'init',
        payload: {
          config: {},
          variables: {},
          host_api: {},
          nodeType: report.node_type,
          locale,
          node_report: {
            ...report,
            execution_id: execId,
          },
          outputData: report.output_data,
          reportData: report.plugin_data || report.output_data,
        },
      },
      '*'
    );
  }, [report, locale, currentExecutionId]);

  // 监听来自 iframe 的握手消息 (ready / requestInit / resize)
  useEffect(() => {
    const handleWindowMessage = (event: MessageEvent) => {
      if (event.source !== iframeRef.current?.contentWindow) return;
      const data = event.data;
      if (!data || typeof data !== 'object') return;
      if (data.type === 'ready' || data.type === 'requestInit') {
        iframeReadyRef.current = true;
        setReady(true);
        sendInit();
      } else if (data.type === 'resize' && data.payload && typeof data.payload === 'object') {
        const reportedHeight = (data.payload as { height?: unknown }).height;
        if (typeof reportedHeight === 'number' && Number.isFinite(reportedHeight) && reportedHeight > 0) {
          setIframeHeight((prev) => {
            if (Math.abs(prev - reportedHeight) >= 4) {
              return reportedHeight;
            }
            return prev;
          });
        }
      } else if (data.type === 'requestAction' && data.payload && typeof data.payload === 'object') {
        const { requestId, action, params } = data.payload as {
          requestId: string;
          action: string;
          params?: unknown;
        };
        requestHandler(action, params)
          .then((result) => {
            iframeRef.current?.contentWindow?.postMessage(
              { type: 'actionResult', payload: { requestId, success: true, result } },
              '*'
            );
          })
          .catch((err) => {
            iframeRef.current?.contentWindow?.postMessage(
              {
                type: 'actionResult',
                payload: {
                  requestId,
                  success: false,
                  error: err instanceof Error ? err.message : String(err),
                },
              },
              '*'
            );
          });
      } else if (data.type === 'copy' && data.payload && typeof data.payload === 'object') {
        const copyText = (data.payload as { text?: string }).text;
        if (copyText) {
          copyToClipboard(copyText);
          toast.success(getLL().execution.report.copied_to_clipboard());
        }
      }
    };
    window.addEventListener('message', handleWindowMessage);
    return () => window.removeEventListener('message', handleWindowMessage);
  }, [sendInit]);

  // 初始加载及重试机制：解决 WebKit/Chromium 中 onLoad 触发早于 iframe 内部 JS 监听器绑定的竞态
  useEffect(() => {
    iframeReadyRef.current = false;
    sendInit();

    let retryCount = 0;
    const maxRetries = 20; // 2 秒内重试
    const timer = window.setInterval(() => {
      if (iframeReadyRef.current) {
        window.clearInterval(timer);
        return;
      }
      retryCount++;
      if (retryCount >= maxRetries) {
        window.clearInterval(timer);
        return;
      }
      sendInit();
    }, 100);

    return () => window.clearInterval(timer);
  }, [sendInit]);

  // 重置 chunk 索引（当 nodeUuid 变更时）
  useEffect(() => {
    lastSentChunkIndexRef.current = 0;
    lastSentGrpcIndexRef.current = 0;
  }, [nodeUuid]);
  // 通用插件流消息转发：`{type:"stream", payload:{...}}`
  useEffect(() => {
    if (!ready || !iframeRef.current || !nodeStreamData) return;
    const target = iframeRef.current.contentWindow;
    if (!target) return;
    const chunks = nodeStreamData.chunks;
    const startIdx = lastSentChunkIndexRef.current;
    for (let i = startIdx; i < chunks.length; i++) {
      const chunk = chunks[i];
      const rawData =
        typeof chunk.raw === 'object' && chunk.raw !== null
          ? (chunk.raw as Record<string, unknown>)
          : {};
      target.postMessage(
        {
          type: 'stream',
          payload: {
            ...rawData,
            kind: chunk.kind,
            delta_content: chunk.delta_content,
            delta_reasoning: chunk.delta_reasoning,
            text: chunk.text,
            fullContent: nodeStreamData.fullContent,
            fullReasoning: nodeStreamData.fullReasoning,
            timestamp: chunk.timestamp,
          },
        },
        '*'
      );
    }
    lastSentChunkIndexRef.current = chunks.length;
  }, [ready, nodeStreamData]);

  // 实时流消息转发：`{type:"stream", payload:{call_id, kind, data}}`
  useEffect(() => {
    if (!ready || !iframeRef.current) return;
    const target = iframeRef.current.contentWindow;
    if (!target) return;
    const startIdx = lastSentGrpcIndexRef.current;
    for (let i = startIdx; i < realtimeMessages.length; i++) {
      const m = realtimeMessages[i];
      let data: unknown = null;
      try {
        data = JSON.parse(m.data);
      } catch {
        data = m.data;
      }
      target.postMessage(
        { type: 'stream', payload: { call_id: m.call_id, kind: 'message', data } },
        '*'
      );
    }
    lastSentGrpcIndexRef.current = realtimeMessages.length;
    if (realtimeError) {
      target.postMessage(
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
        style={{ height: iframeHeight }}
        className="w-full min-h-[320px] bg-background block transition-[height] duration-150"
      />
    </div>
  );
}
