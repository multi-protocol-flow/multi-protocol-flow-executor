/**
 * WebSocket Send+Wait 节点结构化数据展示组件
 * 
 * 展示 WS Send+Wait 节点的执行数据，包括：
 * - 连接信息（connection_id, url）
 * - 发送内容（sent.data, sent.type）
 * - 响应内容（message.data, message.type, message.parsed）
 * - 耗时分解（send_duration_ms, wait_response_duration_ms）
 */
import { 
  Link, 
  Send, 
  MessageSquare, 
  Clock, 
  AlertTriangle,
  Timer,
  ArrowRightLeft,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { RawJsonViewer } from './RawJsonViewer';
import { VariableChangesCard } from './VariableChangesCard';

/** WS Send+Wait 输出数据结构 */
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

interface WsSendWaitViewerProps {
  /** 节点执行报告 */
  report: NodeExecutionReport;
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
function truncateText(text: string, maxLength: number = 200): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + '...';
}

/**
 * WebSocket Send+Wait 节点数据展示组件
 */
export function WsSendWaitViewer({ report }: WsSendWaitViewerProps) {
  const outputData = report.output_data as WsSendWaitOutput | undefined;
  const hasError = outputData?.error || report.status === 'failed';
  const isTimedOut = outputData?.wait_timed_out === true;

  // 获取连接 URL
  const connectionUrl = outputData?.url;
  const connectionId = outputData?.connection_id;

  // 发送数据
  const sentData = outputData?.sent;
  
  // 响应数据
  const messageData = outputData?.message;

  // 耗时
  const sendDuration = outputData?.send_duration_ms;
  const waitDuration = outputData?.wait_response_duration_ms;

  return (
    <div className="space-y-4">
      {/* 错误信息 */}
      {hasError && outputData?.error && (
        <div className="rounded-lg bg-red-50 border border-red-200/50 p-3">
          <div className="flex items-center gap-1.5 mb-1 text-red-600">
            <AlertTriangle className="w-3.5 h-3.5" />
            <span className="font-medium text-sm">执行错误</span>
          </div>
          <p className="text-red-600 text-sm">{outputData.error}</p>
        </div>
      )}

      {/* 超时警告 */}
      {isTimedOut && !hasError && (
        <div className="rounded-lg bg-amber-50 border border-amber-200/50 p-3">
          <div className="flex items-center gap-1.5 text-amber-600">
            <Timer className="w-3.5 h-3.5" />
            <span className="font-medium text-sm">等待响应超时</span>
          </div>
          <p className="text-amber-600/80 text-xs mt-1">
            在指定时间内未收到匹配的响应消息
          </p>
        </div>
      )}

      {/* 连接信息 */}
      <div className="rounded-lg bg-violet-50/50 border border-violet-200/30 p-3">
        <div className="flex items-center gap-1.5 mb-2 text-violet-700">
          <Link className="w-3.5 h-3.5" />
          <span className="font-medium text-sm">连接信息</span>
        </div>
        <div className="space-y-1.5 text-sm">
          <div className="flex items-center gap-2">
            <span className="text-muted-foreground w-16">连接 ID:</span>
            <Badge variant="secondary" className="font-mono text-xs">
              {connectionId || '-'}
            </Badge>
          </div>
          {connectionUrl && (
            <div className="flex items-start gap-2">
              <span className="text-muted-foreground w-16">URL:</span>
              <code className="text-xs bg-violet-100/50 px-1.5 py-0.5 rounded text-violet-800 break-all">
                {connectionUrl}
              </code>
            </div>
          )}
        </div>
      </div>

      {/* 发送内容 */}
      {sentData && (
        <div className="rounded-lg bg-slate-50/50 border border-slate-200/30 p-3">
          <div className="flex items-center gap-1.5 mb-2 text-slate-700">
            <Send className="w-3.5 h-3.5" />
            <span className="font-medium text-sm">发送内容</span>
            <Badge 
              variant="outline" 
              className={cn(
                'ml-auto text-xs',
                sentData.type === 'binary' 
                  ? 'bg-orange-100 text-orange-700 border-orange-200' 
                  : 'bg-slate-100 text-slate-600 border-slate-200'
              )}
            >
              {sentData.type === 'binary' ? '二进制' : '文本'}
            </Badge>
          </div>
          <pre className="text-xs bg-slate-100/50 border border-slate-200/50 rounded p-2 overflow-x-auto text-slate-700 max-h-40 overflow-y-auto">
            {truncateText(sentData.data, 500)}
          </pre>
        </div>
      )}

      {/* 响应内容 */}
      {messageData ? (
        <div className="rounded-lg bg-emerald-50/50 border border-emerald-200/30 p-3">
          <div className="flex items-center gap-1.5 mb-2 text-emerald-700">
            <MessageSquare className="w-3.5 h-3.5" />
            <span className="font-medium text-sm">响应内容</span>
            <Badge 
              variant="outline" 
              className={cn(
                'ml-auto text-xs',
                messageData.type === 'binary' 
                  ? 'bg-orange-100 text-orange-700 border-orange-200' 
                  : 'bg-emerald-100 text-emerald-600 border-emerald-200'
              )}
            >
              {messageData.type === 'binary' ? '二进制' : '文本'}
            </Badge>
          </div>
          
          {/* 原始数据 */}
          <pre className="text-xs bg-emerald-100/50 border border-emerald-200/50 rounded p-2 overflow-x-auto text-emerald-800 max-h-40 overflow-y-auto mb-2">
            {truncateText(messageData.data, 500)}
          </pre>
          
          {/* 解析后的 JSON（如果有） */}
          {messageData.parsed !== null && messageData.parsed !== undefined && (
            <div className="mt-2">
              <div className="text-xs text-emerald-600/70 mb-1">解析后的 JSON:</div>
              <div className="bg-emerald-100/30 border border-emerald-200/30 rounded p-2 max-h-60 overflow-auto">
                <RawJsonViewer data={messageData.parsed} />
              </div>
            </div>
          )}
        </div>
      ) : !isTimedOut && !hasError && (
        <div className="rounded-lg bg-slate-50/30 border border-slate-200/20 p-3 text-center">
          <div className="text-muted-foreground text-sm">
            <ArrowRightLeft className="w-4 h-4 mx-auto mb-1 opacity-50" />
            未等待响应或无响应数据
          </div>
        </div>
      )}

      {/* 耗时分解 */}
      <div className="rounded-lg bg-slate-50/50 border border-slate-200/30 p-3">
        <div className="flex items-center gap-1.5 mb-2 text-slate-600">
          <Clock className="w-3.5 h-3.5" />
          <span className="font-medium text-sm">耗时分解</span>
        </div>
        <div className="grid grid-cols-2 gap-3 text-sm">
          <div className="flex items-center justify-between bg-white/50 rounded px-2.5 py-1.5 border border-slate-200/30">
            <span className="text-muted-foreground">发送耗时</span>
            <span className="font-mono text-slate-700">
              {formatDuration(sendDuration)}
            </span>
          </div>
          <div className="flex items-center justify-between bg-white/50 rounded px-2.5 py-1.5 border border-slate-200/30">
            <span className="text-muted-foreground">等待耗时</span>
            <span className="font-mono text-slate-700">
              {formatDuration(waitDuration)}
            </span>
          </div>
        </div>
        
        {/* 总耗时（从 report 中获取） */}
        <div className="mt-2 flex items-center justify-between bg-slate-100/50 rounded px-2.5 py-1.5 border border-slate-200/30">
          <span className="text-muted-foreground font-medium">总执行时间</span>
          <span className="font-mono text-slate-800 font-medium">
            {formatDuration(report.duration_ms)}
          </span>
        </div>
      </div>

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
