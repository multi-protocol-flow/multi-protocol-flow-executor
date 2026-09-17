/**
 * TcpUdpViewer - TCP/UDP 节点执行详情查看器
 * 结构化展示 TCP Connect/Send/Receive/Close 和 UDP Bind/SendTo/RecvFrom/Close 八种节点的执行结果
 *
 * TCP Connect: 连接配置、连接状态、耗时
 * TCP Send: 发送数据、接收数据（含 hex 预览）、耗时
 * TCP Receive: 接收数据（含 hex 预览）、耗时
 * TCP Close: 关闭配置、关闭状态、耗时
 * UDP Bind: 绑定配置、绑定状态、耗时
 * UDP SendTo: 发送数据、目标地址、耗时
 * UDP RecvFrom: 接收数据（含 hex 预览）、来源地址、耗时
 * UDP Close: 关闭配置、关闭状态、耗时
 */
import { useState, memo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Cable,
  Unplug,
  Clock,
  Globe,
  ArrowUpFromLine,
  ArrowDownToLine,
  Router,
  CircleDot,
} from 'lucide-react';
import type { NodeExecutionReport, TcpUdpRecording } from '@/types';
import { Badge } from '@/components/ui/badge';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { cn } from '@/lib/utils';
import { VariableChangesCard } from './VariableChangesCard';
import { HexViewer, dataToBytes } from '@/components/common/HexViewer';
import { RecordingViewer } from './RecordingViewer';
import { getLL } from '@/i18n/i18n-lazy';
import { CopyButton } from '@/components/common';
interface TcpUdpViewerProps {
  /** 节点执行报告 */
  report: NodeExecutionReport;
  /** 关联的 TCP/UDP 录制数据（可选） */
  recording?: TcpUdpRecording;
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

// ============================================================================
// TCP/UDP 输出数据类型
// ============================================================================

/**
 * TCP Connect 输出数据
 */
interface TcpConnectOutput {
  success?: boolean;
  connection_id?: string | null;
  host?: string;
  port?: number;
  use_tls?: boolean;
  timing?: {
    total_ms?: number;
  };
  error?: string;
}

/**
 * TCP Send 输出数据
 */
interface TcpSendOutput {
  success?: boolean;
  connection_id?: string;
  sent?: {
    data?: string;
    encoding?: string;
    size?: number;
    framing?: string;
  };
  received?: {
    data?: string;
    encoding?: string;
    size?: number;
    text_preview?: string;
  };
  timing?: {
    send_ms?: number;
    receive_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/**
 * TCP Receive 输出数据
 */
interface TcpReceiveOutput {
  success?: boolean;
  connection_id?: string;
  received?: {
    data?: string;
    encoding?: string;
    size?: number;
  } | null;
  received_text_preview?: string | null;
  timing?: {
    receive_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/**
 * TCP Close 输出数据
 */
interface TcpCloseOutput {
  success?: boolean;
  connection_id?: string;
  timing?: {
    total_ms?: number;
  };
  error?: string;
}

/**
 * UDP Bind 输出数据
 */
interface UdpBindOutput {
  success?: boolean;
  connection_id?: string;
  local_addr?: string;
  local_port?: number;
  timing?: {
    total_ms?: number;
  };
  error?: string;
}

/**
 * UDP SendTo 输出数据
 */
interface UdpSendToOutput {
  success?: boolean;
  connection_id?: string;
  sent?: {
    data?: string;
    encoding?: string;
    size?: number;
    target?: string;
  };
  timing?: {
    total_ms?: number;
  };
  error?: string;
}

/**
 * UDP RecvFrom 输出数据
 */
interface UdpRecvFromOutput {
  success?: boolean;
  connection_id?: string;
  received?: {
    data?: string;
    encoding?: string;
    size?: number;
    source_addr?: string;
    source_port?: number;
  };
  timing?: {
    total_ms?: number;
  };
  error?: string;
}

/**
 * UDP Close 输出数据
 */
interface UdpCloseOutput {
  success?: boolean;
  connection_id?: string;
  timing?: {
    total_ms?: number;
  };
  error?: string;
}

// ============================================================================
// Hex 预览工具 — 已迁移至 HexViewer 组件
// ============================================================================

/**
 * 错误信息展示
 */
function ErrorBlock({ error }: { error?: string }) {
  if (!error) return null;
  return (
    <div className="bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800 rounded p-2">
      <p className="text-sm text-red-700 dark:text-red-300 break-all">{error}</p>
    </div>
  );
}

/**
 * 耗时指标卡片
 */
function TimingMetric({ label, value }: { label: string; value: number | undefined }) {
  if (value === undefined) return null;
  return (
    <div className="flex justify-between items-center p-2 bg-muted rounded">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-mono text-foreground">{value}ms</span>
    </div>
  );
}

/**
 * 协议标签颜色映射
 */
function ProtocolBadge({ protocol }: { protocol: 'TCP' | 'UDP' }) {
  const colorClass = protocol === 'TCP'
    ? 'text-teal-600 dark:text-teal-400 bg-teal-50 dark:bg-teal-950/50 border-0'
    : 'text-orange-600 dark:text-orange-400 bg-orange-50 dark:bg-orange-950/50 border-0';
  return (
    <Badge variant="outline" className={cn('font-mono text-xs shrink-0', colorClass)}>
      {protocol}
    </Badge>
  );
}

export const TcpUdpViewer = memo(function TcpUdpViewer({ report, recording }: TcpUdpViewerProps) {
  const ll = getLL();
  const nodeType = report.node_type;

  switch (nodeType) {
    case 'tcp:connect':
      return <TcpConnectSection report={report} />;
    case 'tcp:send':
      return <TcpSendSection report={report} recording={recording} />;
    case 'tcp:receive':
      return <TcpReceiveSection report={report} recording={recording} />;
    case 'tcp:close':
      return <TcpCloseSection report={report} />;
    case 'udp:bind':
      return <UdpBindSection report={report} />;
    case 'udp:send_to':
      return <UdpSendToSection report={report} />;
    case 'udp:recv_from':
      return <UdpRecvFromSection report={report} />;
    case 'udp:close':
      return <UdpCloseSection report={report} />;
    default:
      return (
        <div className="p-4 text-sm text-muted-foreground italic">
          {ll.viewer.tcp.unknown_node_type({ type: nodeType })}
    </div>
  );
  }
});

// ============================================================================
// TCP Connect 查看器
// ============================================================================

function TcpConnectSection({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as TcpConnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 连接配置 */}
      <CollapsibleSection
        title={ll.viewer.tcp.section_connection_info()}
        icon={<Cable className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* 主机地址 */}
          <div className="flex items-center gap-2">
            <ProtocolBadge protocol="TCP" />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {output?.host
                ? `${output.host}:${output?.port ?? ''}`
                : <span className="text-muted-foreground italic">{ll.viewer.tcp.no_address_configured()}</span>}
            </code>
          </div>

          {/* TLS / 连接信息 */}
          <div className="flex flex-wrap gap-2">
            {output?.use_tls !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                TLS: {output.use_tls ? ll.viewer.tcp.label_enabled() : ll.viewer.tcp.label_disabled()}
              </Badge>
            )}
            {output?.connection_id && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.tcp.label_connection_id()}: {output.connection_id}
              </Badge>
            )}
          </div>
        </div>
      </CollapsibleSection>

      {/* 连接状态 */}
      <CollapsibleSection
        title={ll.viewer.tcp.section_connection_status()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.tcp.status_connected()} failText={ll.viewer.tcp.status_connection_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.tcp.connect_success()
                : output?.error ?? ll.viewer.tcp.connect_error()}
            </span>
          </div>

          {/* 连接 ID */}
          {output?.connection_id && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{ll.viewer.tcp.label_connection_id()}:</span>
              <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                {output.connection_id}
              </code>
            </div>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 耗时 */}
      {totalDuration !== undefined && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <span className="text-xs font-mono text-teal-600 font-medium">
              {ll.viewer.common.total_duration_ms({ ms: totalDuration })}
            </span>
          }
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_total()} value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// TCP Send 查看器
// ============================================================================

function TcpSendSection({ report, recording }: { report: NodeExecutionReport; recording?: TcpUdpRecording }) {
  const ll = getLL();
  const [showRecording, setShowRecording] = useState(false);
  const output = report.output_data as TcpSendOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const sentData = output?.sent;
  const receivedData = output?.received;
  const sendDuration = output?.timing?.send_ms;
  const receiveDuration = output?.timing?.receive_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 发送数据 */}
      <CollapsibleSection
        title={ll.viewer.tcp.section_send_data()}
        icon={<ArrowUpFromLine className="w-4 h-4" />}
        defaultOpen={true}
        extra={
          <div className="flex items-center gap-2">
            {sentData?.size !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                {sentData.size} bytes
              </Badge>
            )}
            {sentData && (
              <CopyButton
                value={sentData.data ?? sentData}
                title={ll.execution.report.copy_all()}
                size="xs"
              />
            )}
          </div>
        }
      >
        <div className="space-y-2">
          {/* 编码 / 分帧信息 */}
          <div className="flex flex-wrap gap-2">
            {sentData?.encoding && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.tcp.label_encoding()}: {sentData.encoding}
              </Badge>
            )}
            {sentData?.framing && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.tcp.label_framing()}: {sentData.framing}
              </Badge>
            )}
            {output?.connection_id && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.tcp.label_connection()}: {output.connection_id}
              </Badge>
            )}
          </div>

          {/* 发送内容 */}
          {sentData?.data && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">{ll.viewer.tcp.label_data_content()}</div>
              <div className="bg-muted rounded p-2">
                <pre className="text-xs font-mono text-foreground whitespace-pre-wrap break-all max-h-60 overflow-auto">
                  {sentData.data}
                </pre>
              </div>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 接收数据 */}
      {receivedData && (
        <CollapsibleSection
          title={ll.viewer.tcp.section_receive_data()}
          icon={<ArrowDownToLine className="w-4 h-4" />}
          defaultOpen={true}
          extra={
            <div className="flex items-center gap-2">
              {receivedData.size !== undefined && (
                <Badge variant="outline" className="font-mono text-xs">
                  {receivedData.size} bytes
                </Badge>
              )}
              <CopyButton
                value={receivedData.data ?? (receivedData as { text_preview?: string }).text_preview ?? receivedData}
                title={ll.execution.report.copy_all()}
                size="xs"
              />
            </div>
          }
        >
          <div className="space-y-2">
            {/* 编码信息 */}
            <div className="flex flex-wrap gap-2">
              {receivedData.encoding && (
                <Badge variant="outline" className="font-mono text-xs">
                  {ll.viewer.tcp.label_encoding()}: {receivedData.encoding}
                </Badge>
              )}
            </div>

            {/* 文本预览 */}
            {receivedData.text_preview && (
              <div>
                <div className="text-xs font-medium text-muted-foreground mb-1.5">{ll.viewer.tcp.label_text_preview()}</div>
                <div className="bg-muted rounded p-2">
                  <pre className="text-xs font-mono text-foreground whitespace-pre-wrap break-all max-h-60 overflow-auto">
                    {receivedData.text_preview}
                  </pre>
                </div>
              </div>
            )}

            {/* Hex 预览 */}
            {receivedData.data && (
              <div>
                <div className="text-xs font-medium text-muted-foreground mb-1.5">{ll.viewer.tcp.label_hex_preview()}</div>
                <HexViewer data={dataToBytes(receivedData.data)} height={300} />
              </div>
            )}
          </div>
        </CollapsibleSection>
      )}

      {/* 执行状态 */}
      <CollapsibleSection
        title={ll.viewer.common.section_execution_status()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.tcp.status_sent()} failText={ll.viewer.tcp.status_send_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? receivedData ? ll.viewer.tcp.send_receive_complete() : ll.viewer.tcp.send_complete()
                : output?.error ?? ll.viewer.tcp.status_send_failed()}
            </span>
          </div>
          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 耗时分解 */}
      {(sendDuration !== undefined || receiveDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-teal-600 font-medium">
              {ll.viewer.common.total_duration_ms({ ms: totalDuration })}
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_send()} value={sendDuration} />
            <TimingMetric label={ll.viewer.common.timing_receive()} value={receiveDuration} />
            <TimingMetric label={ll.viewer.common.timing_total()} value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />

      {/* 录制查看 */}
      {recording && (
        <CollapsibleSection
          title={ll.viewer.tcp.section_view_recording()}
          icon={<CircleDot className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <button
              type="button"
              onClick={(e) => { e.stopPropagation(); setShowRecording(!showRecording); }}
              className="text-xs text-teal-600 hover:text-teal-700 dark:text-teal-400 dark:hover:text-teal-300"
            >
              {showRecording ? ll.viewer.recording.collapse() : ll.viewer.recording.expand()}
            </button>
          }
        >
          {showRecording ? (
            <RecordingViewer recording={recording} />
          ) : (
            <p className="text-sm text-muted-foreground">
              {ll.viewer.tcp.recording_summary({ count: recording.entries.length, duration: recording.duration_ms })}
            </p>
          )}
        </CollapsibleSection>
      )}
    </div>
  );
}

// ============================================================================
// TCP Receive 查看器
// ============================================================================

function TcpReceiveSection({ report, recording }: { report: NodeExecutionReport; recording?: TcpUdpRecording }) {
  const ll = getLL();
  const [showRecording, setShowRecording] = useState(false);
  const output = report.output_data as TcpReceiveOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const receivedData = output?.received;
  const textPreview = output?.received_text_preview;
  const receiveDuration = output?.timing?.receive_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 接收数据 */}
      <CollapsibleSection
        title={ll.viewer.tcp.section_receive_data()}
        icon={<ArrowDownToLine className="w-4 h-4" />}
        defaultOpen={true}
        extra={
          <div className="flex items-center gap-2">
            {receivedData?.size !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                {receivedData.size} bytes
              </Badge>
            )}
            {receivedData && (
              <CopyButton
                value={receivedData.data ?? receivedData}
                title={ll.execution.report.copy_all()}
                size="xs"
              />
            )}
          </div>
        }
      >
        <div className="space-y-2">
          {/* 编码 / 连接信息 */}
          <div className="flex flex-wrap gap-2">
            <ProtocolBadge protocol="TCP" />
            {receivedData?.encoding && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.tcp.label_encoding()}: {receivedData.encoding}
              </Badge>
            )}
            {output?.connection_id && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.tcp.label_connection()}: {output.connection_id}
              </Badge>
            )}
          </div>

          {/* 文本预览 */}
          {textPreview && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">{ll.viewer.tcp.label_text_preview()}</div>
              <div className="bg-muted rounded p-2">
                <pre className="text-xs font-mono text-foreground whitespace-pre-wrap break-all max-h-60 overflow-auto">
                  {textPreview}
                </pre>
              </div>
            </div>
          )}

          {/* Hex 预览 */}
          {receivedData?.data && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">{ll.viewer.tcp.label_hex_preview()}</div>
              <HexViewer data={dataToBytes(receivedData.data)} height={300} />
            </div>
          )}

          {/* 无数据提示（超时等场景） */}
          {(!receivedData?.data && !textPreview) && (
            <p className="text-sm text-muted-foreground italic">{ll.viewer.tcp.no_received_data()}</p>
          )}
        </div>
      </CollapsibleSection>

      {/* 执行状态 */}
      <CollapsibleSection
        title={ll.viewer.common.section_execution_status()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.tcp.status_received()} failText={ll.viewer.tcp.status_receive_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? receivedData
                  ? ll.viewer.tcp.receive_success_with_size({ size: receivedData.size ?? 0 })
                  : ll.viewer.tcp.receive_timeout_no_data()
                : output?.error ?? ll.viewer.tcp.status_receive_failed()}
            </span>
          </div>
          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 耗时分解 */}
      {(receiveDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-teal-600 font-medium">
              {ll.viewer.common.total_duration_ms({ ms: totalDuration })}
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_receive()} value={receiveDuration} />
            <TimingMetric label={ll.viewer.common.timing_total()} value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />

      {/* 录制查看 */}
      {recording && (
        <CollapsibleSection
          title={ll.viewer.tcp.section_view_recording()}
          icon={<CircleDot className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <button
              type="button"
              onClick={(e) => { e.stopPropagation(); setShowRecording(!showRecording); }}
              className="text-xs text-teal-600 hover:text-teal-700 dark:text-teal-400 dark:hover:text-teal-300"
            >
              {showRecording ? ll.viewer.recording.collapse() : ll.viewer.recording.expand()}
            </button>
          }
        >
          {showRecording ? (
            <RecordingViewer recording={recording} />
          ) : (
            <p className="text-sm text-muted-foreground">
              {ll.viewer.tcp.recording_summary({ count: recording.entries.length, duration: recording.duration_ms })}
            </p>
          )}
        </CollapsibleSection>
      )}
    </div>
  );
}

// ============================================================================
// TCP Close 查看器
// ============================================================================

function TcpCloseSection({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as TcpCloseOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 关闭配置 */}
      <CollapsibleSection
        title={ll.viewer.tcp.section_close_config()}
        icon={<Unplug className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <ProtocolBadge protocol="TCP" />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {output?.connection_id
                ? output.connection_id
                : <span className="text-muted-foreground italic">{ll.viewer.tcp.no_connection_id()}</span>}
            </code>
          </div>
        </div>
      </CollapsibleSection>

      {/* 关闭状态 */}
      <CollapsibleSection
        title={ll.viewer.tcp.section_close_status()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.tcp.status_closed()} failText={ll.viewer.tcp.status_close_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.tcp.close_success()
                : output?.error ?? ll.viewer.tcp.close_error()}
            </span>
          </div>
          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 耗时 */}
      {totalDuration !== undefined && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <span className="text-xs font-mono text-teal-600 font-medium">
              {ll.viewer.common.total_duration_ms({ ms: totalDuration })}
            </span>
          }
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_total()} value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// UDP Bind 查看器
// ============================================================================

function UdpBindSection({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as UdpBindOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 绑定配置 */}
      <CollapsibleSection
        title={ll.viewer.tcp.section_bind_config()}
        icon={<Router className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* 本地地址 */}
          <div className="flex items-center gap-2">
            <ProtocolBadge protocol="UDP" />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {output?.local_addr
                ? `${output.local_addr}:${output?.local_port ?? ''}`
                : <span className="text-muted-foreground italic">{ll.viewer.tcp.no_address_configured()}</span>}
            </code>
          </div>

          {/* 连接 ID */}
          {output?.connection_id && (
            <div className="flex flex-wrap gap-2">
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.tcp.label_connection_id()}: {output.connection_id}
              </Badge>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 绑定状态 */}
      <CollapsibleSection
        title={ll.viewer.tcp.section_bind_status()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.tcp.status_bound()} failText={ll.viewer.tcp.status_bind_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.tcp.udp_bind_success_addr({ addr: `${output?.local_addr ?? ''}:${output?.local_port ?? ''}` })
                : output?.error ?? ll.viewer.tcp.bind_success()}
            </span>
          </div>

          {/* 连接 ID */}
          {output?.connection_id && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{ll.viewer.tcp.label_connection_id()}:</span>
              <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                {output.connection_id}
              </code>
            </div>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 耗时 */}
      {totalDuration !== undefined && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <span className="text-xs font-mono text-orange-600 font-medium">
              {ll.viewer.common.total_duration_ms({ ms: totalDuration })}
            </span>
          }
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_total()} value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// UDP SendTo 查看器
// ============================================================================

function UdpSendToSection({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as UdpSendToOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const sentData = output?.sent;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 发送数据 */}
      <CollapsibleSection
        title={ll.viewer.tcp.section_send_data()}
        icon={<ArrowUpFromLine className="w-4 h-4" />}
        defaultOpen={true}
        extra={
          <div className="flex items-center gap-2">
            {sentData?.size !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                {sentData.size} bytes
              </Badge>
            )}
            {sentData && (
              <CopyButton
                value={sentData.data ?? sentData}
                title={ll.execution.report.copy_all()}
                size="xs"
              />
            )}
          </div>
        }
      >
        <div className="space-y-2">
          {/* 目标地址 */}
          <div className="flex items-center gap-2">
            <ProtocolBadge protocol="UDP" />
            {sentData?.target ? (
              <code className="text-sm font-mono text-foreground bg-muted px-2 py-1 rounded">
                → {sentData.target}
              </code>
            ) : (
              <span className="text-sm text-muted-foreground italic">{ll.viewer.tcp.no_target_address()}</span>
            )}
          </div>

          {/* 编码信息 */}
          <div className="flex flex-wrap gap-2">
            {sentData?.encoding && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.tcp.label_encoding()}: {sentData.encoding}
              </Badge>
            )}
            {output?.connection_id && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.tcp.label_connection()}: {output.connection_id}
              </Badge>
            )}
          </div>

          {/* 发送内容 */}
          {sentData?.data && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">{ll.viewer.tcp.label_data_content()}</div>
              <div className="bg-muted rounded p-2">
                <pre className="text-xs font-mono text-foreground whitespace-pre-wrap break-all max-h-60 overflow-auto">
                  {sentData.data}
                </pre>
              </div>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 执行状态 */}
      <CollapsibleSection
        title={ll.viewer.common.section_execution_status()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.tcp.status_sent()} failText={ll.viewer.tcp.status_send_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? sentData?.target
                  ? ll.viewer.tcp.udp_sent_to({ target: sentData.target })
                  : ll.viewer.tcp.udp_sent()
                : output?.error ?? ll.viewer.tcp.status_send_failed()}
            </span>
          </div>
          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 耗时 */}
      {totalDuration !== undefined && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <span className="text-xs font-mono text-orange-600 font-medium">
              {ll.viewer.common.total_duration_ms({ ms: totalDuration })}
            </span>
          }
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_total()} value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// UDP RecvFrom 查看器
// ============================================================================

function UdpRecvFromSection({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as UdpRecvFromOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const receivedData = output?.received;
  const totalDuration = output?.timing?.total_ms;

  // 来源地址
  const sourceAddr = receivedData?.source_addr
    ? `${receivedData.source_addr}:${receivedData?.source_port ?? ''}`
    : null;

  return (
    <div className="space-y-3">
      {/* 接收数据 */}
      <CollapsibleSection
        title={ll.viewer.tcp.section_receive_data()}
        icon={<ArrowDownToLine className="w-4 h-4" />}
        defaultOpen={true}
        extra={
          <div className="flex items-center gap-2">
            {receivedData?.size !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                {receivedData.size} bytes
              </Badge>
            )}
            {receivedData && (
              <CopyButton
                value={receivedData.data ?? receivedData}
                title={ll.execution.report.copy_all()}
                size="xs"
              />
            )}
          </div>
        }
      >
        <div className="space-y-2">
          {/* 来源地址 */}
          <div className="flex items-center gap-2">
            <ProtocolBadge protocol="UDP" />
            {sourceAddr ? (
              <code className="text-sm font-mono text-foreground bg-muted px-2 py-1 rounded">
                ← {sourceAddr}
              </code>
            ) : (
              <span className="text-sm text-muted-foreground italic">{ll.viewer.tcp.unknown_source()}</span>
            )}
          </div>

          {/* 编码信息 */}
          <div className="flex flex-wrap gap-2">
            {receivedData?.encoding && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.tcp.label_encoding()}: {receivedData.encoding}
              </Badge>
            )}
            {output?.connection_id && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.tcp.label_connection()}: {output.connection_id}
              </Badge>
            )}
          </div>

          {/* Hex 预览 */}
          {receivedData?.data && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">{ll.viewer.tcp.label_hex_preview()}</div>
              <HexViewer data={dataToBytes(receivedData.data)} height={300} />
            </div>
          )}

          {/* 无数据提示 */}
          {(!receivedData?.data) && (
            <p className="text-sm text-muted-foreground italic">{ll.viewer.tcp.no_received_data()}</p>
          )}
        </div>
      </CollapsibleSection>

      {/* 执行状态 */}
      <CollapsibleSection
        title={ll.viewer.common.section_execution_status()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.tcp.status_received()} failText={ll.viewer.tcp.status_receive_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? sourceAddr
                  ? ll.viewer.tcp.udp_recv_success_from({ source: sourceAddr })
                  : ll.viewer.tcp.udp_recv_success()
                : output?.error ?? ll.viewer.tcp.status_receive_failed()}
            </span>
          </div>
          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 耗时 */}
      {totalDuration !== undefined && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <span className="text-xs font-mono text-orange-600 font-medium">
              {ll.viewer.common.total_duration_ms({ ms: totalDuration })}
            </span>
          }
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_total()} value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// UDP Close 查看器
// ============================================================================

function UdpCloseSection({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as UdpCloseOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 关闭配置 */}
      <CollapsibleSection
        title={ll.viewer.tcp.section_close_config()}
        icon={<Unplug className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <ProtocolBadge protocol="UDP" />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {output?.connection_id
                ? output.connection_id
                : <span className="text-muted-foreground italic">{ll.viewer.tcp.no_connection_id()}</span>}
            </code>
          </div>
        </div>
      </CollapsibleSection>

      {/* 关闭状态 */}
      <CollapsibleSection
        title={ll.viewer.tcp.section_close_status()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.tcp.status_closed()} failText={ll.viewer.tcp.status_close_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.tcp.udp_close_success()
                : output?.error ?? ll.viewer.tcp.close_error()}
            </span>
          </div>
          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 耗时 */}
      {totalDuration !== undefined && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <span className="text-xs font-mono text-orange-600 font-medium">
              {ll.viewer.common.total_duration_ms({ ms: totalDuration })}
            </span>
          }
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_total()} value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
