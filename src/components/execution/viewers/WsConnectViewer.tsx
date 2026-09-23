/**
 * WsConnectViewer - WebSocket Connect 节点执行详情查看器
 * 结构化展示连接配置、连接状态、首条消息和耗时分解
 */
import { useState, memo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Wifi,
  Clock,
  MessageSquare,
  Globe,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { getLL } from '@/i18n/i18n-lazy';
import { RawJsonViewer } from './RawJsonViewer';
import { VariableChangesCard } from './VariableChangesCard';

interface WsConnectViewerProps {
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
      <div className="flex flex-wrap items-center justify-between w-full bg-muted/30 hover:bg-muted/50 transition-colors gap-2 pr-3">
        <button
          type="button"
          onClick={() => setIsOpen(!isOpen)}
          className="flex-1 min-w-0 flex items-center gap-2 p-3 text-left"
          aria-expanded={isOpen}
        >
          <div className="text-muted-foreground shrink-0">
            {isOpen ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
          </div>
          <div className="text-muted-foreground shrink-0">{icon}</div>
          <span className="font-medium text-foreground text-sm truncate min-w-0">{title}</span>
        </button>
        {extra && <div className="shrink-0 pl-3 sm:pl-0 py-1">{extra}</div>}
      </div>
      {isOpen && <div className="p-3 border-t border-border bg-background">{children}</div>}
    </div>
  );
}

/**
 * WS Connect 输出数据类型
 */
interface WsConnectOutput {
  success?: boolean;
  connected?: boolean;
  url?: string;
  connection_id?: string;
  message?: {
    data?: unknown;
    type?: string;
  } | null;
  timing?: {
    connect_ms?: number | null;
    collect_response_ms?: number | null;
    total_ms?: number | null;
  };
}

/**
 * WS Connect 配置数据类型
 */
interface WsConnectConfig {
  url?: string;
  collect_mode?: string;
  timeout_ms?: number;
}

export const WsConnectViewer = memo(function WsConnectViewer({ report }: WsConnectViewerProps) {
  const ll = getLL();
  const output = report.output_data as WsConnectOutput | undefined;
  const resolvedConfig = report.resolved_config as WsConnectConfig | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  // 配置数据
  const configUrl = resolvedConfig?.url || output?.url || '';
  const collectMode = resolvedConfig?.collect_mode || 'none';
  const timeoutMs = resolvedConfig?.timeout_ms;

  // 输出数据
  const connectionId = output?.connection_id;
  const message = output?.message;
  const connectDuration = output?.timing?.connect_ms;
  const collectDuration = output?.timing?.collect_response_ms;
  const totalDuration = output?.timing?.total_ms ?? ((connectDuration || 0) + (collectDuration || 0));

  return (
    <div className="space-y-3">
      {/* 连接配置 */}
      <CollapsibleSection
        title={ll.execution.viewers.connection_config()}
        icon={<Wifi className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* URL */}
          <div className="flex items-center gap-2">
             <Badge variant="outline" className="font-mono text-xs text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-950/50 border-0 shrink-0">
              WS
            </Badge>
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {configUrl || <span className="text-muted-foreground italic">{ll.execution.viewers.config_url()}</span>}
            </code>
          </div>

          {/* 采集模式 */}
          <div className="flex items-center gap-2 text-sm">
            <span className="text-muted-foreground">{ll.execution.viewers.collect_config()}:</span>
            <Badge variant="outline" className="font-mono text-xs">
              {(() => {
                switch (collectMode) {
                  case 'first_match': return ll.execution.viewers.ws_collect_mode_first_match();
                  case 'condition_match': return ll.execution.viewers.ws_collect_mode_condition_match();
                  case 'duration_collect': return ll.execution.viewers.ws_collect_mode_duration_collect();
                  default: return ll.execution.viewers.ws_collect_mode_first_match();
                }
              })()}
            </Badge>
          </div>

          {/* 超时设置 */}
          {timeoutMs !== undefined && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{ll.execution.viewers.timeout_label()}:</span>
              <Badge variant="secondary" className="font-mono text-xs">
                {timeoutMs}ms
              </Badge>
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
          {/* 状态 */}
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.execution.viewers.connected()} failText={ll.execution.viewers.disconnected()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess ? ll.execution.viewers.ws_connect_success_desc() : ll.execution.viewers.ws_connect_fail_desc()}
            </span>
          </div>

          {/* 连接 ID */}
          {connectionId && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{ll.execution.viewers.connection_id_label()}:</span>
              <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                {connectionId}
              </code>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 首条消息 */}
      <CollapsibleSection
        title={ll.execution.viewers.ws_first_message()}
        icon={<MessageSquare className="w-4 h-4" />}
        defaultOpen={false}
        extra={message?.type && (
          <Badge variant="outline" className="font-mono text-xs">
            {message.type}
          </Badge>
        )}
      >
        {message ? (
          <div className="space-y-2">
            {/* 消息类型 */}
            {message.type && (
              <div className="flex items-center gap-2 text-sm">
                <span className="text-muted-foreground">{ll.execution.viewers.type_label_single()}:</span>
                <Badge variant="outline" className="font-mono text-xs">
                  {message.type}
                </Badge>
              </div>
            )}
            {/* 消息数据 */}
            {message.data !== undefined && (
              <div>
                <div className="text-xs font-medium text-muted-foreground mb-1.5">{ll.execution.viewers.data_label()}</div>
                <div className="bg-muted rounded p-2">
                  {typeof message.data === 'string' ? (
                    <pre className="text-xs text-foreground whitespace-pre-wrap break-all">
                      {message.data}
                    </pre>
                  ) : (
                    <RawJsonViewer data={message.data} />
                  )}
                </div>
              </div>
            )}
          </div>
        ) : (
          <p className="text-sm text-muted-foreground italic">{ll.execution.viewers.no_first_message()}</p>
        )}
      </CollapsibleSection>

      {/* 耗时分解 */}
      {(connectDuration !== undefined || collectDuration !== undefined) && (
        <CollapsibleSection
          title={ll.execution.viewers.timing_breakdown_short()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration > 0 && (
            <span className="text-xs font-mono text-violet-600 font-medium">
              {ll.execution.viewers.total_label()} {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            {connectDuration !== undefined && (
              <div className="flex justify-between items-center p-2 bg-muted rounded">
                <span className="text-muted-foreground">{ll.execution.viewers.ws_timing_connect()}</span>
                <span className="font-mono text-foreground">{connectDuration}ms</span>
              </div>
            )}
            {collectDuration !== undefined && (
              <div className="flex justify-between items-center p-2 bg-muted rounded">
                <span className="text-muted-foreground">{ll.execution.viewers.ws_timing_collect()}</span>
                <span className="font-mono text-foreground">{collectDuration}ms</span>
              </div>
            )}
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
});
