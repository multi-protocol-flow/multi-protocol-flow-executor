/**
 * WsSendCollectViewer - WebSocket Send+Collect 节点执行详情查看器
 * 结构化展示发送配置、发送内容、收集消息列表、匹配消息、连接信息和耗时分解
 */
import { useState, memo } from 'react';
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
import { getLL } from '@/i18n/i18n-lazy';
import { RawJsonViewer } from './RawJsonViewer';
import { VariableChangesCard } from './VariableChangesCard';
import { HexViewer, dataToBytes } from '@/components/common/HexViewer';

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
 * 获取收集模式的显示文本
 */
function getCollectModeLabel(collectMode: string | undefined, ll: ReturnType<typeof getLL>): string {
  const llViewers = ll.execution.viewers;
  switch (collectMode) {
    case 'first_match': return llViewers.ws_collect_mode_first_match();
    case 'condition_match': return llViewers.ws_collect_mode_condition_match();
    case 'duration_collect': return llViewers.ws_collect_mode_duration_collect();
    default: return llViewers.ws_collect_mode_first_match();
  }
}

/**
 * 获取匹配类型的显示文本
 */
function getMatchTypeLabel(matchType: string | undefined, ll: ReturnType<typeof getLL>): string {
  const llViewers = ll.execution.viewers;
  switch (matchType) {
    case 'json_path': return 'JSONPath';
    case 'regex': return llViewers.ws_match_type_regex();
    case 'expression': return llViewers.ws_match_type_expression();
    default: return 'JSONPath';
  }
}

export const WsSendCollectViewer = memo(function WsSendCollectViewer({ report }: WsSendCollectViewerProps) {
  const ll = getLL();
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
        <div className="rounded-lg bg-red-50 dark:bg-red-950/50 border border-red-200 dark:border-red-800 p-3">
          <div className="flex items-center gap-1.5 text-red-600 dark:text-red-400">
            <AlertTriangle className="w-4 h-4" />
            <span className="font-medium text-sm">{getLL().execution.viewers.execution_failed()}</span>
          </div>
          <p className="text-red-600 dark:text-red-400 text-sm mt-1">{outputData.error}</p>
        </div>
      )}

      {/* 超时警告 */}
      {isTimedOut && !hasError && (
        <div className="rounded-lg bg-amber-50 dark:bg-amber-950/50 border border-amber-200 dark:border-amber-800 p-3">
          <div className="flex items-center gap-1.5 text-amber-600 dark:text-amber-400">
            <Timer className="w-4 h-4" />
            <span className="font-medium text-sm">{ll.execution.viewers.collection_timed_out()}</span>
          </div>
          <p className="text-amber-600/80 dark:text-amber-400/80 text-xs mt-1">
            {ll.execution.viewers.collect_timeout_desc()}
          </p>
        </div>
      )}

      {/* 连接信息 */}
      <CollapsibleSection
        title={ll.execution.viewers.connection_info()}
        icon={<Link className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          {/* 连接 ID */}
          <div className="flex items-center gap-2 text-sm">
            <span className="text-muted-foreground">{ll.execution.viewers.connection_id_label()}:</span>
            <Badge variant="secondary" className="font-mono text-xs">
              {connectionId || '-'}
            </Badge>
          </div>

          {/* URL */}
          {connectionUrl && (
            <div className="flex items-center gap-2">
               <Badge variant="outline" className="font-mono text-xs text-cyan-600 dark:text-cyan-400 bg-cyan-50 dark:bg-cyan-950/50 border-0 shrink-0">
                URL
              </Badge>
              <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
                {connectionUrl}
              </code>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 采集配置（独立面板，更醒目） */}
      <CollapsibleSection
        title={ll.execution.viewers.collect_config()}
        icon={<Clock className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* 收集响应 */}
          <div className="flex items-center gap-2 text-sm">
            <span className="text-muted-foreground">{ll.execution.viewers.ws_collect_response_label()}</span>
            <Badge variant="outline" className="font-mono text-xs">
              {collectResponse ? ll.execution.viewers.ws_yes() : ll.execution.viewers.ws_no()}
            </Badge>
          </div>

          {/* 收集模式（仅当收集响应时显示） */}
          {collectResponse && (
            <>
              <div className="flex items-center gap-2 text-sm">
                 <span className="text-muted-foreground">{ll.execution.viewers.ws_collect_mode_label()}</span>
                <Badge variant="outline" className="font-mono text-xs">
                  {getCollectModeLabel(collectMode, ll)}
                </Badge>
              </div>

               {/* 采集条件（仅当采集模式为 condition_match 时显示） */}
               {collectMode === 'condition_match' && collectCondition && (
                 <div className="space-y-1">
                    <div className="text-xs font-medium text-muted-foreground">{ll.execution.viewers.ws_collect_condition_label()}</div>
                  <div className="bg-muted rounded p-2 space-y-1">
                    <div className="flex items-center gap-2 text-xs">
                      <span className="text-muted-foreground">{ll.execution.viewers.ws_match_type_label()}</span>
                      <Badge variant="secondary" className="font-mono text-xs">
                        {getMatchTypeLabel(collectCondition.match_type, ll)}
                      </Badge>
                    </div>
                    {collectCondition.expression && (
                      <div>
                        <span className="text-xs text-muted-foreground">{ll.execution.viewers.ws_expression_label()}</span>
                        <code className="text-xs bg-muted px-1.5 py-0.5 rounded font-mono">
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
                    <span className="text-muted-foreground">{ll.execution.viewers.ws_collect_timeout_label()}</span>
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
        title={ll.execution.viewers.send_config()}
        icon={<Send className="w-4 h-4" />}
        defaultOpen={false}
        extra={sentData?.type && (
           <Badge variant="outline" className={cn(
             'font-mono text-xs',
             sentData.type === 'binary'
               ? 'text-orange-600 dark:text-orange-400 bg-orange-50 dark:bg-orange-950/50'
                : 'text-muted-foreground bg-muted'
           )}>
            {sentData.type === 'binary' ? ll.execution.viewers.binary_label() : ll.execution.viewers.text_label()}
          </Badge>
        )}
      >
        <div className="space-y-3">
          {/* 消息类型 */}
          {configMessageType && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{ll.execution.viewers.ws_message_type_label()}</span>
              <Badge variant="outline" className="font-mono text-xs">
                {configMessageType === 'binary' ? ll.execution.viewers.binary_label() : ll.execution.viewers.text_label()}
              </Badge>
            </div>
          )}

          {/* 发送超时时间 */}
          {sendTimeoutMs !== undefined && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{ll.execution.viewers.ws_send_timeout_label()}</span>
              <Badge variant="secondary" className="font-mono text-xs">
                {formatDuration(sendTimeoutMs)}
              </Badge>
            </div>
          )}

          {/* 实际发送的内容 */}
          {sentData && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">{ll.execution.viewers.send_content()}</div>
              {sentData.type === 'binary' ? (
                <div className="rounded-md border border-orange-200 dark:border-orange-800 overflow-hidden">
                  <div className="px-2 py-1 bg-orange-50 dark:bg-orange-950/50 border-b border-orange-200 dark:border-orange-800 flex items-center gap-2">
                    <span className="text-[10px] font-mono font-medium text-orange-600 dark:text-orange-400">Binary Hex View</span>
                    <span className="text-[10px] text-orange-500 dark:text-orange-500">
                      {dataToBytes(sentData.data).length} bytes
                    </span>
                  </div>
                  <HexViewer data={dataToBytes(sentData.data)} height={180} readOnly />
                </div>
              ) : (
                <div className="bg-muted rounded p-2">
                  <pre className="text-xs text-foreground whitespace-pre-wrap break-all">
                    {sentData.data}
                  </pre>
                </div>
              )}
            </div>
          )}

          {/* 配置的消息（当没有实际发送数据时显示） */}
          {!sentData && configMessage && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">{ll.execution.viewers.config_message()}</div>
              <pre className="text-xs bg-muted border border-border rounded p-2 overflow-x-auto text-foreground whitespace-pre-wrap break-all">
                {configMessage}
              </pre>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 收集消息列表 */}
      {collectedMessages.length > 0 && (
        <CollapsibleSection
          title={ll.execution.viewers.matched_message()}
          icon={<List className="w-4 h-4" />}
          defaultOpen={true}
          extra={(
            <Badge variant="secondary" className="font-mono text-xs">
              {ll.execution.viewers.ws_message_count_label({ count: messageCount ?? collectedMessages.length })}
            </Badge>
          )}
        >
          <div className="space-y-2 max-h-60 overflow-y-auto">
            {collectedMessages.map((msg, idx) => {
              const isMatched = matchedMessage && matchedMessage.data === msg.data && matchedMessage.type === msg.type;
              return (
                <div
                  key={`${idx}-${msg.type}-${msg.data.length}`}
                  className={cn(
                    'rounded-lg border p-2',
                    isMatched
                      ? 'bg-emerald-50 dark:bg-emerald-950/50 border-emerald-300 dark:border-emerald-700'
                      : 'bg-muted border-border'
                  )}
                >
                  <div className="flex items-center gap-2 mb-1">
                    <Badge variant="outline" className={cn(
                       'font-mono text-xs',
                       msg.type === 'binary'
                        ? 'text-orange-600 dark:text-orange-400 bg-orange-50 dark:bg-orange-950/50'
                         : 'text-muted-foreground bg-muted'
                     )}>
                      {msg.type === 'binary' ? ll.execution.viewers.binary_label() : ll.execution.viewers.text_label()}
                    </Badge>
                    <span className="text-xs text-muted-foreground">#{idx + 1}</span>
                    {isMatched && (
<Badge className="font-mono text-xs bg-emerald-100 dark:bg-emerald-900/50 text-emerald-700 dark:text-emerald-300 border-0">
                         {ll.execution.viewers.matched_label()}
                       </Badge>
                    )}
                  </div>
                  {msg.type === 'binary' ? (
                    <div className="rounded border border-orange-200 dark:border-orange-800 overflow-hidden">
                      <div className="px-2 py-0.5 bg-orange-50 dark:bg-orange-950/50 border-b border-orange-200 dark:border-orange-800 flex items-center gap-2">
                        <span className="text-[10px] font-mono text-orange-600 dark:text-orange-400">Binary Hex View</span>
                        <span className="text-[10px] text-orange-500">{dataToBytes(msg.data).length} bytes</span>
                      </div>
                      <HexViewer data={dataToBytes(msg.data)} height={120} readOnly />
                    </div>
                  ) : (
                    <pre className="text-xs text-foreground whitespace-pre-wrap break-all">
                      {msg.data}
                    </pre>
                  )}
                  {msg.parsed !== null && msg.parsed !== undefined && (
                    <details className="mt-1">
                      <summary className="text-xs text-muted-foreground cursor-pointer">{ll.execution.viewers.ws_parse_json()}</summary>
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
          title={ll.execution.viewers.matched_message()}
          icon={<MessageSquare className="w-4 h-4" />}
          defaultOpen={true}
          extra={(
             <Badge variant="outline" className={cn(
               'font-mono text-xs',
               messageData.type === 'binary'
                ? 'text-orange-600 dark:text-orange-400 bg-orange-50 dark:bg-orange-950/50'
                : 'text-emerald-600 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/50'
             )}>
              {messageData.type === 'binary' ? ll.execution.viewers.binary_label() : ll.execution.viewers.text_label()}
            </Badge>
          )}
        >
          <div className="space-y-2">
            {/* 原始数据 */}
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">{ll.execution.viewers.raw_data()}</div>
              {messageData.type === 'binary' ? (
                <div className="rounded-md border border-orange-200 dark:border-orange-800 overflow-hidden">
                  <div className="px-2 py-1 bg-orange-50 dark:bg-orange-950/50 border-b border-orange-200 dark:border-orange-800 flex items-center gap-2">
                    <span className="text-[10px] font-mono font-medium text-orange-600 dark:text-orange-400">Binary Hex View</span>
                    <span className="text-[10px] text-orange-500 dark:text-orange-500">
                      {dataToBytes(messageData.data).length} bytes
                    </span>
                  </div>
                  <HexViewer data={dataToBytes(messageData.data)} height={200} readOnly />
                </div>
              ) : (
                <div className="bg-muted rounded p-2">
                  <pre className="text-xs text-foreground whitespace-pre-wrap break-all">
                    {messageData.data}
                  </pre>
                </div>
              )}
            </div>

            {/* 解析后的 JSON */}
            {messageData.parsed !== null && messageData.parsed !== undefined && (
              <div>
                <div className="text-xs font-medium text-muted-foreground mb-1.5">{ll.execution.viewers.parsed_json()}</div>
                <div className="bg-muted rounded p-2">
                  <RawJsonViewer data={messageData.parsed} />
                </div>
              </div>
            )}
          </div>
        </CollapsibleSection>
      ) : !isTimedOut && !hasError && (
        <CollapsibleSection
          title={ll.execution.viewers.matched_message()}
          icon={<MessageSquare className="w-4 h-4" />}
          defaultOpen={false}
        >
          <p className="text-sm text-muted-foreground italic">{ll.execution.viewers.no_matched_message()}</p>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      <CollapsibleSection
        title={ll.execution.viewers.timing_breakdown_short()}
        icon={<Clock className="w-4 h-4" />}
        defaultOpen={false}
        extra={(
          <span className="text-xs font-mono text-violet-600 font-medium">
            {ll.execution.viewers.total_label()} {report.duration_ms}ms
          </span>
        )}
      >
        <div className="grid grid-cols-2 gap-2 text-sm">
          <div className="flex justify-between items-center p-2 bg-muted rounded">
            <span className="text-muted-foreground">{ll.execution.viewers.ws_timing_send()}</span>
            <span className="font-mono text-foreground">{formatDuration(sendDuration)}</span>
          </div>
          <div className="flex justify-between items-center p-2 bg-muted rounded">
            <span className="text-muted-foreground">{ll.execution.viewers.ws_timing_collect()}</span>
            <span className="font-mono text-foreground">{formatDuration(collectDuration)}</span>
          </div>
        </div>
      </CollapsibleSection>

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
});
