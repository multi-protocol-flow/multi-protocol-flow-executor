/**
 * GrpcViewer - gRPC 节点执行详情查看器
 * 结构化展示 gRPC Connect/Call/Close 三种节点的执行结果
 * Connect: 连接配置、服务发现、耗时
 * Call: 调用配置、响应状态、响应数据、耗时、实时流式消息
 * Close: 关闭配置、耗时
 */
import { useState, useMemo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Cable,
  Radio,
  Unplug,
  Clock,
  Globe,
  Server,
  AlertTriangle,
  Activity,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { cn } from '@/lib/utils';
import { RawJsonViewer } from './RawJsonViewer';
import { VariableChangesCard } from './VariableChangesCard';
import { getLL } from '@/i18n/i18n-lazy';
import { useDebugStore } from '@/stores/debug';

interface GrpcViewerProps {
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
        {extra && <div className="ml-auto">{extra}</div>}
      </button>
      {isOpen && <div className="p-3 border-t border-border bg-background">{children}</div>}
    </div>
  );
}

/**
 * Key-Value 列表组件
 */
interface KeyValueListProps {
  data: Record<string, unknown> | undefined | null;
}

function KeyValueList({ data }: KeyValueListProps) {
  if (!data || Object.keys(data).length === 0) {
    return <p className="text-sm text-muted-foreground italic">{getLL().viewer.common.no_data()}</p>;
  }

  return (
    <div className="space-y-1.5">
      {Object.entries(data).map(([key, value]) => (
        <div key={key} className="flex items-start gap-2 text-sm">
          <span className="font-mono text-muted-foreground min-w-0 shrink-0">{key}:</span>
          <span className="text-foreground break-all">
            {typeof value === 'object' ? JSON.stringify(value) : String(value)}
          </span>
        </div>
      ))}
    </div>
  );
}

// ============================================================================
// 工具函数
// ============================================================================

/**
 * 格式化字节数为人类可读字符串
 */
function formatSizeBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

// ============================================================================
// gRPC 输出数据类型
// ============================================================================

/**
 * gRPC Connect 输出数据
 */
interface GrpcConnectOutput {
  success?: boolean;
  connection_id?: string | null;
  url?: string;
  request?: {
    url?: string;
    use_tls?: boolean;
    tls_skip_verify?: boolean;
    enable_reflection?: boolean;
    proto_files_count?: number;
    default_metadata_count?: number;
    connect_timeout_ms?: number;
  };
  timing?: {
    connect_ms?: number;
    total_ms?: number;
  };
  services?: Array<{
    service_name: string;
    methods: Array<{
      method_name: string;
      input_type: string;
      output_type: string;
      is_server_streaming: boolean;
      is_client_streaming?: boolean;
    }>;
  }>;
  service_count?: number;
  error?: string;
}

/**
 * gRPC Call 输出数据（Unary 和 Server Streaming 共用）
 */
interface GrpcCallOutput {
  success?: boolean;
  /** Streaming pattern: 'unary' | 'server_streaming' | 'client_streaming' | 'bidi_streaming' */
  pattern?: string;
  connection_id?: string;
  service_name?: string;
  method_name?: string;
  request?: {
    connection_id?: string;
    service_name?: string;
    method_name?: string;
    request_json?: string;
    timeout_ms?: number;
    metadata_count?: number;
  };
  timing?: {
    call_ms?: number;
    total_ms?: number;
  };
  /** Response data (Unary/Client Streaming: single object) */
  data?: unknown;
  /** Responses array (Server/Bidi Streaming: multiple responses) */
  responses?: Array<{ index?: number; data?: unknown; received_at_ms?: number; size_bytes?: number }>;
  /** Number of messages sent (Client/Bidi Streaming) */
  sent_count?: number;
  /** Number of messages received (Server/Bidi Streaming) */
  received_count?: number;
  /** gRPC status */
  status?: string;
  /** gRPC status message */
  status_message?: string;
  /** Response metadata */
  response_metadata?: Record<string, unknown>;
  /** Response trailers */
  trailers?: Record<string, unknown>;
  /** Whether server streaming response */
  is_streaming?: boolean;
  /** Streaming timing info */
  streaming_timing?: {
    total_ms?: number;
    first_response_ms?: number;
  };
  error?: string;
  /** 结构化 gRPC 错误详情（来自 google.rpc.Status 的 google.protobuf.Any） */
  error_details?: GrpcErrorDetail[];
}

/**
 * gRPC 错误详情（对应后端 GrpcErrorDetail 类型）
 */
interface GrpcErrorDetail {
  /** 错误详情类型 URL */
  type_url: string;
  /** 解码后的 JSON 数据 */
  decoded?: Record<string, unknown> | null;
  /** 原始十六进制数据 */
  raw_hex?: string | null;
}

/**
 * gRPC Close 输出数据
 */
interface GrpcCloseOutput {
  success?: boolean;
  connection_id?: string;
  request?: {
    connection_id?: string;
  };
  timing?: {
    close_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/**
 * 判断 Call 响应数据是否为 Server Streaming（数组形式）
 */
function isStreamingData(data: unknown): boolean {
  return Array.isArray(data);
}

export function GrpcViewer({ report }: GrpcViewerProps) {
  const nodeType = report.node_type;
  const ll = getLL();

  // 根据 node_type 分发到对应的子组件
  switch (nodeType) {
    case 'grpc_connect':
      return <GrpcConnectSection report={report} />;
    case 'grpc_call':
      return <GrpcCallSection report={report} />;
    case 'grpc_close':
      return <GrpcCloseSection report={report} />;
    default:
      return (
        <div className="p-4 text-sm text-muted-foreground italic">
          {ll.viewer.grpc.unknown_node_type({ type: nodeType })}
        </div>
      );
  }
}

// ============================================================================
// gRPC Connect 查看器
// ============================================================================

function GrpcConnectSection({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as GrpcConnectOutput | undefined;
  const resolvedConfig = report.resolved_config as GrpcConnectOutput['request'] | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  // 配置数据：优先 resolved_config，否则 output.request
  const requestConfig = output?.request || resolvedConfig;
  const configUrl = requestConfig?.url || output?.url || '';
  const connectDuration = output?.timing?.connect_ms;
  const totalDuration = output?.timing?.total_ms;
  const services = output?.services;
  const serviceCount = output?.service_count ?? services?.length ?? 0;

  return (
    <div className="space-y-3">
      {/* 连接配置 */}
      <CollapsibleSection
        title={ll.execution.viewers.connection_config()}
        icon={<Cable className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* URL */}
          <div className="flex items-center gap-2">
            <Badge
              variant="outline"
              className="font-mono text-xs text-violet-600 dark:text-violet-400 bg-violet-50 dark:bg-violet-950/50 border-0 shrink-0"
            >
              gRPC
            </Badge>
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {configUrl || <span className="text-muted-foreground italic">{ll.execution.viewers.config_url()}</span>}
            </code>
          </div>

          {/* TLS / Reflection 信息 */}
          {requestConfig && (
            <div className="flex flex-wrap gap-2">
              {requestConfig.use_tls !== undefined && (
                <Badge variant="outline" className="font-mono text-xs">
                  TLS: {requestConfig.use_tls ? ll.execution.viewers.tls_enabled() : ll.execution.viewers.tls_disabled()}
                </Badge>
              )}
              {requestConfig.tls_skip_verify && (
                <Badge variant="outline" className="font-mono text-xs text-amber-600 dark:text-amber-400">
                  {ll.execution.viewers.skip_cert_verify()}
                </Badge>
              )}
              {requestConfig.enable_reflection !== undefined && (
                <Badge variant="outline" className="font-mono text-xs">
                  Reflection: {requestConfig.enable_reflection ? ll.execution.viewers.reflection_enabled() : ll.execution.viewers.reflection_disabled()}
                </Badge>
              )}
              {requestConfig.proto_files_count !== undefined && requestConfig.proto_files_count > 0 && (
                <Badge variant="outline" className="font-mono text-xs">
                  {ll.execution.viewers.proto_files()}: {requestConfig.proto_files_count}
                </Badge>
              )}
              {requestConfig.connect_timeout_ms !== undefined && (
                <Badge variant="secondary" className="font-mono text-xs">
                  {ll.execution.viewers.timeout_label()}: {requestConfig.connect_timeout_ms}ms
                </Badge>
              )}
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 连接状态 */}
      <CollapsibleSection
        title={ll.execution.viewers.ws_connect_status()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <Badge
              className={cn(
                'font-mono text-xs px-2 py-0.5',
                isSuccess
                  ? 'text-emerald-600 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/50'
                  : 'text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/50'
              )}
            >
              {isSuccess ? ll.execution.viewers.connected() : ll.execution.viewers.disconnected()}
            </Badge>
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.execution.viewers.services_discovered({ count: serviceCount })
                : output?.error ?? ll.execution.viewers.error_message()}
            </span>
          </div>

          {/* 连接 ID */}
          {output?.connection_id && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{ll.execution.viewers.connection_id_label()}:</span>
              <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                {output.connection_id}
              </code>
            </div>
          )}

          {/* 错误信息 */}
          {!isSuccess && output?.error && (
            <div className="bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800 rounded p-2">
              <p className="text-sm text-red-700 dark:text-red-300 break-all">{output.error}</p>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 服务发现 */}
      {isSuccess && services && services.length > 0 && (
        <CollapsibleSection
          title={ll.viewer.grpc.section_services()}
          icon={<Server className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <Badge variant="outline" className="font-mono text-xs">
              {ll.viewer.grpc.services_count({ count: serviceCount })}
            </Badge>
          }
        >
          <div className="space-y-3">
            {services.map((service) => (
              <div key={service.service_name} className="border border-border rounded-lg p-2.5">
                <div className="font-mono text-sm font-medium text-foreground mb-1.5">
                  {service.service_name}
                </div>
                {service.methods.length > 0 ? (
                  <div className="space-y-1">
                    {service.methods.map((method) => (
                      <div
                        key={method.method_name}
                        className="flex items-center gap-2 text-xs bg-muted rounded px-2 py-1"
                      >
                        <code className="text-foreground font-mono">{method.method_name}</code>
                        <span className="text-muted-foreground">→</span>
                        <code className="text-muted-foreground font-mono">{method.output_type}</code>
                        {method.is_server_streaming && method.is_client_streaming ? (
                          <Badge
                            variant="outline"
                            className="text-[10px] px-1 py-0 text-purple-600 dark:text-purple-400 border-purple-300 dark:border-purple-700"
                          >
                            {ll.execution.viewers.bidi_streaming()}
                          </Badge>
                        ) : method.is_server_streaming ? (
                          <Badge
                            variant="outline"
                            className="text-[10px] px-1 py-0 text-amber-600 dark:text-amber-400 border-amber-300 dark:border-amber-700"
                          >
                            {ll.execution.viewers.server_streaming()}
                          </Badge>
                        ) : method.is_client_streaming ? (
                          <Badge
                            variant="outline"
                            className="text-[10px] px-1 py-0 text-blue-600 dark:text-blue-400 border-blue-300 dark:border-blue-700"
                          >
                            {ll.execution.viewers.client_streaming()}
                          </Badge>
                        ) : (
                          <Badge
                            variant="outline"
                            className="text-[10px] px-1 py-0 text-green-600 dark:text-green-400 border-green-300 dark:border-green-700"
                          >
                            Unary
                          </Badge>
                        )}
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-xs text-muted-foreground italic">{ll.execution.viewers.no_methods()}</p>
                )}
              </div>
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(connectDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title={ll.execution.viewers.timing_breakdown_short()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-violet-600 font-medium">
              {ll.execution.viewers.total_label()} {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            {connectDuration !== undefined && (
              <div className="flex justify-between items-center p-2 bg-muted rounded">
                <span className="text-muted-foreground">连接耗时</span>
                <span className="font-mono text-foreground">{connectDuration}ms</span>
              </div>
            )}
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// gRPC 实时流式消息项
// ============================================================================

interface GrpcRealtimeMessageItemProps {
  message: {
    call_id: string;
    index: number;
    data: string;
    received_at_ms: number;
    size_bytes: number;
    is_last: boolean;
  };
}

function GrpcRealtimeMessageItem({ message }: GrpcRealtimeMessageItemProps) {
  const [isOpen, setIsOpen] = useState(false);

  // 解析 JSON 数据用于格式化显示
  let parsedData: unknown;
  try {
    parsedData = JSON.parse(message.data);
  } catch {
    parsedData = message.data;
  }

  return (
    <div className="border border-border rounded p-2">
      <div className="flex items-center gap-2 mb-1">
        <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono">
          #{message.index}
        </Badge>
        <span className="text-[10px] text-muted-foreground font-mono">
          +{message.received_at_ms}ms
        </span>
        <span className="text-[10px] text-muted-foreground font-mono">
          {formatSizeBytes(message.size_bytes)}
        </span>
        {message.is_last && (
          <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono text-amber-600 dark:text-amber-400 border-amber-300 dark:border-amber-700">
            最后一条
          </Badge>
        )}
        <button
          type="button"
          onClick={() => setIsOpen(!isOpen)}
          className="ml-auto text-muted-foreground hover:text-foreground"
        >
          {isOpen ? <ChevronDown className="w-3 h-3" /> : <ChevronRight className="w-3 h-3" />}
        </button>
      </div>
      {isOpen && (
        <div className="bg-muted rounded p-1.5 mt-1">
          <RawJsonViewer data={parsedData} />
        </div>
      )}
    </div>
  );
}

// ============================================================================
// gRPC Call 查看器
// ============================================================================

/**
 * 从 type_url 中提取短类型名
 * `type.googleapis.com/google.rpc.RetryInfo` → `RetryInfo`
 */
function formatTypeUrl(typeUrl: string): { shortName: string; fullName: string } {
  const fullName = typeUrl.split('/').pop() ?? typeUrl;
  const parts = fullName.split('.');
  const shortName = parts.length > 0 ? parts[parts.length - 1] : fullName;
  return { shortName, fullName };
}

/**
 * gRPC 错误详情卡片
 */
function GrpcErrorDetailsCard({ details }: { details: GrpcErrorDetail[] }) {
  const ll = getLL();
  if (details.length === 0) return null;

  return (
    <CollapsibleSection
      title={`${ll.viewer.grpc.section_errors()} (${details.length})`}
      icon={<AlertTriangle className="w-4 h-4" />}
      defaultOpen={true}
    >
      <div className="space-y-2">
        {details.map((detail, idx) => {
          const { shortName, fullName } = formatTypeUrl(detail.type_url);
          return (
            <div key={idx} className="border border-border rounded-lg overflow-hidden">
              {/* 类型标题栏 */}
              <div className="flex items-center gap-2 px-3 py-2 bg-amber-50 dark:bg-amber-950/30">
                <Badge variant="outline" className="font-mono text-[10px] px-1 py-0 text-amber-700 dark:text-amber-300 border-amber-300 dark:border-amber-700">
                  {shortName}
                </Badge>
                <code className="text-xs text-muted-foreground font-mono truncate">{fullName}</code>
              </div>
              {/* 解码内容或原始十六进制 */}
              <div className="p-2">
                {detail.decoded ? (
                  <div className="bg-muted rounded p-2">
                    <RawJsonViewer data={detail.decoded} />
                  </div>
                ) : detail.raw_hex ? (
                  <div>
                    <div className="text-xs text-muted-foreground mb-1">{ll.viewer.common.label_raw_data()}</div>
                    <code className="text-xs font-mono text-foreground break-all bg-muted rounded p-2 block">
                      {detail.raw_hex.length > 512
                        ? `${detail.raw_hex.slice(0, 512)}...`
                        : detail.raw_hex}
                    </code>
                  </div>
                ) : (
                  <p className="text-xs text-muted-foreground italic">空消息体</p>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </CollapsibleSection>
  );
}

function GrpcCallSection({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as GrpcCallOutput | undefined;
  const resolvedConfig = report.resolved_config as GrpcCallOutput['request'] | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  // 配置数据
  const requestConfig = output?.request || resolvedConfig;
  const serviceName = output?.service_name || requestConfig?.service_name || '';
  const methodName = output?.method_name || requestConfig?.method_name || '';
  const callDuration = output?.timing?.call_ms;
  const totalDuration = output?.timing?.total_ms;
  const responseData = output?.data;
  const responseMetadata = output?.response_metadata;
  const trailers = output?.trailers;
  const isStreaming = output?.is_streaming ?? isStreamingData(responseData);

  // Streaming responses (new unified format)
  const streamingResponses = output?.responses ?? [];
  const hasStreamingResponses = streamingResponses.length > 0;

  // 实时流式消息：从 debug store 获取与当前节点相关的消息
  const nodeUuid = report.node_uuid;
  const grpcStreamMessages = useDebugStore((s) => s.grpcStreamMessages);
  const grpcStreamErrors = useDebugStore((s) => s.grpcStreamErrors);

  // 找到与当前节点相关的所有 call_id 的实时消息
  const realtimeMessages = useMemo(() => {
    const messages: Array<{ call_id: string; index: number; data: string; received_at_ms: number; size_bytes: number; is_last: boolean }> = [];
    for (const [callId, msgs] of Object.entries(grpcStreamMessages)) {
      const nodeMsgs = msgs.filter((m) => m.node_uuid === nodeUuid);
      if (nodeMsgs.length > 0) {
        for (const m of nodeMsgs) {
          messages.push({ ...m, call_id: callId });
        }
      }
    }
    // 按 index 排序
    messages.sort((a, b) => a.index - b.index);
    return messages;
  }, [grpcStreamMessages, nodeUuid]);

  // 实时错误
  const realtimeError = useMemo(() => {
    for (const err of Object.values(grpcStreamErrors)) {
      if (err.node_uuid === nodeUuid) return err;
    }
    return null;
  }, [grpcStreamErrors, nodeUuid]);

  // 是否显示实时流式消息面板（仅在调试模式、有实时消息、且非最终结果时）
  const showRealtimeMessages = realtimeMessages.length > 0;
  const hasFinalResults = hasStreamingResponses; // 最终结果已收集

  // Determine pattern label and color
  const patternLabel = (() => {
    switch (output?.pattern) {
      case 'unary': return { text: ll.viewer.grpc.unary_call(), color: 'text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-950/50' };
      case 'server_streaming': return { text: ll.viewer.grpc.server_streaming(), color: 'text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-950/50' };
      case 'client_streaming': return { text: ll.viewer.grpc.client_streaming(), color: 'text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-950/50' };
      case 'bidi_streaming': return { text: ll.viewer.grpc.bidi_streaming(), color: 'text-purple-600 dark:text-purple-400 bg-purple-50 dark:bg-purple-950/50' };
      default: return isStreaming
        ? { text: ll.viewer.grpc.server_streaming(), color: 'text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-950/50' }
        : { text: ll.viewer.grpc.unary_call(), color: 'text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-950/50' };
    }
  })();

  return (
    <div className="space-y-3">
      {/* 调用配置 */}
      <CollapsibleSection
        title={ll.viewer.grpc.section_call_config()}
        icon={<Radio className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* 服务名.方法名 */}
          <div className="flex items-center gap-2">
            <Badge variant="outline" className={cn("font-mono text-xs border-0 shrink-0", patternLabel.color)}>
              {patternLabel.text}
            </Badge>
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {serviceName && methodName
                ? `${serviceName}/${methodName}`
                : <span className="text-muted-foreground italic">未配置服务方法</span>}
            </code>
          </div>

          {/* 连接 ID / 超时 / metadata */}
          {requestConfig && (
            <div className="flex flex-wrap gap-2">
              {requestConfig.connection_id && (
                <Badge variant="outline" className="font-mono text-xs">
                  连接: {requestConfig.connection_id}
                </Badge>
              )}
              {requestConfig.timeout_ms !== undefined && (
                <Badge variant="secondary" className="font-mono text-xs">
                  超时: {requestConfig.timeout_ms}ms
                </Badge>
              )}
              {requestConfig.metadata_count !== undefined && requestConfig.metadata_count > 0 && (
                <Badge variant="outline" className="font-mono text-xs">
                  Metadata: {requestConfig.metadata_count} 项
                </Badge>
              )}
            </div>
          )}

          {/* 请求 JSON */}
          {requestConfig?.request_json && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">请求体</div>
              <div className="bg-muted rounded p-2">
                <RawJsonViewer data={requestConfig.request_json} />
              </div>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 响应状态 */}
      <CollapsibleSection
        title={ll.viewer.grpc.section_response_status()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <Badge
              className={cn(
                'font-mono text-xs px-2 py-0.5',
                isSuccess
                  ? 'text-emerald-600 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/50'
                  : 'text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/50'
              )}
            >
              {output?.status ?? (isSuccess ? 'OK' : 'Error')}
            </Badge>
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `${serviceName}/${methodName} 调用成功`
                : output?.status_message ?? output?.error ?? '调用失败'}
            </span>
          </div>

          {/* 错误信息 */}
          {!isSuccess && output?.error && (
            <div className="bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800 rounded p-2">
              <p className="text-sm text-red-700 dark:text-red-300 break-all">{output.error}</p>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 错误详情（google.rpc.Status 结构化详情） */}
      {!isSuccess && output?.error_details && output.error_details.length > 0 && (
        <GrpcErrorDetailsCard details={output.error_details} />
      )}

      {/* Streaming Statistics */}
      {(output?.sent_count !== undefined || output?.received_count !== undefined) && (
        <CollapsibleSection
          title={ll.viewer.grpc.section_streaming_stats()}
          icon={<Server className="w-4 h-4" />}
          defaultOpen={true}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            {output?.sent_count !== undefined && (
              <div className="flex justify-between items-center p-2 bg-muted rounded">
                <span className="text-muted-foreground">发送消息数</span>
                <span className="font-mono text-blue-600 font-medium">{output.sent_count}</span>
              </div>
            )}
            {output?.received_count !== undefined && (
              <div className="flex justify-between items-center p-2 bg-muted rounded">
                <span className="text-muted-foreground">接收消息数</span>
                <span className="font-mono text-emerald-600 font-medium">{output.received_count}</span>
              </div>
            )}
            {output?.streaming_timing?.first_response_ms !== undefined && (
              <div className="flex justify-between items-center p-2 bg-muted rounded">
                <span className="text-muted-foreground">首响应耗时</span>
                <span className="font-mono text-foreground">{output.streaming_timing.first_response_ms}ms</span>
              </div>
            )}
            {output?.streaming_timing?.total_ms !== undefined && (
              <div className="flex justify-between items-center p-2 bg-muted rounded">
                <span className="text-muted-foreground">流式总耗时</span>
                <span className="font-mono text-foreground">{output.streaming_timing.total_ms}ms</span>
              </div>
            )}
          </div>
        </CollapsibleSection>
      )}

      {/* 实时流式消息（来自 DebugEvent 通道） */}
      {showRealtimeMessages && !hasFinalResults && (
        <CollapsibleSection
          title={`${ll.viewer.grpc.section_streaming_stats()} (${realtimeMessages.length})`}
          icon={<Activity className="w-4 h-4" />}
          defaultOpen={true}
          extra={
            <div className="flex items-center gap-2">
              <span className="text-xs font-mono text-emerald-600 animate-pulse">实时</span>
              <Badge variant="outline" className="font-mono text-xs">
                {realtimeMessages.length}{ll.viewer.common.items({ count: realtimeMessages.length })}
              </Badge>
            </div>
          }
        >
          <div className="space-y-2 max-h-[300px] overflow-y-auto">
            {realtimeMessages.map((msg) => (
              <GrpcRealtimeMessageItem key={`${msg.call_id}-${msg.index}`} message={msg} />
            ))}
          </div>
          {/* 实时错误 */}
          {realtimeError && (
            <div className="mt-2 bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800 rounded p-2">
              <p className="text-sm text-red-700 dark:text-red-300 break-all">{realtimeError.message}</p>
            </div>
          )}
        </CollapsibleSection>
      )}

      {/* Response Data - Single Response (Unary/Client Streaming) */}
      {responseData !== undefined && responseData !== null && !hasStreamingResponses && (
        <CollapsibleSection
          title={ll.viewer.grpc.section_response()}
          icon={<Globe className="w-4 h-4" />}
          defaultOpen={false}
        >
          <div className="bg-muted rounded p-2">
            <RawJsonViewer data={responseData} />
          </div>
        </CollapsibleSection>
      )}

      {/* Response Data - Streaming Responses (Server/Bidi Streaming) */}
      {hasStreamingResponses && (
        <CollapsibleSection
          title={`${ll.viewer.grpc.section_streaming_responses()} (${streamingResponses.length})`}
          icon={<Globe className="w-4 h-4" />}
          defaultOpen={false}
        >
          <div className="space-y-2">
            {streamingResponses.map((resp, idx) => (
              <div key={idx} className="border border-border rounded p-2">
                <div className="flex items-center gap-2 mb-1">
                  <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono">
                    #{resp.index ?? idx + 1}
                  </Badge>
                  {resp.received_at_ms !== undefined && (
                    <span className="text-[10px] text-muted-foreground font-mono">
                      +{resp.received_at_ms}ms
                    </span>
                  )}
                  {resp.size_bytes !== undefined && (
                    <span className="text-[10px] text-muted-foreground font-mono">
                      {formatSizeBytes(resp.size_bytes)}
                    </span>
                  )}
                </div>
                <div className="bg-muted rounded p-1.5">
                  <RawJsonViewer data={resp.data} />
                </div>
              </div>
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* Legacy: array response data fallback (backward compatibility) */}
      {responseData !== undefined && responseData !== null && hasStreamingResponses && Array.isArray(responseData) && (
        <CollapsibleSection
          title={`${ll.viewer.grpc.section_response()} (${responseData.length})`}
          icon={<Globe className="w-4 h-4" />}
          defaultOpen={false}
        >
          <div className="bg-muted rounded p-2">
            <RawJsonViewer data={responseData} />
          </div>
        </CollapsibleSection>
      )}

      {/* 响应 Metadata */}
      {responseMetadata && Object.keys(responseMetadata).length > 0 && (
        <CollapsibleSection
          title={ll.viewer.grpc.section_response_metadata()}
          icon={<Server className="w-4 h-4" />}
          defaultOpen={false}
        >
          <KeyValueList data={responseMetadata} />
        </CollapsibleSection>
      )}

      {/* Trailers */}
      {trailers && Object.keys(trailers).length > 0 && (
        <CollapsibleSection
          title={ll.viewer.grpc.section_trailers()}
          icon={<Server className="w-4 h-4" />}
          defaultOpen={false}
        >
          <KeyValueList data={trailers} />
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(callDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-violet-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            {callDuration !== undefined && (
              <div className="flex justify-between items-center p-2 bg-muted rounded">
                <span className="text-muted-foreground">调用耗时</span>
                <span className="font-mono text-foreground">{callDuration}ms</span>
              </div>
            )}
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// gRPC Close 查看器
// ============================================================================

function GrpcCloseSection({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as GrpcCloseOutput | undefined;
  const resolvedConfig = report.resolved_config as GrpcCloseOutput['request'] | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  // 配置数据
  const requestConfig = output?.request || resolvedConfig;
  const connectionId = output?.connection_id || requestConfig?.connection_id || '';
  const closeDuration = output?.timing?.close_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 关闭配置 */}
      <CollapsibleSection
        title={ll.viewer.grpc.section_close_config()}
        icon={<Unplug className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* 连接 ID */}
          <div className="flex items-center gap-2">
            <Badge
              variant="outline"
              className="font-mono text-xs text-violet-600 dark:text-violet-400 bg-violet-50 dark:bg-violet-950/50 border-0 shrink-0"
            >
              gRPC
            </Badge>
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {connectionId || <span className="text-muted-foreground italic">未指定连接 ID</span>}
            </code>
          </div>
        </div>
      </CollapsibleSection>

      {/* 关闭状态 */}
      <CollapsibleSection
        title={ll.viewer.grpc.section_close()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.grpc.status_closed()} failText={ll.viewer.grpc.status_close_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.grpc.close_success()
                : output?.error ?? ll.viewer.grpc.close_error()}
            </span>
          </div>

          {/* 错误信息 */}
          {!isSuccess && output?.error && (
            <div className="bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800 rounded p-2">
              <p className="text-sm text-red-700 dark:text-red-300 break-all">{output.error}</p>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 耗时分解 */}
      {(closeDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-violet-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            {closeDuration !== undefined && (
              <div className="flex justify-between items-center p-2 bg-muted rounded">
                <span className="text-muted-foreground">关闭耗时</span>
                <span className="font-mono text-foreground">{closeDuration}ms</span>
              </div>
            )}
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
