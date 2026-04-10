/**
 * WsSendCollectViewer - WebSocket Send+Collect 节点执行详情查看器
 * 结构化展示发送配置、发送内容、收集消息列表、匹配消息、连接信息和耗时分解
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
  List,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { RawJsonViewer } from './RawJsonViewer';
import { VariableChangesCard } from './VariableChangesCard';

interface WsSendCollectViewerProps {
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
 * WS Send+Collect 输出数据结构
 */
interface WsSendCollectOutput {
  connection_id: string;
  url?: string;
  collect_timed_out: boolean;
  sent?: {
    data: string;
    type: 'text' | 'binary';
  } | null;
  message?: {
    data: string;
    type: 'text' | 'binary';
    parsed?: unknown;
  } | null;
  collected_messages?: Array<{
    data: string;
    type: 'text' | 'binary';
    parsed?: unknown;
  }>;
  matched_message?: {
    data: string;
    type: 'text' | 'binary';
    parsed?: unknown;
  } | null;
  message_count?: number;
  timing?: {
    send_ms?: number | null;
    collect_response_ms?: number | null;
    total_ms?: number | null;
  };
  error?: string;
}

/**
 * WS Send+Collect 配置数据结构
 */
interface WsSendCollectConfig {
  connection_id?: string;
  message?: string;
  message_type?: 'text' | 'binary';
  send_timeout_ms?: number;
  collect_response?: boolean;
  collect_mode?: 'first_match' | 'condition_match' | 'duration_collect';
  collect_condition?: {
    match_type?: 'json_path' | 'regex' | 'expression';
    expression?: string;
  };
  collect_timeout_ms?: number;
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

/**
 * 获取收集模式的显示文本
 */
function getCollectModeLabel(collectMode: string | undefined): string {
  switch (collectMode) {
    case 'first_match': return '采集首条即停';
    case 'condition_match': return '匹配条件即停';
    case 'duration_collect': return '持续采集';
    default: return '采集首条即停';
  }
}

/**
 * 获取匹配类型的显示文本
 */
function getMatchTypeLabel(matchType: string | undefined): string {
  switch (matchType) {
    case 'json_path': return 'JSONPath';
    case 'regex': return '正则表达式';
    case 'expression': return '表达式';
    default: return 'JSONPath';
  }
}

export function WsSendCollectViewer({ report }: WsSendCollectViewerProps) {
  const outputData = report.output_data as WsSendCollectOutput | undefined;
  const resolvedConfig = report.resolved_config as WsSendCollectConfig | undefined;
  
  const hasError = outputData?.error || report.status === 'failed';
  const isTimedOut = outputData?.collect_timed_out === true;

  // 配置数据
  const configConnectionId = resolvedConfig?.connection_id;
  const configMessage = resolvedConfig?.message;
  const configMessageType = resolvedConfig?.message_type;
  const sendTimeoutMs = resolvedConfig?.send_timeout_ms;
  const collectResponse = resolvedConfig?.collect_response ?? true;
  const collectMode = resolvedConfig?.collect_mode;
  const collectCondition = resolvedConfig?.collect_condition;
  const collectTimeoutMs = resolvedConfig?.collect_timeout_ms;

  // 输出数据
  const connectionUrl = outputData?.url;
  const connectionId = outputData?.connection_id || configConnectionId;
  const sentData = outputData?.sent;
  const messageData = outputData?.message || outputData?.matched_message;
  const collectedMessages = outputData?.collected_messages || [];
  const messageCount = outputData?.message_count;
  const matchedMessage = outputData?.matched_message;
  const sendDuration = outputData?.timing?.send_ms;
  const collectDuration = outputData?.timing?.collect_response_ms;

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
            <span className="font-medium text-sm">采集响应超时</span>
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
            <div className="flex items-center gap-2">
              <Badge variant="outline" className="font-mono text-xs text-cyan-600 bg-cyan-50 border-0 shrink-0">
                URL
              </Badge>
              <code className="flex-1 text-sm font-mono text-slate-700 break-all bg-slate-50 px-2 py-1 rounded">
                {connectionUrl}
              </code>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 采集配置（独立面板，更醒目） */}
      <CollapsibleSection
        title="采集配置"
        icon={<Clock className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* 收集响应 */}
          <div className="flex items-center gap-2 text-sm">
            <span className="text-muted-foreground">采集响应:</span>
            <Badge variant="outline" className="font-mono text-xs">
              {collectResponse ? '是' : '否'}
            </Badge>
          </div>

          {/* 收集模式（仅当收集响应时显示） */}
          {collectResponse && (
            <>
              <div className="flex items-center gap-2 text-sm">
                 <span className="text-muted-foreground">采集模式:</span>
                <Badge variant="outline" className="font-mono text-xs">
                  {getCollectModeLabel(collectMode)}
                </Badge>
              </div>

               {/* 采集条件（仅当采集模式为 condition_match 时显示） */}
               {collectMode === 'condition_match' && collectCondition && (
                 <div className="space-y-1">
                   <div className="text-xs font-medium text-muted-foreground">采集条件</div>
                  <div className="bg-slate-50 rounded p-2 space-y-1">
                    <div className="flex items-center gap-2 text-xs">
                      <span className="text-muted-foreground">匹配类型:</span>
                      <Badge variant="secondary" className="font-mono text-xs">
                        {getMatchTypeLabel(collectCondition.match_type)}
                      </Badge>
                    </div>
                    {collectCondition.expression && (
                      <div>
                        <span className="text-xs text-muted-foreground">表达式: </span>
                        <code className="text-xs bg-slate-100 px-1.5 py-0.5 rounded font-mono">
                          {collectCondition.expression}
                        </code>
                      </div>
                    )}
                  </div>
                </div>
              )}

               {/* 采集超时时间 */}
               {collectTimeoutMs !== undefined && (
                 <div className="flex items-center gap-2 text-sm">
                   <span className="text-muted-foreground">采集超时:</span>
                  <Badge variant="secondary" className="font-mono text-xs">
                    {formatDuration(collectTimeoutMs)}
                  </Badge>
                </div>
              )}
            </>
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
          {/* 消息类型 */}
          {configMessageType && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">消息类型:</span>
              <Badge variant="outline" className="font-mono text-xs">
                {configMessageType === 'binary' ? '二进制' : '文本'}
              </Badge>
            </div>
          )}

          {/* 发送超时时间 */}
          {sendTimeoutMs !== undefined && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">发送超时:</span>
              <Badge variant="secondary" className="font-mono text-xs">
                {formatDuration(sendTimeoutMs)}
              </Badge>
            </div>
          )}

          {/* 实际发送的内容 */}
          {sentData && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">发送内容</div>
              <div className="bg-slate-50 rounded p-2">
                <pre className="text-xs text-slate-700 whitespace-pre-wrap break-all max-h-40 overflow-y-auto">
                  {truncateText(sentData.data, 500)}
                </pre>
              </div>
            </div>
          )}

          {/* 配置的消息（当没有实际发送数据时显示） */}
          {!sentData && configMessage && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">配置消息</div>
              <pre className="text-xs bg-slate-50 border border-slate-200 rounded p-2 overflow-x-auto text-slate-700 max-h-32 overflow-y-auto whitespace-pre-wrap break-all">
                {truncateText(configMessage, 500)}
              </pre>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 收集消息列表 */}
      {collectedMessages.length > 0 && (
        <CollapsibleSection
          title="收集消息"
          icon={<List className="w-4 h-4" />}
          defaultOpen={true}
          extra={(
            <Badge variant="secondary" className="font-mono text-xs">
              {messageCount ?? collectedMessages.length} 条
            </Badge>
          )}
        >
          <div className="space-y-2 max-h-60 overflow-y-auto">
            {collectedMessages.map((msg, idx) => {
              const isMatched = matchedMessage && matchedMessage.data === msg.data && matchedMessage.type === msg.type;
              return (
                <div
                  key={idx}
                  className={cn(
                    'rounded-lg border p-2',
                    isMatched
                      ? 'bg-emerald-50 border-emerald-300'
                      : 'bg-slate-50 border-slate-200'
                  )}
                >
                  <div className="flex items-center gap-2 mb-1">
                    <Badge variant="outline" className={cn(
                      'font-mono text-xs',
                      msg.type === 'binary'
                        ? 'text-orange-600 bg-orange-50'
                        : 'text-slate-600 bg-slate-50'
                    )}>
                      {msg.type === 'binary' ? '二进制' : '文本'}
                    </Badge>
                    <span className="text-xs text-muted-foreground">#{idx + 1}</span>
                    {isMatched && (
                      <Badge className="font-mono text-xs bg-emerald-100 text-emerald-700 border-0">
                        匹配
                      </Badge>
                    )}
                  </div>
                  <pre className="text-xs text-slate-700 whitespace-pre-wrap break-all">
                    {truncateText(msg.data, 300)}
                  </pre>
                  {msg.parsed !== null && msg.parsed !== undefined && (
                    <details className="mt-1">
                      <summary className="text-xs text-muted-foreground cursor-pointer">解析 JSON</summary>
                      <div className="mt-1">
                        <RawJsonViewer data={msg.parsed} />
                      </div>
                    </details>
                  )}
                </div>
              );
            })}
          </div>
        </CollapsibleSection>
      )}

      {/* 匹配消息 */}
      {messageData ? (
        <CollapsibleSection
          title="匹配消息"
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
          title="匹配消息"
          icon={<MessageSquare className="w-4 h-4" />}
          defaultOpen={false}
        >
          <p className="text-sm text-muted-foreground italic">未采集响应或无匹配消息</p>
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
            <span className="text-muted-foreground">收集耗时</span>
            <span className="font-mono text-slate-700">{formatDuration(collectDuration)}</span>
          </div>
        </div>
      </CollapsibleSection>

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
