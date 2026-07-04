/**
 * RedisViewer - Redis 节点执行详情查看器
 * 结构化展示 Redis Connect/Command/Close 三种节点的执行结果
 * Connect: 连接状态、服务器信息、耗时
 * Command: 执行命令、返回结果、耗时
 * Close: 关闭连接状态、耗时
 */
import { useState, memo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Radio,
  Terminal,
  X,
  Clock,
  Globe,
  Database,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { VariableChangesCard } from './VariableChangesCard';
import { getLL } from '@/i18n/i18n-lazy';


interface RedisViewerProps {
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

/** Redis 协议标签 */
function RedisBadge() {
  return (
    <Badge
      variant="outline"
      className="font-mono text-xs text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/50 border-0 shrink-0"
    >
      REDIS
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
// Redis 输出数据类型
// ============================================================================

/** Redis Connect 输出数据 */
interface RedisConnectOutput {
  success?: boolean;
  connection_id?: string | null;
  server_info?: {
    host?: string;
    port?: number;
    version?: string;
    db_index?: number;
  };
  timing?: {
    connect_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/** Redis Command 输出数据 */
interface RedisCommandOutput {
  success?: boolean;
  command?: string;
  result?: unknown;
  result_type?: string;
  timing?: {
    command_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/** Redis Close 输出数据 */
interface RedisCloseOutput {
  success?: boolean;
  connection_id?: string | null;
  timing?: {
    close_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

// ============================================================================
// 主组件
// ============================================================================

export const RedisViewer = memo(function RedisViewer({ report }: RedisViewerProps) {
  const nodeType = report.node_type;

  switch (nodeType) {
    case 'redis_connect':
      return <RedisConnectView report={report} />;
    case 'redis_command':
      return <RedisCommandView report={report} />;
    case 'redis_close':
      return <RedisCloseView report={report} />;
    default:
      return (
        <div className="p-4 text-sm text-muted-foreground italic">
          {`Unknown Redis node type: ${nodeType}`}
        </div>
      );
  }
});

// ============================================================================
// Redis Connect 查看器
// ============================================================================

function RedisConnectView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as RedisConnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const connectDuration = output?.timing?.connect_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 连接状态 */}
      <CollapsibleSection
        title={ll.viewer.redis.section_connection()}
        icon={<Radio className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.redis.status_connected()} failText={ll.viewer.redis.status_connection_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.redis.connect_success()
                : output?.error ?? ll.viewer.redis.connect_error()}
            </span>
          </div>

          {/* 连接 ID */}
          {output?.connection_id && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{ll.viewer.redis.label_connection_id()}:</span>
              <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                {output.connection_id}
              </code>
            </div>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 服务器信息 */}
      {isSuccess && output?.server_info && (
        <CollapsibleSection
          title={ll.viewer.redis.section_server_info()}
          icon={<Globe className="w-4 h-4" />}
          defaultOpen={true}
        >
          <div className="flex flex-wrap gap-2">
            {output.server_info.host && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.redis.label_host()}: {output.server_info.host}
              </Badge>
            )}
            {output.server_info.port !== undefined && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.redis.label_port()}: {output.server_info.port}
              </Badge>
            )}
            {output.server_info.version && (
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.redis.label_version()}: {output.server_info.version}
              </Badge>
            )}
            {output.server_info.db_index !== undefined && (
              <Badge variant="secondary" className="font-mono text-xs">
                DB: {output.server_info.db_index}
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
            <span className="text-xs font-mono text-red-600 font-medium">
              {ll.viewer.common.total_duration_ms({ms: totalDuration})}
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
// Redis Command 查看器
// ============================================================================

function RedisCommandView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as RedisCommandOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const commandDuration = output?.timing?.command_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 命令信息 */}
      <CollapsibleSection
        title={ll.viewer.redis.section_command_info()}
        icon={<Terminal className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* 命令 */}
          <div className="flex items-center gap-2">
            <RedisBadge />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {output?.command || <span className="text-muted-foreground italic">{ll.viewer.redis.no_command_executed()}</span>}
            </code>
          </div>

          {/* 结果类型 */}
          {output?.result_type && (
            <div className="flex items-center gap-2">
              <Badge variant="outline" className="font-mono text-xs">
                {ll.viewer.redis.label_type()}: {output.result_type}
              </Badge>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 执行状态 */}
      <CollapsibleSection
        title={ll.viewer.common.section_execution_status()}
        icon={<Database className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.redis.status_success()} failText={ll.viewer.redis.status_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.redis.command_success()
                : output?.error ?? ll.viewer.redis.command_error()}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 返回结果 */}
      {isSuccess && output?.result !== undefined && (
        <CollapsibleSection
          title={ll.viewer.redis.section_result_data()}
          icon={<Database className="w-4 h-4" />}
          defaultOpen={true}
        >
          <pre className="text-xs font-mono text-foreground whitespace-pre-wrap break-all max-h-60 overflow-auto bg-muted rounded p-2">
            {typeof output.result === 'string'
              ? output.result
              : JSON.stringify(output.result, null, 2)}
          </pre>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(commandDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-red-600 font-medium">
              {ll.viewer.common.total_duration_ms({ms: totalDuration})}
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_command()} value={commandDuration} />
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
// Redis Close 查看器
// ============================================================================

function RedisCloseView({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as RedisCloseOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const closeDuration = output?.timing?.close_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 关闭状态 */}
      <CollapsibleSection
        title={ll.viewer.redis.section_close()}
        icon={<X className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.redis.status_closed()} failText={ll.viewer.redis.status_close_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.redis.close_success()
                : output?.error ?? ll.viewer.redis.close_error()}
            </span>
          </div>

          {/* 连接 ID */}
          {output?.connection_id && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{ll.viewer.redis.label_connection_id()}:</span>
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
          title={ll.viewer.common.duration_decomposition()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-red-600 font-medium">
              {ll.viewer.common.total_duration_ms({ms: totalDuration})}
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label={ll.viewer.common.timing_close()} value={closeDuration} />
            <TimingMetric label={ll.viewer.common.timing_total()} value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
