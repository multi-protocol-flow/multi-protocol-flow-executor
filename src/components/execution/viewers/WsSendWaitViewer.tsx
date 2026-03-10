/**
 * WsSendWaitViewer - WebSocket Send+Wait 节点执行详情查看器
 * 结构化展示发送配置、发送内容、响应内容、连接信息和耗时分解
 */
import { useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Send,
  MessageSquare,
  Clock,
  AlertTriangle,
  Link,
  Timer,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { RawJsonViewer } from './RawJsonViewer';
import { VariableChangesCard } from './VariableChangesCard';

interface WsSendWaitViewerProps {
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
 * WS Send+Wait 输出数据结构
 */
interface WsSendWaitOutput {
  connection_id: string;
  url?: string;
  wait_timed_out: boolean;
  sent?: {
    data: string;
    type: 'text' | 'binary';
  } | null;
  message?: {
    data: string;
    type: 'text' | 'binary';
    parsed?: unknown;
  } | null;
  send_duration_ms?: number | null;
  wait_response_duration_ms?: number | null;
  error?: string;
}

/**
 * WS Send+Wait 配置数据结构
 */
interface WsSendWaitConfig {
  connection_id?: string;
  message?: string;
  wait_response?: boolean;
  timeout_ms?: number;
}

/**
 * 格式化毫秒时间
 */
function formatDuration(ms: number | null | undefined): string {
  if (ms === null || ms === undefined) return '-';
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

/**
 * 截断长文本
 */
function truncateText(text: string, maxLength: number = 500): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + '...';
}

export function WsSendWaitViewer({ report }: WsSendWaitViewerProps) {
  const outputData = report.output_data as WsSendWaitOutput | undefined;
  const resolvedConfig = report.resolved_config as WsSendWaitConfig | undefined;
  
  const hasError = outputData?.error || report.status === 'failed';
  const isTimedOut = outputData?.wait_timed_out === true;

  // 配置数据
  const configConnectionId = resolvedConfig?.connection_id;
  const configMessage = resolvedConfig?.message;
  const waitResponse = resolvedConfig?.wait_response ?? true;
  const timeoutMs = resolvedConfig?.timeout_ms;

  // 输出数据
  const connectionUrl = outputData?.url;
  const connectionId = outputData?.connection_id || configConnectionId;
  const sentData = outputData?.sent;
  const messageData = outputData?.message;
  const sendDuration = outputData?.send_duration_ms;
  const waitDuration = outputData?.wait_response_duration_ms;

  return (
    <div className="space-y-3">
      {/* 错误信息 */}
      {hasError && outputData?.error && (
        <div className="rounded-lg bg-red-50 border border-red-200 p-3">
          <div className="flex items-center gap-1.5 text-red-600">
            <AlertTriangle className="w-4 h-4" />
            <span className="font-medium text-sm">执行错误</span>
          </div>
          <p className="text-red-600 text-sm mt-1">{outputData.error}</p>
        </div>
      )}

      {/* 超时警告 */}
      {isTimedOut && !hasError && (
        <div className="rounded-lg bg-amber-50 border border-amber-200 p-3">
          <div className="flex items-center gap-1.5 text-amber-600">
            <Timer className="w-4 h-4" />
            <span className="font-medium text-sm">等待响应超时</span>
          </div>
          <p className="text-amber-600/80 text-xs mt-1">
            在指定时间内未收到匹配的响应消息
          </p>
        </div>
      )}

      {/* 连接信息 */}
      <CollapsibleSection
        title="连接信息"
        icon={<Link className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          {/* 连接 ID */}
          <div className="flex items-center gap-2 text-sm">
            <span className="text-muted-foreground">连接 ID:</span>
            <Badge variant="secondary" className="font-mono text-xs">
              {connectionId || '-'}
            </Badge>
          </div>

          {/* URL */}
          {connectionUrl && (
            <div className="flex items-start gap-2">
              <span className="text-muted-foreground text-sm whitespace-nowrap">URL:</span>
              <code className="flex-1 text-sm font-mono text-slate-700 break-all bg-slate-50 px-2 py-1 rounded">
                {connectionUrl}
              </code>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 发送配置 */}
      <CollapsibleSection
        title="发送配置"
        icon={<Send className="w-4 h-4" />}
        defaultOpen={false}
        extra={sentData?.type && (
          <Badge variant="outline" className={cn(
            'font-mono text-xs',
            sentData.type === 'binary'
              ? 'text-orange-600 bg-orange-50'
              : 'text-slate-600 bg-slate-50'
          )}>
            {sentData.type === 'binary' ? '二进制' : '文本'}
          </Badge>
        )}
      >
        <div className="space-y-3">
          {/* 等待响应 */}
          <div className="flex items-center gap-2 text-sm">
            <span className="text-muted-foreground">等待响应:</span>
            <Badge variant="outline" className="font-mono text-xs">
              {waitResponse ? '是' : '否'}
            </Badge>
          </div>

          {/* 超时时间 */}
          {timeoutMs !== undefined && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">超时时间:</span>
              <Badge variant="secondary" className="font-mono text-xs">
                {timeoutMs}ms
              </Badge>
            </div>
          )}

          {/* 配置的消息 */}
          {configMessage && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">配置消息</div>
              <pre className="text-xs bg-slate-50 border border-slate-200 rounded p-2 overflow-x-auto text-slate-700 max-h-32 overflow-y-auto whitespace-pre-wrap break-all">
                {truncateText(configMessage, 500)}
              </pre>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 发送内容 */}
      {sentData && (
        <CollapsibleSection
          title="发送内容"
          icon={<Send className="w-4 h-4" />}
          defaultOpen={false}
          extra={(
            <Badge variant="outline" className={cn(
              'font-mono text-xs',
              sentData.type === 'binary'
                ? 'text-orange-600 bg-orange-50'
                : 'text-slate-600 bg-slate-50'
            )}>
              {sentData.type === 'binary' ? '二进制' : '文本'}
            </Badge>
          )}
        >
          <div className="bg-slate-50 rounded p-2">
            <pre className="text-xs text-slate-700 whitespace-pre-wrap break-all max-h-40 overflow-y-auto">
              {truncateText(sentData.data, 500)}
            </pre>
          </div>
        </CollapsibleSection>
      )}

      {/* 响应内容 */}
      {messageData ? (
        <CollapsibleSection
          title="响应内容"
          icon={<MessageSquare className="w-4 h-4" />}
          defaultOpen={true}
          extra={(
            <Badge variant="outline" className={cn(
              'font-mono text-xs',
              messageData.type === 'binary'
                ? 'text-orange-600 bg-orange-50'
                : 'text-emerald-600 bg-emerald-50'
            )}>
              {messageData.type === 'binary' ? '二进制' : '文本'}
            </Badge>
          )}
        >
          <div className="space-y-2">
            {/* 原始数据 */}
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">原始数据</div>
              <div className="bg-slate-50 rounded p-2">
                <pre className="text-xs text-slate-700 whitespace-pre-wrap break-all max-h-40 overflow-y-auto">
                  {truncateText(messageData.data, 500)}
                </pre>
              </div>
            </div>

            {/* 解析后的 JSON */}
            {messageData.parsed !== null && messageData.parsed !== undefined && (
              <div>
                <div className="text-xs font-medium text-muted-foreground mb-1.5">解析后的 JSON</div>
                <div className="bg-slate-50 rounded p-2">
                  <RawJsonViewer data={messageData.parsed} />
                </div>
              </div>
            )}
          </div>
        </CollapsibleSection>
      ) : !isTimedOut && !hasError && (
        <CollapsibleSection
          title="响应内容"
          icon={<MessageSquare className="w-4 h-4" />}
          defaultOpen={false}
        >
          <p className="text-sm text-muted-foreground italic">未等待响应或无响应数据</p>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      <CollapsibleSection
        title="耗时分解"
        icon={<Clock className="w-4 h-4" />}
        defaultOpen={false}
        extra={(
          <span className="text-xs font-mono text-violet-600 font-medium">
            总计 {report.duration_ms}ms
          </span>
        )}
      >
        <div className="grid grid-cols-2 gap-2 text-sm">
          <div className="flex justify-between items-center p-2 bg-slate-50 rounded">
            <span className="text-muted-foreground">发送耗时</span>
            <span className="font-mono text-slate-700">{formatDuration(sendDuration)}</span>
          </div>
          <div className="flex justify-between items-center p-2 bg-slate-50 rounded">
            <span className="text-muted-foreground">等待耗时</span>
            <span className="font-mono text-slate-700">{formatDuration(waitDuration)}</span>
          </div>
        </div>
      </CollapsibleSection>

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
