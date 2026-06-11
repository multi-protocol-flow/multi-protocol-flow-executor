/**
 * KafkaViewer - Kafka 节点执行详情查看器
 * 结构化展示 Kafka Connect/Produce/Consume/ConsumeBatch/Admin/SchemaRegistry/Close 七种节点的执行结果
 * Connect: 连接状态、Broker 信息、耗时
 * Produce: 发送配置、发送状态（topic/partition/offset）、耗时
 * Consume: 消费配置、消息列表、耗时
 * ConsumeBatch: 批量消费配置、消息列表、耗时
 * Admin: 管理操作结果、耗时
 * SchemaRegistry: Schema 操作结果、耗时
 * Close: 关闭连接状态
 */
import { useState, memo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Cable,
  Send,
  Inbox,
  X,
  Clock,
  Globe,
  MessageSquare,
  Settings,
  FileJson,
  Database,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { VariableChangesCard } from './VariableChangesCard';
import { getLL } from '@/i18n/i18n-lazy';

interface KafkaViewerProps {
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

/** Kafka 协议标签 */
function KafkaBadge() {
  return (
    <Badge
      variant="outline"
      className="font-mono text-xs text-orange-600 dark:text-orange-400 bg-orange-50 dark:bg-orange-950/50 border-0 shrink-0"
    >
      KAFKA
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
// Kafka 输出数据类型
// ============================================================================

/** Kafka Connect 输出数据 */
interface KafkaConnectOutput {
  success?: boolean;
  connection_id?: string | null;
  broker_info?: {
    bootstrap_servers?: string;
    broker_id?: number;
    rack?: string;
    cluster_id?: string;
  };
  timing?: {
    connect_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/** Kafka Produce 输出数据 */
interface KafkaProduceOutput {
  success?: boolean;
  topic?: string;
  partition?: number;
  offset?: number;
  timestamp?: number;
  key_size?: number;
  value_size?: number;
  timing?: {
    produce_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/** Kafka 单条消费消息 */
interface KafkaMessage {
  topic?: string;
  partition?: number;
  offset?: number;
  key?: string;
  value?: string;
  headers?: Record<string, string>;
  timestamp?: number;
}

/** Kafka Consume 输出数据 */
interface KafkaConsumeOutput {
  success?: boolean;
  topic?: string;
  partition?: number;
  messages?: KafkaMessage[];
  message_count?: number;
  timing?: {
    consume_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/** Kafka ConsumeBatch 输出数据 */
interface KafkaConsumeBatchOutput {
  success?: boolean;
  topic?: string;
  messages?: KafkaMessage[];
  message_count?: number;
  batch_strategy?: string;
  timing?: {
    consume_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/** Kafka Admin 输出数据 */
interface KafkaAdminOutput {
  success?: boolean;
  operation?: string;
  result?: Record<string, unknown>;
  timing?: {
    admin_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/** Kafka SchemaRegistry 输出数据 */
interface KafkaSchemaRegistryOutput {
  success?: boolean;
  operation?: string;
  result?: Record<string, unknown>;
  timing?: {
    registry_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/** Kafka Close 输出数据 */
interface KafkaCloseOutput {
  success?: boolean;
  connection_id?: string | null;
  timing?: {
    close_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

// ============================================================================
// 消息卡片组件
// ============================================================================

function KafkaMessageCard({ message, index }: { message: KafkaMessage; index: number }) {
  const value = message.value ?? '';
  const truncatedValue = value.length > 200 ? value.slice(0, 200) + '...' : value;

  return (
    <div className="border border-border rounded-lg p-2.5">
      {/* 消息头部 */}
      <div className="flex items-center gap-2 mb-1.5 flex-wrap">
        <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono">
          #{index + 1}
        </Badge>
        {message.topic && (
          <code className="text-xs font-mono text-foreground break-all">
            {message.topic}
          </code>
        )}
        {message.partition !== undefined && (
          <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono">
            P{message.partition}
          </Badge>
        )}
        {message.offset !== undefined && (
          <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono text-blue-600 dark:text-blue-400 border-blue-300 dark:border-blue-700">
            @{message.offset}
          </Badge>
        )}
        {message.timestamp !== undefined && (
          <span className="text-[10px] text-muted-foreground font-mono ml-auto">
            {message.timestamp}ms
          </span>
        )}
      </div>

      {/* Key */}
      {message.key && (
        <div className="flex items-center gap-2 mb-1.5 text-xs">
          <span className="text-muted-foreground shrink-0">Key:</span>
          <code className="font-mono text-foreground bg-muted px-1.5 py-0.5 rounded break-all">
            {message.key.length > 100 ? message.key.slice(0, 100) + '...' : message.key}
          </code>
        </div>
      )}

      {/* Value 预览 */}
      {value && (
        <pre className="text-xs font-mono text-foreground whitespace-pre-wrap break-all max-h-40 overflow-auto bg-muted rounded p-2">
          {truncatedValue}
        </pre>
      )}

      {/* Headers */}
      {message.headers && Object.keys(message.headers).length > 0 && (
        <div className="mt-1.5 text-xs text-muted-foreground">
          Headers: {Object.entries(message.headers).map(([k, v]) => `${k}=${v}`).join(', ')}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// 主组件
// ============================================================================

export const KafkaViewer = memo(function KafkaViewer({ report }: KafkaViewerProps) {
  const nodeType = report.node_type;
  const ll = getLL();

  switch (nodeType) {
    case 'kafka_connect':
      return <KafkaConnectView report={report} />;
    case 'kafka_produce':
      return <KafkaProduceView report={report} />;
    case 'kafka_consume':
      return <KafkaConsumeView report={report} />;
    case 'kafka_consume_batch':
      return <KafkaConsumeBatchView report={report} />;
    case 'kafka_admin':
      return <KafkaAdminView report={report} />;
    case 'kafka_schema_registry':
      return <KafkaSchemaRegistryView report={report} />;
    case 'kafka_close':
      return <KafkaCloseView report={report} />;
    default:
      return (
        <div className="p-4 text-sm text-muted-foreground italic">
          {ll.viewer.kafka.unknown_node_type({ type: nodeType })}
        </div>
      );
  }
});

// ============================================================================
// Kafka Connect 查看器
// ============================================================================

function KafkaConnectView({ report }: { report: NodeExecutionReport }) {
  const output = report.output_data as KafkaConnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const brokerInfo = output?.broker_info;
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
                ? 'Kafka 连接建立成功'
                : output?.error ?? '无法建立 Kafka 连接'}
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
          title="Broker 信息"
          icon={<Globe className="w-4 h-4" />}
          defaultOpen={true}
        >
          <div className="flex flex-wrap gap-2">
            {brokerInfo.bootstrap_servers && (
              <Badge variant="outline" className="font-mono text-xs">
                Servers: {brokerInfo.bootstrap_servers}
              </Badge>
            )}
            {brokerInfo.broker_id !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                Broker ID: {brokerInfo.broker_id}
              </Badge>
            )}
            {brokerInfo.cluster_id && (
              <Badge variant="outline" className="font-mono text-xs">
                Cluster: {brokerInfo.cluster_id}
              </Badge>
            )}
            {brokerInfo.rack && (
              <Badge variant="secondary" className="font-mono text-xs">
                Rack: {brokerInfo.rack}
              </Badge>
            )}
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(connectDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-orange-600 font-medium">
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
// Kafka Produce 查看器
// ============================================================================

function KafkaProduceView({ report }: { report: NodeExecutionReport }) {
  const output = report.output_data as KafkaProduceOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const produceDuration = output?.timing?.produce_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 发送配置 */}
      <CollapsibleSection
        title="发送配置"
        icon={<Send className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* Topic */}
          <div className="flex items-center gap-2">
            <KafkaBadge />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {output?.topic || <span className="text-muted-foreground italic">未配置 Topic</span>}
            </code>
          </div>

          {/* Partition / Offset / Size */}
          <div className="flex flex-wrap gap-2">
            {output?.partition !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                Partition: {output.partition}
              </Badge>
            )}
            {output?.offset !== undefined && (
              <Badge variant="outline" className="font-mono text-xs text-blue-600 dark:text-blue-400 border-blue-300 dark:border-blue-700">
                Offset: {output.offset}
              </Badge>
            )}
            {output?.key_size !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                Key 大小: {output.key_size} 字节
              </Badge>
            )}
            {output?.value_size !== undefined && (
              <Badge variant="secondary" className="font-mono text-xs">
                Value 大小: {output.value_size} 字节
              </Badge>
            )}
          </div>
        </div>
      </CollapsibleSection>

      {/* 发送状态 */}
      <CollapsibleSection
        title="发送状态"
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已发送" failText="发送失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `消息已成功发送到 ${output?.topic ?? 'topic'} [P${output?.partition ?? 0}@${output?.offset ?? 0}]`
                : output?.error ?? '消息发送失败'}
            </span>
          </div>

          {/* Timestamp */}
          {isSuccess && output?.timestamp !== undefined && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">时间戳:</span>
              <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                {output.timestamp}
              </code>
            </div>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 耗时分解 */}
      {(produceDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-orange-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label="发送耗时" value={produceDuration} />
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
// Kafka Consume 查看器
// ============================================================================

function KafkaConsumeView({ report }: { report: NodeExecutionReport }) {
  const output = report.output_data as KafkaConsumeOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const messages = output?.messages ?? [];
  const messageCount = output?.message_count ?? messages.length;
  const consumeDuration = output?.timing?.consume_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 消费配置 */}
      <CollapsibleSection
        title="消费配置"
        icon={<Inbox className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* Topic */}
          <div className="flex items-center gap-2">
            <KafkaBadge />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {output?.topic || <span className="text-muted-foreground italic">未配置 Topic</span>}
            </code>
          </div>

          {output?.partition !== undefined && (
            <div className="flex flex-wrap gap-2">
              <Badge variant="outline" className="font-mono text-xs">
                Partition: {output.partition}
              </Badge>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 消费状态 */}
      <CollapsibleSection
        title="消费状态"
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已消费" failText="消费失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `消费成功，收到 ${messageCount} 条消息`
                : output?.error ?? '消费失败'}
            </span>
          </div>

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
              <KafkaMessageCard key={idx} message={msg} index={idx} />
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(consumeDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-orange-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label="消费耗时" value={consumeDuration} />
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
// Kafka ConsumeBatch 查看器
// ============================================================================

function KafkaConsumeBatchView({ report }: { report: NodeExecutionReport }) {
  const output = report.output_data as KafkaConsumeBatchOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const messages = output?.messages ?? [];
  const messageCount = output?.message_count ?? messages.length;
  const consumeDuration = output?.timing?.consume_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 批量消费配置 */}
      <CollapsibleSection
        title="批量消费配置"
        icon={<Database className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* Topic */}
          <div className="flex items-center gap-2">
            <KafkaBadge />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {output?.topic || <span className="text-muted-foreground italic">未配置 Topic</span>}
            </code>
          </div>

          <div className="flex flex-wrap gap-2">
            {output?.batch_strategy && (
              <Badge variant="outline" className="font-mono text-xs">
                策略: {output.batch_strategy}
              </Badge>
            )}
          </div>
        </div>
      </CollapsibleSection>

      {/* 批量消费状态 */}
      <CollapsibleSection
        title="批量消费状态"
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已消费" failText="消费失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `批量消费成功，共 ${messageCount} 条消息`
                : output?.error ?? '批量消费失败'}
            </span>
          </div>

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
              <KafkaMessageCard key={idx} message={msg} index={idx} />
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(consumeDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-orange-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label="消费耗时" value={consumeDuration} />
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
// Kafka Admin 查看器
// ============================================================================

function KafkaAdminView({ report }: { report: NodeExecutionReport }) {
  const output = report.output_data as KafkaAdminOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const adminDuration = output?.timing?.admin_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 管理操作 */}
      <CollapsibleSection
        title="管理操作"
        icon={<Settings className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="成功" failText="失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `管理操作${output?.operation ? ` (${output.operation})` : ''}执行成功`
                : output?.error ?? '管理操作执行失败'}
            </span>
          </div>

          {output?.operation && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">操作:</span>
              <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                {output.operation}
              </code>
            </div>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 操作结果 */}
      {isSuccess && output?.result && Object.keys(output.result).length > 0 && (
        <CollapsibleSection
          title="操作结果"
          icon={<FileJson className="w-4 h-4" />}
          defaultOpen={false}
        >
          <pre className="text-xs font-mono text-foreground whitespace-pre-wrap break-all max-h-60 overflow-auto bg-muted rounded p-2">
            {JSON.stringify(output.result, null, 2)}
          </pre>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(adminDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-orange-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label="操作耗时" value={adminDuration} />
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
// Kafka SchemaRegistry 查看器
// ============================================================================

function KafkaSchemaRegistryView({ report }: { report: NodeExecutionReport }) {
  const output = report.output_data as KafkaSchemaRegistryOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const registryDuration = output?.timing?.registry_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* Schema 操作 */}
      <CollapsibleSection
        title="Schema 操作"
        icon={<FileJson className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="成功" failText="失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `Schema 操作${output?.operation ? ` (${output.operation})` : ''}执行成功`
                : output?.error ?? 'Schema 操作执行失败'}
            </span>
          </div>

          {output?.operation && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">操作:</span>
              <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                {output.operation}
              </code>
            </div>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* Schema 结果 */}
      {isSuccess && output?.result && Object.keys(output.result).length > 0 && (
        <CollapsibleSection
          title="Schema 详情"
          icon={<FileJson className="w-4 h-4" />}
          defaultOpen={false}
        >
          <pre className="text-xs font-mono text-foreground whitespace-pre-wrap break-all max-h-60 overflow-auto bg-muted rounded p-2">
            {JSON.stringify(output.result, null, 2)}
          </pre>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(registryDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-orange-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label="操作耗时" value={registryDuration} />
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
// Kafka Close 查看器
// ============================================================================

function KafkaCloseView({ report }: { report: NodeExecutionReport }) {
  const output = report.output_data as KafkaCloseOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const closeDuration = output?.timing?.close_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 关闭状态 */}
      <CollapsibleSection
        title="关闭状态"
        icon={<X className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="已关闭" failText="关闭失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? 'Kafka 连接已成功关闭'
                : output?.error ?? '关闭连接失败'}
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

      {/* 耗时分解 */}
      {(closeDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-orange-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label="关闭耗时" value={closeDuration} />
            <TimingMetric label="总耗时" value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
