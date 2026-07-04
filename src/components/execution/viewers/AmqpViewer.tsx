/**
 * AmqpViewer - AMQP 节点执行详情查看器
 * 结构化展示 AMQP 0-9-1 和 AMQP 1.0 共 18 种节点的执行结果
 *
 * AMQP 0-9-1:
 *   Connect: 连接状态、服务器属性、连接参数
 *   Close: 关闭状态
 *   ExchangeDeclare/Delete: 交换机操作结果
 *   QueueDeclare: 队列声明结果（消息数、消费者数）
 *   QueueDelete: 队列删除结果（已删除消息数）
 *   QueueBind: 绑定结果
 *   Publish: 发布状态、确认状态
 *   Consume: 消息列表（属性、Body、投递信息）
 *   Get: 单条消息详情
 *   Ack: 确认状态
 *   Management: 管理 API 响应
 *
 * AMQP 1.0:
 *   Connect: 连接状态、容器 ID
 *   Send: 发送状态、投递状态
 *   Receive: 消息列表（application-properties、Body）
 *   Accept: 接受状态
 *   Reject: 拒绝状态
 *   Disconnect: 断开状态
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
  Globe,
  MessageSquare,
  Server,
  Layers,
  Check,
  Ban,
  Settings,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { VariableChangesCard } from './VariableChangesCard';
import { RawJsonViewer } from './RawJsonViewer';
import { getLL } from '@/i18n/i18n-lazy';

interface AmqpViewerProps {
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

/** AMQP 协议标签 */
function AmqpBadge() {
  return (
    <Badge
      variant="outline"
      className="font-mono text-xs text-orange-600 dark:text-orange-400 bg-orange-50 dark:bg-orange-950/50 border-0 shrink-0"
    >
      AMQP
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

/** 键值对行 */
function KvRow({ label, value }: { label: string; value: unknown }) {
  if (value === undefined || value === null) return null;
  return (
    <div className="flex items-center gap-2 text-sm">
      <span className="text-muted-foreground shrink-0">{label}:</span>
      <span className="font-mono text-xs text-foreground break-all">{String(value)}</span>
    </div>
  );
}

// ============================================================================
// AMQP 输出数据类型
// ============================================================================

/** AMQP Connect 输出数据 */
interface AmqpConnectOutput {
  success?: boolean;
  connection_id?: string | null;
  server_properties?: {
    product?: string;
    version?: string;
    host?: string;
    [key: string]: unknown;
  };
  channel_max?: number;
  frame_max?: number;
  error?: string;
}

/** AMQP Close 输出数据 */
interface AmqpCloseOutput {
  success?: boolean;
  error?: string;
}

/** AMQP Exchange 操作输出数据 */
interface AmqpExchangeOutput {
  success?: boolean;
  exchange?: string;
  exchange_type?: string;
  error?: string;
}

/** AMQP QueueDeclare 输出数据 */
interface AmqpQueueDeclareOutput {
  success?: boolean;
  queue?: string;
  message_count?: number;
  consumer_count?: number;
  error?: string;
}

/** AMQP QueueDelete 输出数据 */
interface AmqpQueueDeleteOutput {
  success?: boolean;
  queue?: string;
  deleted_message_count?: number;
  error?: string;
}

/** AMQP QueueBind 输出数据 */
interface AmqpQueueBindOutput {
  success?: boolean;
  queue?: string;
  exchange?: string;
  routing_key?: string;
  error?: string;
}

/** AMQP Publish 输出数据 */
interface AmqpPublishOutput {
  success?: boolean;
  exchange?: string;
  routing_key?: string;
  confirm_status?: string;
  message_id?: string | null;
  error?: string;
}

/** AMQP 消息属性 */
interface AmqpMessageProperties {
  content_type?: string;
  content_encoding?: string;
  delivery_mode?: number;
  priority?: number;
  correlation_id?: string;
  reply_to?: string;
  expiration?: string;
  message_id?: string;
  timestamp?: number;
  type?: string;
  user_id?: string;
  app_id?: string;
  headers?: Record<string, unknown>;
  [key: string]: unknown;
}

/** AMQP 投递信息 */
interface AmqpDeliveryInfo {
  delivery_tag?: number;
  redelivered?: boolean;
  exchange?: string;
  routing_key?: string;
  consumer_tag?: string;
}

/** AMQP 单条消息 */
interface AmqpMessage {
  properties?: AmqpMessageProperties;
  body?: unknown;
  delivery_info?: AmqpDeliveryInfo;
}

/** AMQP Consume 输出数据 */
interface AmqpConsumeOutput {
  success?: boolean;
  messages?: AmqpMessage[];
  message_count?: number;
  duration_ms?: number;
  error?: string;
}

/** AMQP Get 输出数据 */
interface AmqpGetOutput {
  success?: boolean;
  message?: AmqpMessage;
  message_count?: number;
  error?: string;
}

/** AMQP Ack 输出数据 */
interface AmqpAckOutput {
  success?: boolean;
  delivery_tag?: number;
  error?: string;
}

/** AMQP Management 输出数据 */
interface AmqpManagementOutput {
  success?: boolean;
  response?: unknown;
  overview?: {
    connection_count?: number;
    channel_count?: number;
    queue_count?: number;
    message_stats?: {
      publish?: number;
      deliver?: number;
      ack?: number;
      [key: string]: unknown;
    };
    [key: string]: unknown;
  };
  error?: string;
}

/** AMQP 1.0 Connect 输出数据 */
interface Amqp10ConnectOutput {
  success?: boolean;
  connection_id?: string | null;
  container_id?: string;
  session_info?: string;
  error?: string;
}

/** AMQP 1.0 Send 输出数据 */
interface Amqp10SendOutput {
  success?: boolean;
  delivery_state?: string;
  message_id?: string | null;
  error?: string;
}

/** AMQP 1.0 消息 */
interface Amqp10Message {
  application_properties?: Record<string, unknown>;
  message_annotations?: Record<string, unknown>;
  body?: unknown;
  delivery_id?: number;
  message_id?: string;
}

/** AMQP 1.0 Receive 输出数据 */
interface Amqp10ReceiveOutput {
  success?: boolean;
  messages?: Amqp10Message[];
  message_count?: number;
  duration_ms?: number;
  error?: string;
}

/** AMQP 1.0 Accept 输出数据 */
interface Amqp10AcceptOutput {
  success?: boolean;
  error?: string;
}

/** AMQP 1.0 Reject 输出数据 */
interface Amqp10RejectOutput {
  success?: boolean;
  error?: string;
}

/** AMQP 1.0 Disconnect 输出数据 */
interface Amqp10DisconnectOutput {
  success?: boolean;
  error?: string;
}

// ============================================================================
// 消息卡片组件
// ============================================================================

/** 尝试格式化消息体 */
function MessageBody({ body }: { body: unknown }) {
  if (body === undefined || body === null) return null;

  return (
    <div className="bg-muted rounded p-2 max-h-40 overflow-auto">
      <RawJsonViewer data={body} />
    </div>
  );
}

/** AMQP 0-9-1 消息属性展示 */
function AmqpMessagePropertiesView({ properties }: { properties?: AmqpMessageProperties }) {
  if (!properties) return null;

  const entries = Object.entries(properties).filter(
    ([, v]) => v !== undefined && v !== null
  );
  if (entries.length === 0) return null;

  return (
    <div className="grid grid-cols-2 gap-1.5 text-xs">
      {entries.map(([key, value]) => (
        <div key={key} className="flex items-center gap-1.5">
          <span className="text-muted-foreground shrink-0">{key}:</span>
          <code className="font-mono text-foreground break-all">
            {typeof value === 'object' ? JSON.stringify(value) : String(value)}
          </code>
        </div>
      ))}
    </div>
  );
}

/** AMQP 0-9-1 消息卡片 */
function AmqpMessageCard({ message, index }: { message: AmqpMessage; index: number }) {
  return (
    <div className="border border-border rounded-lg p-2.5">
      {/* 消息头部 */}
      <div className="flex items-center gap-2 mb-1.5">
        <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono">
          #{index + 1}
        </Badge>
        {message.delivery_info?.routing_key && (
          <code className="text-xs font-mono text-foreground break-all">
            {message.delivery_info.routing_key}
          </code>
        )}
        {message.delivery_info?.exchange && (
          <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono">
            {message.delivery_info.exchange}
          </Badge>
        )}
        {message.delivery_info?.delivery_tag !== undefined && (
          <span className="text-[10px] text-muted-foreground font-mono ml-auto">
            tag: {message.delivery_info.delivery_tag}
          </span>
        )}
      </div>

      {/* 消息属性 */}
      {message.properties && <AmqpMessagePropertiesView properties={message.properties} />}

      {/* 消息体 */}
      {message.body !== undefined && message.body !== null && (
        <div className="mt-1.5">
          <MessageBody body={message.body} />
        </div>
      )}
    </div>
  );
}

/** AMQP 1.0 消息卡片 */
function Amqp10MessageCard({ message, index }: { message: Amqp10Message; index: number }) {
  const ll = getLL();
  return (
    <div className="border border-border rounded-lg p-2.5">
      {/* 消息头部 */}
      <div className="flex items-center gap-2 mb-1.5">
        <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono">
          #{index + 1}
        </Badge>
        {message.message_id && (
          <code className="text-xs font-mono text-foreground break-all">
            {message.message_id}
          </code>
        )}
        {message.delivery_id !== undefined && (
          <span className="text-[10px] text-muted-foreground font-mono ml-auto">
            delivery: {message.delivery_id}
          </span>
        )}
      </div>

      {/* application-properties */}
      {message.application_properties && Object.keys(message.application_properties).length > 0 && (
        <div className="mb-1.5">
          <span className="text-[10px] text-muted-foreground font-medium">{ll.viewer.amqp.label_application_properties()}:</span>
          <div className="grid grid-cols-2 gap-1 text-xs mt-0.5">
            {Object.entries(message.application_properties).map(([key, value]) => (
              <div key={key} className="flex items-center gap-1.5">
                <span className="text-muted-foreground shrink-0">{key}:</span>
                <code className="font-mono text-foreground">{String(value)}</code>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* message-annotations */}
      {message.message_annotations && Object.keys(message.message_annotations).length > 0 && (
        <div className="mb-1.5">
          <span className="text-[10px] text-muted-foreground font-medium">{ll.viewer.amqp.label_message_annotations()}:</span>
          <div className="grid grid-cols-2 gap-1 text-xs mt-0.5">
            {Object.entries(message.message_annotations).map(([key, value]) => (
              <div key={key} className="flex items-center gap-1.5">
                <span className="text-muted-foreground shrink-0">{key}:</span>
                <code className="font-mono text-foreground">{String(value)}</code>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 消息体 */}
      {message.body !== undefined && message.body !== null && (
        <MessageBody body={message.body} />
      )}
    </div>
  );
}

// ============================================================================
// 主组件
// ============================================================================

export const AmqpViewer = memo(function AmqpViewer({ report }: AmqpViewerProps) {
  const ll = getLL();
  const nodeType = report.node_type;

  switch (nodeType) {
    // AMQP 0-9-1
    case 'amqp_connect':
      return <AmqpConnectView report={report} />;
    case 'amqp_close':
      return <AmqpCloseView report={report} />;
    case 'amqp_exchange_declare':
    case 'amqp_exchange_delete':
      return <AmqpExchangeView report={report} />;
    case 'amqp_queue_declare':
      return <AmqpQueueDeclareView report={report} />;
    case 'amqp_queue_delete':
      return <AmqpQueueDeleteView report={report} />;
    case 'amqp_queue_bind':
      return <AmqpQueueBindView report={report} />;
    case 'amqp_publish':
      return <AmqpPublishView report={report} />;
    case 'amqp_consume':
      return <AmqpConsumeView report={report} />;
    case 'amqp_get':
      return <AmqpGetView report={report} />;
    case 'amqp_ack':
      return <AmqpAckView report={report} />;
    case 'amqp_management':
      return <AmqpManagementView report={report} />;
    // AMQP 1.0
    case 'amqp10_connect':
      return <Amqp10ConnectView report={report} />;
    case 'amqp10_send':
      return <Amqp10SendView report={report} />;
    case 'amqp10_receive':
      return <Amqp10ReceiveView report={report} />;
    case 'amqp10_accept':
      return <Amqp10AcceptView report={report} />;
    case 'amqp10_reject':
      return <Amqp10RejectView report={report} />;
    case 'amqp10_disconnect':
      return <Amqp10DisconnectView report={report} />;
    default:
      return (
        <div className="p-4 text-sm text-muted-foreground italic">
          {ll.viewer.amqp.unknown_node_type({ type: nodeType })}
        </div>
      );
  }
});

// ============================================================================
// AMQP 0-9-1 Connect 查看器
// ============================================================================

function AmqpConnectView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as AmqpConnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const serverProps = output?.server_properties;

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_connection()}
        icon={<Cable className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_connected()} failText={ll.viewer.amqp.status_connection_failed()} />
            <AmqpBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.amqp.connect_success()
                : output?.error ?? ll.viewer.amqp.connect_error()}
            </span>
          </div>

          {output?.connection_id && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{ll.viewer.amqp.label_connection_id()}:</span>
              <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                {output.connection_id}
              </code>
            </div>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {isSuccess && serverProps && (
        <CollapsibleSection
          title={ll.viewer.amqp.section_server_properties()}
          icon={<Server className="w-4 h-4" />}
          defaultOpen={true}
        >
          <div className="flex flex-wrap gap-2">
            {serverProps.product && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.amqp.label_server_product()}: {serverProps.product}
              </Badge>
            )}
            {serverProps.version && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.amqp.label_server_version()}: {serverProps.version}
              </Badge>
            )}
            {serverProps.host && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.amqp.label_server_host()}: {serverProps.host}
              </Badge>
            )}
            {output?.channel_max !== undefined && (
              <Badge variant="secondary" className="font-mono text-xs">
                {ll.viewer.amqp.label_channel_max()}: {output.channel_max}
              </Badge>
            )}
            {output?.frame_max !== undefined && (
              <Badge variant="secondary" className="font-mono text-xs">
                {ll.viewer.amqp.label_frame_max()}: {output.frame_max}
              </Badge>
            )}
          </div>
        </CollapsibleSection>
      )}

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 0-9-1 Close 查看器
// ============================================================================

function AmqpCloseView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as AmqpCloseOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_close()}
        icon={<Unplug className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_closed()} failText={ll.viewer.amqp.status_close_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.amqp.close_success()
                : output?.error ?? ll.viewer.amqp.close_error()}
            </span>
          </div>
          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 0-9-1 Exchange 操作查看器
// ============================================================================

function AmqpExchangeView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as AmqpExchangeOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const isDeclare = report.node_type === 'amqp_exchange_declare';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={isDeclare ? ll.viewer.amqp.section_exchange() : ll.viewer.amqp.section_exchange_delete()}
        icon={<Layers className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge
              isSuccess={isSuccess}
              successText={isDeclare ? ll.viewer.amqp.status_declared() : ll.viewer.amqp.status_deleted()}
              failText={isDeclare ? ll.viewer.amqp.status_declare_failed() : ll.viewer.amqp.status_delete_failed()}
            />
            <AmqpBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? isDeclare
                  ? ll.viewer.amqp.exchange_declare_success({ name: output?.exchange ?? '' })
                  : ll.viewer.amqp.exchange_delete_success({ name: output?.exchange ?? '' })
                : output?.error ?? (isDeclare ? ll.viewer.amqp.exchange_declare_error() : ll.viewer.amqp.exchange_delete_error())}
            </span>
          </div>

          {output?.exchange && (
            <KvRow label={ll.viewer.amqp.label_exchange()} value={output.exchange} />
          )}
          {output?.exchange_type && (
            <KvRow label={ll.viewer.amqp.label_exchange_type()} value={output.exchange_type} />
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 0-9-1 QueueDeclare 查看器
// ============================================================================

function AmqpQueueDeclareView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as AmqpQueueDeclareOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_queue_declare()}
        icon={<Inbox className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_declared()} failText={ll.viewer.amqp.status_declare_failed()} />
            <AmqpBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.amqp.queue_declare_success({ name: output?.queue ?? '' })
                : output?.error ?? ll.viewer.amqp.queue_declare_error()}
            </span>
          </div>

          {output?.queue && (
            <KvRow label={ll.viewer.amqp.label_queue_name()} value={output.queue} />
          )}

          {isSuccess && (
            <div className="flex flex-wrap gap-2">
              {output?.message_count !== undefined && (
                <Badge variant="outline" className="font-mono text-xs">
                  {ll.viewer.amqp.label_message_count()}: {output.message_count}
                </Badge>
              )}
              {output?.consumer_count !== undefined && (
                <Badge variant="outline" className="font-mono text-xs">
                  {ll.viewer.amqp.label_consumer_count()}: {output.consumer_count}
                </Badge>
              )}
            </div>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 0-9-1 QueueDelete 查看器
// ============================================================================

function AmqpQueueDeleteView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as AmqpQueueDeleteOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_queue_delete()}
        icon={<X className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_deleted()} failText={ll.viewer.amqp.status_delete_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.amqp.queue_delete_success({ name: output?.queue ?? '' })
                : output?.error ?? ll.viewer.amqp.queue_delete_error()}
            </span>
          </div>

          {output?.queue && (
            <KvRow label={ll.viewer.amqp.label_queue_name()} value={output.queue} />
          )}

          {isSuccess && output?.deleted_message_count !== undefined && (
            <Badge variant="secondary" className="font-mono text-xs">
              {ll.viewer.amqp.label_deleted_messages()}: {output.deleted_message_count}
            </Badge>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 0-9-1 QueueBind 查看器
// ============================================================================

function AmqpQueueBindView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as AmqpQueueBindOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_queue_bind()}
        icon={<Layers className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_bound()} failText={ll.viewer.amqp.status_bind_failed()} />
            <AmqpBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.amqp.queue_bind_success()
                : output?.error ?? ll.viewer.amqp.queue_bind_error()}
            </span>
          </div>

          {output?.queue && <KvRow label={ll.viewer.amqp.label_queue()} value={output.queue} />}
          {output?.exchange && <KvRow label={ll.viewer.amqp.label_exchange()} value={output.exchange} />}
          {output?.routing_key && <KvRow label={ll.viewer.amqp.label_routing_key()} value={output.routing_key} />}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 0-9-1 Publish 查看器
// ============================================================================

function AmqpPublishView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as AmqpPublishOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_publish_config()}
        icon={<Send className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <AmqpBadge />
            {output?.routing_key ? (
              <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
                {output.routing_key}
              </code>
            ) : (
              <span className="text-sm text-muted-foreground italic">{ll.viewer.amqp.no_routing_key()}</span>
            )}
          </div>

          <div className="flex flex-wrap gap-2">
            {output?.exchange && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.amqp.label_exchange()}: {output.exchange}
              </Badge>
            )}
          </div>
        </div>
      </CollapsibleSection>

      <CollapsibleSection
        title={ll.viewer.amqp.section_publish_status()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_published()} failText={ll.viewer.amqp.status_publish_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.amqp.publish_success()
                : output?.error ?? ll.viewer.amqp.publish_error()}
            </span>
          </div>

          {isSuccess && output?.confirm_status && (
            <KvRow label={ll.viewer.amqp.label_confirm_status()} value={output.confirm_status} />
          )}
          {isSuccess && output?.message_id && (
            <KvRow label={ll.viewer.amqp.label_message_id()} value={output.message_id} />
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 0-9-1 Consume 查看器
// ============================================================================

function AmqpConsumeView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as AmqpConsumeOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const messages = output?.messages ?? [];
  const messageCount = output?.message_count ?? messages.length;

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_consume_status()}
        icon={<Inbox className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_consume_complete()} failText={ll.viewer.amqp.status_consume_failed()} />
            <AmqpBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.amqp.consume_success({ count: messageCount })
                : output?.error ?? ll.viewer.amqp.consume_error()}
            </span>
          </div>

          {output?.duration_ms !== undefined && (
            <KvRow label={ll.viewer.amqp.label_duration()} value={`${output.duration_ms}ms`} />
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {messages.length > 0 && (
        <CollapsibleSection
          title={ll.viewer.amqp.section_message_list()}
          icon={<MessageSquare className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <Badge variant="outline" className="font-mono text-xs">
              {ll.viewer.amqp.messages_count({ count: messageCount })}
            </Badge>
          }
        >
          <div className="space-y-2">
            {messages.map((msg, idx) => (
              <AmqpMessageCard key={idx} message={msg} index={idx} />
            ))}
          </div>
        </CollapsibleSection>
      )}

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 0-9-1 Get 查看器
// ============================================================================

function AmqpGetView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as AmqpGetOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const message = output?.message;

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_get_status()}
        icon={<Inbox className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_fetched()} failText={ll.viewer.amqp.status_fetch_failed()} />
            <AmqpBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? message
                  ? ll.viewer.amqp.fetch_success()
                  : ll.viewer.amqp.fetch_empty()
                : output?.error ?? ll.viewer.amqp.fetch_error()}
            </span>
          </div>

          {output?.message_count !== undefined && (
            <KvRow label={ll.viewer.amqp.label_queue_messages()} value={output.message_count} />
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {message && (
        <CollapsibleSection
          title={ll.viewer.amqp.section_message_detail()}
          icon={<MessageSquare className="w-4 h-4" />}
          defaultOpen={true}
        >
          <AmqpMessageCard message={message} index={0} />
        </CollapsibleSection>
      )}

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 0-9-1 Ack 查看器
// ============================================================================

function AmqpAckView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as AmqpAckOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_ack_status()}
        icon={<Check className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_acknowledged()} failText={ll.viewer.amqp.status_ack_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? output?.delivery_tag !== undefined
                  ? ll.viewer.amqp.ack_success_with_tag({ tag: output.delivery_tag })
                  : ll.viewer.amqp.ack_success()
                : output?.error ?? ll.viewer.amqp.ack_error()}
            </span>
          </div>

          {output?.delivery_tag !== undefined && (
            <KvRow label={'Delivery Tag'} value={output.delivery_tag} />
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 0-9-1 Management 查看器
// ============================================================================

function AmqpManagementView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as AmqpManagementOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const overview = output?.overview;
  const hasResponse = output?.response !== undefined && output?.response !== null;

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_management()}
        icon={<Settings className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_operation_ok()} failText={ll.viewer.amqp.status_operation_failed()} />
            <AmqpBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.amqp.management_success()
                : output?.error ?? ll.viewer.amqp.management_error()}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {isSuccess && overview && (
        <CollapsibleSection
          title={ll.viewer.amqp.section_overview()}
          icon={<Globe className="w-4 h-4" />}
          defaultOpen={true}
        >
          <div className="flex flex-wrap gap-2">
            {overview.connection_count !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.amqp.label_connection_count()}: {overview.connection_count}
              </Badge>
            )}
            {overview.channel_count !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.amqp.label_channel_count()}: {overview.channel_count}
              </Badge>
            )}
            {overview.queue_count !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.amqp.label_queue_count()}: {overview.queue_count}
              </Badge>
            )}
            {overview.message_stats?.publish !== undefined && (
              <Badge variant="secondary" className="font-mono text-xs">
                {ll.viewer.amqp.label_publish_rate()}: {overview.message_stats.publish}
              </Badge>
            )}
            {overview.message_stats?.deliver !== undefined && (
              <Badge variant="secondary" className="font-mono text-xs">
                {ll.viewer.amqp.label_deliver_rate()}: {overview.message_stats.deliver}
              </Badge>
            )}
          </div>
        </CollapsibleSection>
      )}

      {hasResponse && (
        <CollapsibleSection
          title={ll.viewer.amqp.section_api_response()}
          icon={<Globe className="w-4 h-4" />}
          defaultOpen={false}
        >
          <div className="bg-muted rounded p-2 max-h-60 overflow-auto">
            <RawJsonViewer data={output!.response} />
          </div>
        </CollapsibleSection>
      )}

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 1.0 Connect 查看器
// ============================================================================

function Amqp10ConnectView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as Amqp10ConnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_connection()}
        icon={<Cable className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_connected()} failText={ll.viewer.amqp.status_connection_failed()} />
            <Badge
              variant="outline"
              className="font-mono text-xs text-orange-600 dark:text-orange-400 bg-orange-50 dark:bg-orange-950/50 border-0"
            >
              {ll.viewer.amqp.protocol_amqp10()}
            </Badge>
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.amqp.connect_success_10()
                : output?.error ?? ll.viewer.amqp.connect_error_10()}
            </span>
          </div>

          {output?.connection_id && (
            <KvRow label={ll.viewer.amqp.label_connection_id()} value={output.connection_id} />
          )}
          {output?.container_id && (
            <KvRow label={ll.viewer.amqp.label_container_id()} value={output.container_id} />
          )}
          {output?.session_info && (
            <KvRow label={ll.viewer.amqp.label_session_info()} value={output.session_info} />
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 1.0 Send 查看器
// ============================================================================

function Amqp10SendView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as Amqp10SendOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_send_status()}
        icon={<Send className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_sent()} failText={ll.viewer.amqp.status_send_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.amqp.send_success()
                : output?.error ?? ll.viewer.amqp.send_error()}
            </span>
          </div>

          {isSuccess && output?.delivery_state && (
            <KvRow label={ll.viewer.amqp.label_delivery_status()} value={output.delivery_state} />
          )}
          {isSuccess && output?.message_id && (
            <KvRow label={ll.viewer.amqp.label_message_id()} value={output.message_id} />
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 1.0 Receive 查看器
// ============================================================================

function Amqp10ReceiveView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as Amqp10ReceiveOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const messages = output?.messages ?? [];
  const messageCount = output?.message_count ?? messages.length;

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_receive_status()}
        icon={<Inbox className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_received()} failText={ll.viewer.amqp.status_receive_failed()} />
            <Badge
              variant="outline"
              className="font-mono text-xs text-orange-600 dark:text-orange-400 bg-orange-50 dark:bg-orange-950/50 border-0"
            >
              {ll.viewer.amqp.protocol_amqp10()}
            </Badge>
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.amqp.receive_success({ count: messageCount })
                : output?.error ?? ll.viewer.amqp.receive_error()}
            </span>
          </div>

          {output?.duration_ms !== undefined && (
            <KvRow label={ll.viewer.amqp.label_duration()} value={`${output.duration_ms}ms`} />
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {messages.length > 0 && (
        <CollapsibleSection
          title={ll.viewer.amqp.section_message_list()}
          icon={<MessageSquare className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <Badge variant="outline" className="font-mono text-xs">
              {ll.viewer.amqp.messages_count({ count: messageCount })}
            </Badge>
          }
        >
          <div className="space-y-2">
            {messages.map((msg, idx) => (
              <Amqp10MessageCard key={idx} message={msg} index={idx} />
            ))}
          </div>
        </CollapsibleSection>
      )}

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 1.0 Accept 查看器
// ============================================================================

function Amqp10AcceptView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as Amqp10AcceptOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_accept_status()}
        icon={<Check className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_accepted()} failText={ll.viewer.amqp.status_accept_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.amqp.accept_success()
                : output?.error ?? ll.viewer.amqp.accept_error()}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 1.0 Reject 查看器
// ============================================================================

function Amqp10RejectView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as Amqp10RejectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_reject_status()}
        icon={<Ban className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_rejected()} failText={ll.viewer.amqp.status_reject_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.amqp.reject_success()
                : output?.error ?? ll.viewer.amqp.reject_error()}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// AMQP 1.0 Disconnect 查看器
// ============================================================================

function Amqp10DisconnectView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as Amqp10DisconnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={ll.viewer.amqp.section_disconnect_status()}
        icon={<Unplug className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.amqp.status_disconnected()} failText={ll.viewer.amqp.status_disconnect_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.amqp.disconnect_success()
                : output?.error ?? ll.viewer.amqp.disconnect_error()}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
