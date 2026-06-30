/**
 * MqttViewer - MQTT 节点执行详情查看器
 * 结构化展示 MQTT Connect/Publish/Subscribe/Unsubscribe/Disconnect/WaitMessage/Request 七种节点的执行结果
 * Connect: 连接状态、Broker 信息、耗时
 * Publish: 发布配置、发布状态、耗时
 * Subscribe: 订阅配置、消息列表、耗时
 * Unsubscribe: 取消订阅状态
 * Disconnect: 断开连接状态
 * WaitMessage: 等待消息配置、匹配消息列表、耗时
 * Request: 请求-响应配置、响应消息、耗时
 */
import { useState, memo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Cable,
  Send,
  Inbox,
  X,
  Unplug,
  Clock,
  Globe,
  MessageSquare,
  Radio,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { VariableChangesCard } from './VariableChangesCard';
import { getLL } from '@/i18n/i18n-lazy';

interface MqttViewerProps {
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

// ============================================================================
// 公共组件
// ============================================================================

/** MQTT 协议标签 */
function MqttBadge() {
  return (
    <Badge
      variant="outline"
      className="font-mono text-xs text-teal-600 dark:text-teal-400 bg-teal-50 dark:bg-teal-950/50 border-0 shrink-0"
    >
      MQTT
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
// MQTT 输出数据类型
// ============================================================================

/** MQTT Connect 输出数据 */
interface MqttConnectOutput {
  success?: boolean;
  connection_id?: string | null;
  broker_info?: {
    version?: string;
    session_present?: boolean;
    client_id?: string;
    keep_alive?: number;
  };
  timing?: {
    connect_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/** MQTT Publish 输出数据 */
interface MqttPublishOutput {
  success?: boolean;
  topic?: string;
  qos?: number;
  retain?: boolean;
  payload_size?: number;
  reason_code?: string;
  timing?: {
    publish_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/** MQTT 单条消息 */
interface MqttMessage {
  topic?: string;
  payload?: string;
  qos?: number;
  retain?: boolean;
  timestamp?: number;
}

/** MQTT Subscribe 输出数据 */
interface MqttSubscribeOutput {
  success?: boolean;
  topic?: string;
  qos?: number;
  messages?: MqttMessage[];
  message_count?: number;
  timing?: {
    subscribe_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/** MQTT Unsubscribe 输出数据 */
interface MqttUnsubscribeOutput {
  success?: boolean;
  topic?: string;
  error?: string;
}

/** MQTT Disconnect 输出数据 */
interface MqttDisconnectOutput {
  success?: boolean;
  reason?: string;
  timing?: {
    disconnect_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/** MQTT WaitMessage 输出数据 */
interface MqttWaitMessageOutput {
  success?: boolean;
  topic_filter?: string;
  matched_messages?: MqttMessage[];
  matched_count?: number;
  timeout_ms?: number;
  timing?: {
    wait_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/** MQTT Request 输出数据 */
interface MqttRequestOutput {
  success?: boolean;
  request_topic?: string;
  response_topic?: string;
  correlation_data?: string;
  responses?: MqttMessage[];
  response_count?: number;
  timing?: {
    request_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

// ============================================================================
// 消息卡片组件
// ============================================================================

function MqttMessageCard({ message, index }: { message: MqttMessage; index: number }) {
  const payload = message.payload ?? '';
  const truncatedPayload = payload.length > 200 ? payload.slice(0, 200) + '...' : payload;

  return (
    <div className="border border-border rounded-lg p-2.5">
      {/* 消息头部 */}
      <div className="flex items-center gap-2 mb-1.5">
        <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono">
          #{index + 1}
        </Badge>
        {message.topic && (
          <code className="text-xs font-mono text-foreground break-all">
            {message.topic}
          </code>
        )}
        {message.qos !== undefined && (
          <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono">
            QoS {message.qos}
          </Badge>
        )}
        {message.retain && (
          <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono text-amber-600 dark:text-amber-400 border-amber-300 dark:border-amber-700">
            Retain
          </Badge>
        )}
        {message.timestamp !== undefined && (
          <span className="text-[10px] text-muted-foreground font-mono ml-auto">
            {message.timestamp}ms
          </span>
        )}
      </div>

      {/* Payload 预览 */}
      {payload && (
        <pre className="text-xs font-mono text-foreground whitespace-pre-wrap break-all max-h-40 overflow-auto bg-muted rounded p-2">
          {truncatedPayload}
        </pre>
      )}
    </div>
  );
}

// ============================================================================
// 主组件
// ============================================================================

export const MqttViewer = memo(function MqttViewer({ report }: MqttViewerProps) {
  const nodeType = report.node_type;
  const ll = getLL();

  switch (nodeType) {
    case 'mqtt_connect':
      return <MqttConnectView report={report} />;
    case 'mqtt_publish':
      return <MqttPublishView report={report} />;
    case 'mqtt_subscribe':
      return <MqttSubscribeView report={report} />;
    case 'mqtt_unsubscribe':
      return <MqttUnsubscribeView report={report} />;
    case 'mqtt_disconnect':
      return <MqttDisconnectView report={report} />;
    case 'mqtt_wait_message':
      return <MqttWaitMessageView report={report} />;
    case 'mqtt_request':
      return <MqttRequestView report={report} />;
    default:
      return (
        <div className="p-4 text-sm text-muted-foreground italic">
          {ll.viewer.mqtt.unknown_node_type({ type: nodeType })}
    </div>
  );
  }
});

// ============================================================================
// MQTT Connect 查看器
// ============================================================================

function MqttConnectView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as MqttConnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const brokerInfo = output?.broker_info;
  const connectDuration = output?.timing?.connect_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 连接状态 */}
      <CollapsibleSection
        title={ll.viewer.mqtt.section_connection()}
        icon={<Cable className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.mqtt.status_connected()} failText={ll.viewer.mqtt.status_connection_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.mqtt.connect_success()
                : output?.error ?? ll.viewer.mqtt.connect_error()}
            </span>
          </div>

          {/* 连接 ID */}
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

      {/* Broker 信息 */}
      {isSuccess && brokerInfo && (
        <CollapsibleSection
          title={ll.viewer.mqtt.section_broker_info()}
          icon={<Globe className="w-4 h-4" />}
          defaultOpen={true}
        >
          <div className="flex flex-wrap gap-2">
            {brokerInfo.version && (
              <Badge variant="outline" className="font-mono text-xs">
                版本: {brokerInfo.version}
              </Badge>
            )}
            {brokerInfo.session_present !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                会话恢复: {brokerInfo.session_present ? '是' : '否'}
              </Badge>
            )}
            {brokerInfo.client_id && (
              <Badge variant="outline" className="font-mono text-xs">
                Client ID: {brokerInfo.client_id}
              </Badge>
            )}
            {brokerInfo.keep_alive !== undefined && (
              <Badge variant="secondary" className="font-mono text-xs">
                Keep Alive: {brokerInfo.keep_alive}s
              </Badge>
            )}
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(connectDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-teal-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_connect()} value={connectDuration} />
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
// MQTT Publish 查看器
// ============================================================================

function MqttPublishView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as MqttPublishOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const publishDuration = output?.timing?.publish_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 发布配置 */}
      <CollapsibleSection
        title={ll.viewer.mqtt.section_publish_config()}
        icon={<Send className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* Topic */}
          <div className="flex items-center gap-2">
            <MqttBadge />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {output?.topic || <span className="text-muted-foreground italic">未配置 Topic</span>}
            </code>
          </div>

          {/* QoS / Retain / Payload Size */}
          <div className="flex flex-wrap gap-2">
            {output?.qos !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                QoS: {output.qos}
              </Badge>
            )}
            {output?.retain !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.mqtt.label_retain()}: {output.retain ? '是' : '否'}
              </Badge>
            )}
            {output?.payload_size !== undefined && (
              <Badge variant="secondary" className="font-mono text-xs">
                载荷大小: {output.payload_size} 字节
              </Badge>
            )}
          </div>
        </div>
      </CollapsibleSection>

      {/* 发布状态 */}
      <CollapsibleSection
        title={ll.viewer.mqtt.section_publish_status()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.mqtt.status_published()} failText={ll.viewer.mqtt.status_publish_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `消息已成功发布到 ${output?.topic ?? 'topic'}`
                : output?.error ?? ll.viewer.mqtt.publish_error()}
            </span>
          </div>

          {/* Reason Code */}
          {isSuccess && output?.reason_code && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">Reason Code:</span>
              <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                {output.reason_code}
              </code>
            </div>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 耗时分解 */}
      {(publishDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-teal-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_publish()} value={publishDuration} />
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
// MQTT Subscribe 查看器
// ============================================================================

function MqttSubscribeView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as MqttSubscribeOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const messages = output?.messages ?? [];
  const messageCount = output?.message_count ?? messages.length;
  const subscribeDuration = output?.timing?.subscribe_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 订阅配置 */}
      <CollapsibleSection
        title={ll.viewer.mqtt.section_subscribe_config()}
        icon={<Inbox className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* Topic */}
          <div className="flex items-center gap-2">
            <MqttBadge />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {output?.topic || <span className="text-muted-foreground italic">未配置 Topic</span>}
            </code>
          </div>

          {output?.qos !== undefined && (
            <div className="flex flex-wrap gap-2">
              <Badge variant="outline" className="font-mono text-xs">
                QoS: {output.qos}
              </Badge>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 订阅状态 */}
      <CollapsibleSection
        title={ll.viewer.mqtt.section_subscription()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.mqtt.status_subscribed()} failText={ll.viewer.mqtt.status_subscribe_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `订阅成功，收到 ${messageCount} 条消息`
                : output?.error ?? ll.viewer.mqtt.subscribe_error()}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 消息列表 */}
      {messages.length > 0 && (
        <CollapsibleSection
        title={ll.viewer.mqtt.section_received_messages()}
        icon={<MessageSquare className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <Badge variant="outline" className="font-mono text-xs">
              {messageCount} 条消息
            </Badge>
          }
        >
          <div className="space-y-2">
            {messages.map((msg, idx) => (
              <MqttMessageCard key={idx} message={msg} index={idx} />
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(subscribeDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-teal-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_subscribe()} value={subscribeDuration} />
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
// MQTT Unsubscribe 查看器
// ============================================================================

function MqttUnsubscribeView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as MqttUnsubscribeOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      {/* 取消订阅配置 */}
      <CollapsibleSection
        title={ll.viewer.mqtt.section_unsubscribe_config()}
        icon={<X className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="flex items-center gap-2">
          <MqttBadge />
          <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
            {output?.topic || <span className="text-muted-foreground italic">未配置 Topic</span>}
          </code>
        </div>
      </CollapsibleSection>

      {/* 取消订阅状态 */}
      <CollapsibleSection
        title={ll.viewer.mqtt.section_unsubscribe_status()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.mqtt.status_unsubscribed()} failText={ll.viewer.mqtt.status_unsubscribe_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `已成功取消订阅 ${output?.topic ?? 'topic'}`
                : output?.error ?? ll.viewer.mqtt.unsubscribe_error()}
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

// ============================================================================
// MQTT Disconnect 查看器
// ============================================================================

function MqttDisconnectView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as MqttDisconnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const disconnectDuration = output?.timing?.disconnect_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 断开状态 */}
      <CollapsibleSection
        title={ll.viewer.mqtt.section_disconnect()}
        icon={<Unplug className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.mqtt.status_disconnected()} failText={ll.viewer.mqtt.status_disconnect_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? output?.reason
                  ? `已断开连接: ${output.reason}`
                  : ll.viewer.mqtt.disconnect_success()
                : output?.error ?? ll.viewer.mqtt.disconnect_error()}
            </span>
          </div>

          {/* 断开原因 */}
          {isSuccess && output?.reason && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">原因:</span>
              <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                {output.reason}
              </code>
            </div>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 耗时分解 */}
      {(disconnectDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-teal-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_disconnect()} value={disconnectDuration} />
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
// MQTT WaitMessage 查看器
// ============================================================================

function MqttWaitMessageView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as MqttWaitMessageOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const matchedMessages = output?.matched_messages ?? [];
  const matchedCount = output?.matched_count ?? matchedMessages.length;
  const waitDuration = output?.timing?.wait_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 等待配置 */}
      <CollapsibleSection
        title={ll.viewer.mqtt.section_wait_config()}
        icon={<Radio className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* Topic Filter */}
          <div className="flex items-center gap-2">
            <MqttBadge />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {output?.topic_filter || <span className="text-muted-foreground italic">未配置 Topic Filter</span>}
            </code>
          </div>

          {output?.timeout_ms !== undefined && (
            <div className="flex flex-wrap gap-2">
              <Badge variant="secondary" className="font-mono text-xs">
                超时: {output.timeout_ms}ms
              </Badge>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 等待状态 */}
      <CollapsibleSection
        title={ll.viewer.mqtt.section_wait_status()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.mqtt.status_wait_success()} failText={ll.viewer.mqtt.status_wait_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `匹配到 ${matchedCount} 条消息`
                : output?.error ?? ll.viewer.mqtt.status_wait_failed()}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 匹配消息列表 */}
      {matchedMessages.length > 0 && (
        <CollapsibleSection
          title={ll.viewer.mqtt.section_matched_messages()}
          icon={<MessageSquare className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <Badge variant="outline" className="font-mono text-xs">
              {matchedCount} 条消息
            </Badge>
          }
        >
          <div className="space-y-2">
            {matchedMessages.map((msg, idx) => (
              <MqttMessageCard key={idx} message={msg} index={idx} />
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(waitDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-teal-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_wait()} value={waitDuration} />
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
// MQTT Request 查看器
// ============================================================================

function MqttRequestView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as MqttRequestOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const responses = output?.responses ?? [];
  const responseCount = output?.response_count ?? responses.length;
  const requestDuration = output?.timing?.request_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 请求配置 */}
      <CollapsibleSection
        title={ll.viewer.mqtt.section_request_config()}
        icon={<Send className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* Request Topic */}
          <div className="flex items-center gap-2">
            <MqttBadge />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {output?.request_topic || <span className="text-muted-foreground italic">未配置请求 Topic</span>}
            </code>
          </div>

          {/* Response Topic / Correlation Data */}
          <div className="flex flex-wrap gap-2">
            {output?.response_topic && (
              <Badge variant="outline" className="font-mono text-xs">
                响应 Topic: {output.response_topic}
              </Badge>
            )}
            {output?.correlation_data && (
              <Badge variant="outline" className="font-mono text-xs">
                Correlation: {output.correlation_data}
              </Badge>
            )}
          </div>
        </div>
      </CollapsibleSection>

      {/* 请求状态 */}
      <CollapsibleSection
        title={ll.viewer.mqtt.section_request_status()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.mqtt.status_request_success()} failText={ll.viewer.mqtt.status_request_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `请求-响应完成，收到 ${responseCount} 条响应`
                : output?.error ?? ll.viewer.mqtt.status_request_failed()}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 响应消息列表 */}
      {responses.length > 0 && (
        <CollapsibleSection
          title={ll.viewer.mqtt.section_response()}
          icon={<MessageSquare className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <Badge variant="outline" className="font-mono text-xs">
              {responseCount} 条响应
            </Badge>
          }
        >
          <div className="space-y-2">
            {responses.map((msg, idx) => (
              <MqttMessageCard key={idx} message={msg} index={idx} />
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(requestDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-teal-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_request()} value={requestDuration} />
            <TimingMetric label={ll.viewer.common.timing_total()} value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
