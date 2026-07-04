/**
 * GraphqlViewer - GraphQL 节点执行详情查看器
 * 结构化展示 GraphQL Connect/Query/Subscribe/Introspect/Disconnect 五种节点的执行结果
 * Connect: 连接状态、连接 ID、服务器信息
 * Query/Mutation: 响应数据 (JSON)、错误列表、扩展信息、耗时、HTTP 状态码
 * Subscribe: 事件列表（data/errors/received_at）、事件数量、持续时间
 * Introspect: Schema 类型列表、查询模板
 * Disconnect: 断开状态
 */
import { useState, memo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Cable,
  Send,
  Radio,
  Search,
  Unplug,
  Clock,
  Globe,
  FileJson,
  AlertTriangle,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { cn } from '@/lib/utils';
import { VariableChangesCard } from './VariableChangesCard';
import { getLL } from '@/i18n/i18n-lazy';
import { RawJsonViewer } from './RawJsonViewer';

interface GraphqlViewerProps {
  report: NodeExecutionReport;
}

/** 折叠面板组件 */
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

// ============================================================================
// 公共组件
// ============================================================================

/** GraphQL 协议标签 */
function GraphqlBadge() {
  return (
    <Badge
      variant="outline"
      className="font-mono text-xs text-indigo-600 dark:text-indigo-400 bg-indigo-50 dark:bg-indigo-950/50 border-0 shrink-0"
    >
      GraphQL
    </Badge>
  );
}

/** 错误信息展示 */
function ErrorBlock({ error }: { error?: string }) {
  if (!error) return null;
  return (
    <div className="bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800 rounded p-2">
      <p className="text-sm text-red-700 dark:text-red-300 break-all">{error}</p>
    </div>
  );
}

/** 耗时指标卡片 */
function TimingMetric({ label, value }: { label: string; value: number | undefined }) {
  if (value === undefined) return null;
  return (
    <div className="flex justify-between items-center p-2 bg-muted rounded">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-mono text-foreground">{value}ms</span>
    </div>
  );
}

// ============================================================================
// GraphQL 输出数据类型
// ============================================================================

/** GraphQL Connect 输出数据 */
interface GraphqlConnectOutput {
  success?: boolean;
  connection_id?: string | null;
  server_info?: string | null;
  error?: string;
}

/** GraphQL 请求耗时信息 */
interface GraphqlTimingInfo {
  total_ms?: number;
  dns_ms?: number;
  connect_ms?: number;
  tls_ms?: number;
  server_ms?: number;
}

/** GraphQL Query/Mutation 输出数据 */
interface GraphqlQueryOutput {
  success?: boolean;
  data?: unknown;
  errors?: unknown[];
  extensions?: unknown;
  status_code?: number;
  timing?: GraphqlTimingInfo;
  error?: string;
}

/** GraphQL 订阅事件 */
interface GraphqlSubscriptionEvent {
  data?: unknown;
  errors?: unknown[];
  extensions?: unknown;
  received_at?: number;
}

/** GraphQL Subscribe 输出数据 */
interface GraphqlSubscribeOutput {
  success?: boolean;
  events?: GraphqlSubscriptionEvent[];
  event_count?: number;
  duration_ms?: number;
  error?: string;
}

/** GraphQL 查询模板 */
interface GraphqlQueryTemplate {
  type_name: string;
  operation: string;
  template: string;
}

/** GraphQL Introspect 输出数据 */
interface GraphqlIntrospectOutput {
  success?: boolean;
  schema?: unknown;
  query_templates?: GraphqlQueryTemplate[];
  error?: string;
}

/** GraphQL Disconnect 输出数据 */
interface GraphqlDisconnectOutput {
  success?: boolean;
  error?: string;
}

// ============================================================================
// 订阅事件卡片组件
// ============================================================================

function GraphqlEventCard({ event, index }: { event: GraphqlSubscriptionEvent; index: number }) {
  return (
    <div className="border border-border rounded-lg p-2.5">
      <div className="flex items-center gap-2 mb-1.5">
        <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono">
          #{index + 1}
        </Badge>
        {event.received_at !== undefined && (
          <span className="text-[10px] text-muted-foreground font-mono ml-auto">
            {new Date(event.received_at).toLocaleTimeString()}
          </span>
        )}
      </div>
      {event.data !== undefined && event.data !== null && (
        <div className="bg-muted rounded p-2 max-h-40 overflow-auto mb-1.5">
          <RawJsonViewer data={event.data} />
        </div>
      )}
      {event.errors && event.errors.length > 0 && (
        <div className="bg-red-50 dark:bg-red-950/20 border border-red-200 dark:border-red-800 rounded p-1.5">
          <pre className="text-xs font-mono text-red-700 dark:text-red-300 whitespace-pre-wrap break-all">
            {JSON.stringify(event.errors, null, 2)}
          </pre>
        </div>
      )}
    </div>
  );
}

// ============================================================================
// 主组件
// ============================================================================

export const GraphqlViewer = memo(function GraphqlViewer({ report }: GraphqlViewerProps) {
  const nodeType = report.node_type;
  const ll = getLL();

  switch (nodeType) {
    case 'graphql_connect':
      return <GraphqlConnectView report={report} />;
    case 'graphql_query':
      return <GraphqlQueryView report={report} />;
    case 'graphql_subscribe':
      return <GraphqlSubscribeView report={report} />;
    case 'graphql_introspect':
      return <GraphqlIntrospectView report={report} />;
    case 'graphql_disconnect':
      return <GraphqlDisconnectView report={report} />;
    default:
      return (
        <div className="p-4 text-sm text-muted-foreground italic">
          {ll.viewer.graphql.unknown_node_type({ type: nodeType })}
        </div>
      );
  }
});

// ============================================================================
// GraphQL Connect 查看器
// ============================================================================

function GraphqlConnectView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as GraphqlConnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      {/* 连接状态 */}
      <CollapsibleSection
        title={ll.viewer.graphql.section_connection()}
        icon={<Cable className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.graphql.status_connected()} failText={ll.viewer.graphql.status_connection_failed()} />
            <GraphqlBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.graphql.status_connected()
                : output?.error ?? ll.viewer.graphql.status_connection_failed()}
            </span>
          </div>

          {output?.connection_id && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{ll.viewer.graphql.label_connection_id()}:</span>
              <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                {output.connection_id}
              </code>
            </div>
          )}

          {output?.server_info && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{ll.viewer.graphql.label_server_info()}:</span>
              <span className="text-xs text-foreground">{output.server_info}</span>
            </div>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// GraphQL Query/Mutation 查看器
// ============================================================================

function GraphqlQueryView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as GraphqlQueryOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const hasData = output?.data !== undefined && output?.data !== null;
  const hasErrors = output?.errors && output.errors.length > 0;
  const hasExtensions = output?.extensions !== undefined && output?.extensions !== null;
  const timing = output?.timing;

  return (
    <div className="space-y-3">
      {/* 请求状态 */}
      <CollapsibleSection
        title={ll.viewer.graphql.section_query_status()}
        icon={<Send className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.graphql.status_success()} failText={ll.viewer.graphql.status_failed()} />
            <GraphqlBadge />
            {output?.status_code !== undefined && (
              <Badge
                variant="outline"
                className={cn(
                  'font-mono text-xs px-2 py-0.5',
                  output.status_code >= 200 && output.status_code < 300
                    ? 'text-emerald-600 dark:text-emerald-400'
                    : output.status_code >= 400
                      ? 'text-red-600 dark:text-red-400'
                      : 'text-amber-600 dark:text-amber-400'
                )}
              >
                HTTP {output.status_code}
              </Badge>
            )}
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? hasErrors
                  ? ll.viewer.graphql.query_success_with_errors({ count: output.errors!.length })
                  : ll.viewer.graphql.status_success()
                : output?.error ?? ll.viewer.graphql.query_error()}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 响应数据 */}
      {hasData && (
        <CollapsibleSection
          title={ll.viewer.graphql.section_response_data()}
          icon={<FileJson className="w-4 h-4" />}
          defaultOpen={true}
        >
          <div className="bg-muted rounded p-2 max-h-60 overflow-auto">
            <RawJsonViewer data={output!.data} />
          </div>
        </CollapsibleSection>
      )}

      {/* GraphQL 错误 */}
      {hasErrors && (
        <CollapsibleSection
          title={ll.viewer.graphql.section_errors()}
          icon={<AlertTriangle className="w-4 h-4" />}
          defaultOpen={true}
          extra={
            <Badge variant="outline" className="font-mono text-xs text-red-600 dark:text-red-400">
              {ll.viewer.graphql.error_count({count: output!.errors!.length})}
            </Badge>
          }
        >
          <div className="bg-red-50 dark:bg-red-950/20 border border-red-200 dark:border-red-800 rounded p-2 max-h-40 overflow-auto">
            <pre className="text-xs font-mono text-red-700 dark:text-red-300 whitespace-pre-wrap break-all">
              {JSON.stringify(output!.errors, null, 2)}
            </pre>
          </div>
        </CollapsibleSection>
      )}

      {/* 扩展信息 */}
      {hasExtensions && (
        <CollapsibleSection
          title={ll.viewer.graphql.section_extensions()}
          icon={<Globe className="w-4 h-4" />}
          defaultOpen={false}
        >
          <div className="bg-muted rounded p-2 max-h-40 overflow-auto">
            <RawJsonViewer data={output!.extensions} />
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {timing && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            timing.total_ms !== undefined && (
              <span className="text-xs font-mono text-indigo-600 font-medium">
                {ll.viewer.common.total_duration_ms({ms: timing.total_ms})}
              </span>
            )
          }
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_dns()} value={timing.dns_ms} />
            <TimingMetric label={ll.viewer.common.timing_connect()} value={timing.connect_ms} />
            <TimingMetric label={ll.viewer.common.timing_tls()} value={timing.tls_ms} />
            <TimingMetric label={ll.viewer.common.timing_server()} value={timing.server_ms} />
            <TimingMetric label={ll.viewer.common.timing_total()} value={timing.total_ms} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// GraphQL Subscribe 查看器
// ============================================================================

function GraphqlSubscribeView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as GraphqlSubscribeOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const events = output?.events ?? [];
  const eventCount = output?.event_count ?? events.length;

  return (
    <div className="space-y-3">
      {/* 订阅状态 */}
      <CollapsibleSection
        title={ll.viewer.graphql.section_subscription_status()}
        icon={<Radio className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.graphql.status_subscribed()} failText={ll.viewer.graphql.status_subscribe_failed()} />
            <GraphqlBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.graphql.events_received({ count: eventCount })
                : output?.error ?? ll.viewer.graphql.subscribe_error()}
            </span>
          </div>

          {output?.duration_ms !== undefined && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{ll.viewer.graphql.label_duration()}:</span>
              <span className="font-mono text-xs text-foreground">{output.duration_ms}ms</span>
            </div>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 事件列表 */}
      {events.length > 0 && (
        <CollapsibleSection
          title={ll.viewer.graphql.section_events()}
          icon={<FileJson className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <Badge variant="outline" className="font-mono text-xs">
              {ll.viewer.graphql.event_count({count: eventCount})}
            </Badge>
          }
        >
          <div className="space-y-2">
            {events.map((evt, idx) => (
              <GraphqlEventCard key={idx} event={evt} index={idx} />
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// GraphQL Introspect 查看器
// ============================================================================

function GraphqlIntrospectView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as GraphqlIntrospectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const templates = output?.query_templates ?? [];
  const hasSchema = output?.schema !== undefined && output?.schema !== null;

  return (
    <div className="space-y-3">
      {/* 探索状态 */}
      <CollapsibleSection
        title={ll.viewer.graphql.section_explore_status()}
        icon={<Search className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.graphql.status_explore_success()} failText={ll.viewer.graphql.status_explore_failed()} />
            <GraphqlBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? templates.length > 0
                  ? ll.viewer.graphql.templates_found({ count: templates.length })
                  : ll.viewer.graphql.explore_success()
                : output?.error ?? ll.viewer.graphql.explore_error()}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* Schema 完整数据 */}
      {hasSchema && (
        <CollapsibleSection
          title={ll.viewer.graphql.section_schema_data()}
          icon={<Globe className="w-4 h-4" />}
          defaultOpen={false}
        >
          <div className="bg-muted rounded p-2 max-h-60 overflow-auto">
            <RawJsonViewer data={output.schema} />
          </div>
        </CollapsibleSection>
      )}

      {/* 查询模板 */}
      {templates.length > 0 && (
        <CollapsibleSection
          title={ll.viewer.graphql.section_query_templates()}
          icon={<FileJson className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <Badge variant="outline" className="font-mono text-xs">
              {ll.viewer.graphql.template_count({count: templates.length})}
            </Badge>
          }
        >
          <div className="space-y-2">
            {templates.map((tmpl, idx) => (
              <div key={`${tmpl.type_name}-${idx}`} className="border border-border rounded-lg p-2.5">
                <div className="flex items-center gap-2 mb-1.5">
                  <Badge
                    variant="outline"
                    className={cn(
                      'text-[10px] px-1.5 py-0 font-mono border-0',
                      tmpl.operation === 'query'
                        ? 'text-indigo-600 dark:text-indigo-400 bg-indigo-50 dark:bg-indigo-950/50'
                        : tmpl.operation === 'mutation'
                          ? 'text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-950/50'
                          : 'text-violet-600 dark:text-violet-400 bg-violet-50 dark:bg-violet-950/50'
                    )}
                  >
                    {tmpl.operation}
                  </Badge>
                  <code className="text-xs font-mono text-foreground font-medium">
                    {tmpl.type_name}
                  </code>
                </div>
                <pre className="text-xs font-mono text-muted-foreground whitespace-pre-wrap break-all bg-muted rounded p-2 max-h-32 overflow-auto">
                  {tmpl.template}
                </pre>
              </div>
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// GraphQL Disconnect 查看器
// ============================================================================

function GraphqlDisconnectView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as GraphqlDisconnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      {/* 断开状态 */}
      <CollapsibleSection
        title={ll.viewer.graphql.section_disconnect()}
        icon={<Unplug className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.graphql.status_disconnected()} failText={ll.viewer.graphql.status_disconnect_failed()} />
            <GraphqlBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.graphql.status_disconnected()
                : output?.error ?? ll.viewer.graphql.status_disconnect_failed()}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
