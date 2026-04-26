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
import { useState } from 'react';
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
import { cn } from '@/lib/utils';
import { VariableChangesCard } from './VariableChangesCard';
import { RawJsonViewer } from './RawJsonViewer';

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
          <span className="text-[10px] text-muted-foreground font-medium">应用属性:</span>
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
          <span className="text-[10px] text-muted-foreground font-medium">消息注解:</span>
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

export function AmqpViewer({ report }: AmqpViewerProps) {
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
          未知的 AMQP 节点类型: {nodeType}
        </div>
      );
  }
}

// ============================================================================
// AMQP 0-9-1 Connect 查看器
// ============================================================================

function AmqpConnectView({ report }: { report: NodeExecutionReport }) {
  const output = report.output_data as AmqpConnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const serverProps = output?.server_properties;

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
            <AmqpBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? 'AMQP 连接建立成功'
                : output?.error ?? '无法建立 AMQP 连接'}
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

      {/* 服务器属性 */}
      {isSuccess && serverProps && (
        <CollapsibleSection
          title="服务器属性"
          icon={<Server className="w-4 h-4" />}
          defaultOpen={true}
        >
          <div className="flex flex-wrap gap-2">
            {serverProps.product && (
              <Badge variant="outline" className="font-mono text-xs">
                产品: {serverProps.product}
              </Badge>
            )}
            {serverProps.version && (
              <Badge variant="outline" className="font-mono text-xs">
                版本: {serverProps.version}
              </Badge>
            )}
            {serverProps.host && (
              <Badge variant="outline" className="font-mono text-xs">
                主机: {serverProps.host}
              </Badge>
            )}
            {output?.channel_max !== undefined && (
              <Badge variant="secondary" className="font-mono text-xs">
                最大通道数: {output.channel_max}
              </Badge>
            )}
            {output?.frame_max !== undefined && (
              <Badge variant="secondary" className="font-mono text-xs">
                最大帧大小: {output.frame_max}
              </Badge>
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
// AMQP 0-9-1 Close 查看器
// ============================================================================

function AmqpCloseView({ report }: { report: NodeExecutionReport }) {
  const output = report.output_data as AmqpCloseOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title="关闭状态"
        icon={<Unplug className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已关闭" failText="关闭失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? 'AMQP 连接已成功关闭'
                : output?.error ?? '关闭连接失败'}
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
  const output = report.output_data as AmqpExchangeOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const isDeclare = report.node_type === 'amqp_exchange_declare';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title={isDeclare ? '交换机声明' : '交换机删除'}
        icon={<Layers className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge
              isSuccess={isSuccess}
              successText={isDeclare ? '已声明' : '已删除'}
              failText={isDeclare ? '声明失败' : '删除失败'}
            />
            <AmqpBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? isDeclare
                  ? `交换机 ${output?.exchange ?? ''} 声明成功`
                  : `交换机 ${output?.exchange ?? ''} 删除成功`
                : output?.error ?? (isDeclare ? '交换机声明失败' : '交换机删除失败')}
            </span>
          </div>

          {output?.exchange && (
            <KvRow label="交换机" value={output.exchange} />
          )}
          {output?.exchange_type && (
            <KvRow label="类型" value={output.exchange_type} />
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
  const output = report.output_data as AmqpQueueDeclareOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title="队列声明"
        icon={<Inbox className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已声明" failText="声明失败" />
            <AmqpBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `队列 ${output?.queue ?? ''} 声明成功`
                : output?.error ?? '队列声明失败'}
            </span>
          </div>

          {output?.queue && (
            <KvRow label="队列名" value={output.queue} />
          )}

          {isSuccess && (
            <div className="flex flex-wrap gap-2">
              {output?.message_count !== undefined && (
                <Badge variant="outline" className="font-mono text-xs">
                  消息数: {output.message_count}
                </Badge>
              )}
              {output?.consumer_count !== undefined && (
                <Badge variant="outline" className="font-mono text-xs">
                  消费者数: {output.consumer_count}
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
  const output = report.output_data as AmqpQueueDeleteOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title="队列删除"
        icon={<X className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已删除" failText="删除失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `队列 ${output?.queue ?? ''} 删除成功`
                : output?.error ?? '队列删除失败'}
            </span>
          </div>

          {output?.queue && (
            <KvRow label="队列名" value={output.queue} />
          )}

          {isSuccess && output?.deleted_message_count !== undefined && (
            <Badge variant="secondary" className="font-mono text-xs">
              已删除消息数: {output.deleted_message_count}
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
  const output = report.output_data as AmqpQueueBindOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title="队列绑定"
        icon={<Layers className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已绑定" failText="绑定失败" />
            <AmqpBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? '队列绑定成功'
                : output?.error ?? '队列绑定失败'}
            </span>
          </div>

          {output?.queue && <KvRow label="队列" value={output.queue} />}
          {output?.exchange && <KvRow label="交换机" value={output.exchange} />}
          {output?.routing_key && <KvRow label="路由键" value={output.routing_key} />}

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
  const output = report.output_data as AmqpPublishOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      {/* 发布配置 */}
      <CollapsibleSection
        title="发布配置"
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
              <span className="text-sm text-muted-foreground italic">未配置路由键</span>
            )}
          </div>

          <div className="flex flex-wrap gap-2">
            {output?.exchange && (
              <Badge variant="outline" className="font-mono text-xs">
                交换机: {output.exchange}
              </Badge>
            )}
          </div>
        </div>
      </CollapsibleSection>

      {/* 发布状态 */}
      <CollapsibleSection
        title="发布状态"
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已发布" failText="发布失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? '消息已成功发布'
                : output?.error ?? '消息发布失败'}
            </span>
          </div>

          {isSuccess && output?.confirm_status && (
            <KvRow label="确认状态" value={output.confirm_status} />
          )}
          {isSuccess && output?.message_id && (
            <KvRow label="消息 ID" value={output.message_id} />
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
  const output = report.output_data as AmqpConsumeOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const messages = output?.messages ?? [];
  const messageCount = output?.message_count ?? messages.length;

  return (
    <div className="space-y-3">
      {/* 消费状态 */}
      <CollapsibleSection
        title="消费状态"
        icon={<Inbox className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="消费完成" failText="消费失败" />
            <AmqpBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `收到 ${messageCount} 条消息`
                : output?.error ?? '消费失败'}
            </span>
          </div>

          {output?.duration_ms !== undefined && (
            <KvRow label="持续时间" value={`${output.duration_ms}ms`} />
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 消息列表 */}
      {messages.length > 0 && (
        <CollapsibleSection
          title="消息列表"
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
  const output = report.output_data as AmqpGetOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const message = output?.message;

  return (
    <div className="space-y-3">
      {/* 获取状态 */}
      <CollapsibleSection
        title="获取状态"
        icon={<Inbox className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已获取" failText="获取失败" />
            <AmqpBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? message
                  ? '成功获取消息'
                  : '队列为空，无消息'
                : output?.error ?? '获取消息失败'}
            </span>
          </div>

          {output?.message_count !== undefined && (
            <KvRow label="队列消息数" value={output.message_count} />
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 消息详情 */}
      {message && (
        <CollapsibleSection
          title="消息详情"
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
  const output = report.output_data as AmqpAckOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title="确认状态"
        icon={<Check className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已确认" failText="确认失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? output?.delivery_tag !== undefined
                  ? `消息 (delivery_tag: ${output.delivery_tag}) 已确认`
                  : '消息已确认'
                : output?.error ?? '消息确认失败'}
            </span>
          </div>

          {output?.delivery_tag !== undefined && (
            <KvRow label="Delivery Tag" value={output.delivery_tag} />
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
  const output = report.output_data as AmqpManagementOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const overview = output?.overview;
  const hasResponse = output?.response !== undefined && output?.response !== null;

  return (
    <div className="space-y-3">
      {/* 管理状态 */}
      <CollapsibleSection
        title="管理操作"
        icon={<Settings className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="操作成功" failText="操作失败" />
            <AmqpBadge />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? '管理 API 请求成功'
                : output?.error ?? '管理 API 请求失败'}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 概览统计 */}
      {isSuccess && overview && (
        <CollapsibleSection
          title="概览统计"
          icon={<Globe className="w-4 h-4" />}
          defaultOpen={true}
        >
          <div className="flex flex-wrap gap-2">
            {overview.connection_count !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                连接数: {overview.connection_count}
              </Badge>
            )}
            {overview.channel_count !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                通道数: {overview.channel_count}
              </Badge>
            )}
            {overview.queue_count !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                队列数: {overview.queue_count}
              </Badge>
            )}
            {overview.message_stats?.publish !== undefined && (
              <Badge variant="secondary" className="font-mono text-xs">
                发布速率: {overview.message_stats.publish}
              </Badge>
            )}
            {overview.message_stats?.deliver !== undefined && (
              <Badge variant="secondary" className="font-mono text-xs">
                投递速率: {overview.message_stats.deliver}
              </Badge>
            )}
          </div>
        </CollapsibleSection>
      )}

      {/* 完整响应 */}
      {hasResponse && (
        <CollapsibleSection
          title="API 响应"
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
  const output = report.output_data as Amqp10ConnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title="连接状态"
        icon={<Cable className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已连接" failText="连接失败" />
            <Badge
              variant="outline"
              className="font-mono text-xs text-orange-600 dark:text-orange-400 bg-orange-50 dark:bg-orange-950/50 border-0"
            >
              AMQP 1.0
            </Badge>
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? 'AMQP 1.0 连接建立成功'
                : output?.error ?? '无法建立 AMQP 1.0 连接'}
            </span>
          </div>

          {output?.connection_id && (
            <KvRow label="连接 ID" value={output.connection_id} />
          )}
          {output?.container_id && (
            <KvRow label="容器 ID" value={output.container_id} />
          )}
          {output?.session_info && (
            <KvRow label="会话信息" value={output.session_info} />
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
  const output = report.output_data as Amqp10SendOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title="发送状态"
        icon={<Send className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已发送" failText="发送失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? '消息已成功发送'
                : output?.error ?? '消息发送失败'}
            </span>
          </div>

          {isSuccess && output?.delivery_state && (
            <KvRow label="投递状态" value={output.delivery_state} />
          )}
          {isSuccess && output?.message_id && (
            <KvRow label="消息 ID" value={output.message_id} />
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
  const output = report.output_data as Amqp10ReceiveOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const messages = output?.messages ?? [];
  const messageCount = output?.message_count ?? messages.length;

  return (
    <div className="space-y-3">
      {/* 接收状态 */}
      <CollapsibleSection
        title="接收状态"
        icon={<Inbox className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="接收完成" failText="接收失败" />
            <Badge
              variant="outline"
              className="font-mono text-xs text-orange-600 dark:text-orange-400 bg-orange-50 dark:bg-orange-950/50 border-0"
            >
              AMQP 1.0
            </Badge>
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `收到 ${messageCount} 条消息`
                : output?.error ?? '接收失败'}
            </span>
          </div>

          {output?.duration_ms !== undefined && (
            <KvRow label="持续时间" value={`${output.duration_ms}ms`} />
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 消息列表 */}
      {messages.length > 0 && (
        <CollapsibleSection
          title="消息列表"
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
  const output = report.output_data as Amqp10AcceptOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title="接受状态"
        icon={<Check className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已接受" failText="接受失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? '消息已成功接受'
                : output?.error ?? '消息接受失败'}
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
  const output = report.output_data as Amqp10RejectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
      <CollapsibleSection
        title="拒绝状态"
        icon={<Ban className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已拒绝" failText="拒绝失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? '消息已成功拒绝'
                : output?.error ?? '消息拒绝失败'}
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
  const output = report.output_data as Amqp10DisconnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-3">
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
                ? 'AMQP 1.0 连接已成功断开'
                : output?.error ?? '断开连接失败'}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
