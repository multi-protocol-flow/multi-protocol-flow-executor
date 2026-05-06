/**
 * SseViewer - SSE 节点执行详情查看器
 * 结构化展示 SSE Connect/Listen/Disconnect 三种节点的执行结果
 * Connect: 连接状态、连接 ID、耗时
 * Listen: 事件列表（事件名、ID、数据、时间）、事件数量、耗时
 * Disconnect: 断开状态、连接 ID
 */
import { useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Cable,
  Ear,
  Unplug,
  Clock,
  Globe,
  MessageSquare,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { VariableChangesCard } from './VariableChangesCard';

interface SseViewerProps {
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

// ============================================================================
// 公共组件
// ============================================================================

/** SSE 协议标签 */
function SseBadge() {
  return (
    <Badge
      variant="outline"
      className="font-mono text-xs text-rose-600 dark:text-rose-400 bg-rose-50 dark:bg-rose-950/50 border-0 shrink-0"
    >
      SSE
    </Badge>
  );
}

/** 状态 Badge */
function StatusBadge({ isSuccess, successText, failText }: { isSuccess: boolean; successText: string; failText: string }) {
  return (
    <Badge
      className={cn(
        'font-mono text-xs px-2 py-0.5',
        isSuccess
          ? 'text-emerald-600 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/50'
          : 'text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/50'
      )}
    >
      {isSuccess ? successText : failText}
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
// SSE 输出数据类型
// ============================================================================

/** SSE Connect 输出数据 */
interface SseConnectOutput {
  success?: boolean;
  connection_id?: string | null;
  timing?: {
    connect_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/** SSE 单条事件 */
interface SseEvent {
  event?: string;
  id?: string;
  data?: string;
  timestamp?: number;
}

/** SSE Listen 输出数据 */
interface SseListenOutput {
  success?: boolean;
  connection_id?: string;
  event_filter?: string;
  events?: SseEvent[];
  event_count?: number;
  duration_ms?: number;
  timing?: {
    listen_ms?: number;
    total_ms?: number;
  };
  error?: string;
  /** 请求参数（connection_id 在此字段内） */
  request?: {
    connection_id?: string;
    event_filter?: string | null;
    max_events?: number;
    timeout_ms?: number;
  };
}

/** SSE Disconnect 输出数据 */
interface SseDisconnectOutput {
  success?: boolean;
  connection_id?: string;
  timing?: {
    disconnect_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

// ============================================================================
// 事件卡片组件
// ============================================================================

function SseEventCard({ event, index }: { event: SseEvent; index: number }) {
  const data = event.data ?? '';
  const truncatedData = data.length > 200 ? data.slice(0, 200) + '...' : data;

  return (
    <div className="border border-border rounded-lg p-2.5">
      <div className="flex items-center gap-2 mb-1.5">
        <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono">
          #{index + 1}
        </Badge>
        {event.event && (
          <code className="text-xs font-mono text-foreground break-all">
            {event.event}
          </code>
        )}
        {event.id && (
          <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono">
            ID: {event.id}
          </Badge>
        )}
        {event.timestamp !== undefined && (
          <span className="text-[10px] text-muted-foreground font-mono ml-auto">
            {event.timestamp}ms
          </span>
        )}
      </div>
      {data && (
        <pre className="text-xs font-mono text-foreground whitespace-pre-wrap break-all max-h-40 overflow-auto bg-muted rounded p-2">
          {truncatedData}
        </pre>
      )}
    </div>
  );
}

// ============================================================================
// 主组件
// ============================================================================

export function SseViewer({ report }: SseViewerProps) {
  const nodeType = report.node_type;

  switch (nodeType) {
    case 'sse_connect':
      return <SseConnectView report={report} />;
    case 'sse_listen':
      return <SseListenView report={report} />;
    case 'sse_disconnect':
      return <SseDisconnectView report={report} />;
    default:
      return (
        <div className="p-4 text-sm text-muted-foreground italic">
          未知的 SSE 节点类型: {nodeType}
        </div>
      );
  }
}

// ============================================================================
// SSE Connect 查看器
// ============================================================================

function SseConnectView({ report }: { report: NodeExecutionReport }) {
  const output = report.output_data as SseConnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const connectDuration = output?.timing?.connect_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 连接状态 */}
      <CollapsibleSection
        title="连接状态"
        icon={<Cable className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已连接" failText="连接失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? 'SSE 连接建立成功'
                : output?.error ?? '无法建立 SSE 连接'}
            </span>
          </div>

          {output?.connection_id && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">连接 ID:</span>
              <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                {output.connection_id}
              </code>
            </div>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 耗时分解 */}
      {(connectDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-rose-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label="连接耗时" value={connectDuration} />
            <TimingMetric label="总耗时" value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// SSE Listen 查看器
// ============================================================================

function SseListenView({ report }: { report: NodeExecutionReport }) {
  const output = report.output_data as SseListenOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const events = output?.events ?? [];
  const eventCount = output?.event_count ?? events.length;
  const listenDuration = output?.timing?.listen_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 监听配置 */}
      <CollapsibleSection
        title="监听配置"
        icon={<Ear className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <SseBadge />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {(output?.request?.connection_id || output?.connection_id) || <span className="text-muted-foreground italic">未指定连接 ID</span>}
            </code>
          </div>

          {output?.event_filter && (
            <div className="flex flex-wrap gap-2">
              <Badge variant="outline" className="font-mono text-xs">
                过滤器: {output.event_filter}
              </Badge>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 监听状态 */}
      <CollapsibleSection
        title="监听状态"
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="监听完成" failText="监听失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `收到 ${eventCount} 个事件`
                : output?.error ?? 'SSE 监听失败'}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 事件列表 */}
      {events.length > 0 && (
        <CollapsibleSection
          title="事件列表"
          icon={<MessageSquare className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <Badge variant="outline" className="font-mono text-xs">
              {eventCount} 个事件
            </Badge>
          }
        >
          <div className="space-y-2">
            {events.map((evt, idx) => (
              <SseEventCard key={idx} event={evt} index={idx} />
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(listenDuration !== undefined || totalDuration !== undefined || output?.duration_ms !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-rose-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label="监听耗时" value={listenDuration ?? output?.duration_ms} />
            <TimingMetric label="总耗时" value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// SSE Disconnect 查看器
// ============================================================================

function SseDisconnectView({ report }: { report: NodeExecutionReport }) {
  const output = report.output_data as SseDisconnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const disconnectDuration = output?.timing?.disconnect_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 断开状态 */}
      <CollapsibleSection
        title="断开状态"
        icon={<Unplug className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已断开" failText="断开失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? 'SSE 连接已成功断开'
                : output?.error ?? '断开连接失败'}
            </span>
          </div>

          {output?.connection_id && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">连接 ID:</span>
              <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                {output.connection_id}
              </code>
            </div>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 耗时分解 */}
      {(disconnectDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-rose-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label="断开耗时" value={disconnectDuration} />
            <TimingMetric label="总耗时" value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
