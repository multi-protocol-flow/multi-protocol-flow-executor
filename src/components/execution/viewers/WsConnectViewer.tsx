/**
 * WsConnectViewer - WebSocket Connect 节点执行详情查看器
 * 结构化展示连接配置、连接状态、首条消息和耗时分解
 */
import { useState } from 'react';
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
import { cn } from '@/lib/utils';
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
    wait_response_ms?: number | null;
    total_ms?: number | null;
  };
}

/**
 * WS Connect 配置数据类型
 */
interface WsConnectConfig {
  url?: string;
  wait_mode?: string;
  timeout_ms?: number;
}

export function WsConnectViewer({ report }: WsConnectViewerProps) {
  const output = report.output_data as WsConnectOutput | undefined;
  const resolvedConfig = report.resolved_config as WsConnectConfig | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  // 配置数据
  const configUrl = resolvedConfig?.url || output?.url || '';
  const waitMode = resolvedConfig?.wait_mode || 'none';
  const timeoutMs = resolvedConfig?.timeout_ms;

  // 输出数据
  const connectionId = output?.connection_id;
  const message = output?.message;
  const connectDuration = output?.timing?.connect_ms;
  const waitDuration = output?.timing?.wait_response_ms;
  const totalDuration = output?.timing?.total_ms ?? ((connectDuration || 0) + (waitDuration || 0));

  return (
    <div className="space-y-3">
      {/* 连接配置 */}
      <CollapsibleSection
        title="连接配置"
        icon={<Wifi className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* URL */}
          <div className="flex items-center gap-2">
            <Badge variant="outline" className="font-mono text-xs text-amber-600 bg-amber-50 border-0 shrink-0">
              WS
            </Badge>
            <code className="flex-1 text-sm font-mono text-slate-700 break-all bg-slate-50 px-2 py-1 rounded">
              {configUrl || <span className="text-muted-foreground italic">未配置 URL</span>}
            </code>
          </div>

          {/* 等待模式 */}
          <div className="flex items-center gap-2 text-sm">
            <span className="text-muted-foreground">等待模式:</span>
            <Badge variant="outline" className="font-mono text-xs">
              {(() => {
                switch (waitMode) {
                  case 'first_message': return '等待第一条消息';
                  case 'condition': return '等待条件匹配';
                  case 'duration': return '等待指定时长';
                  default: return '等待第一条消息';
                }
              })()}
            </Badge>
          </div>

          {/* 超时设置 */}
          {timeoutMs !== undefined && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">超时时间:</span>
              <Badge variant="secondary" className="font-mono text-xs">
                {timeoutMs}ms
              </Badge>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 连接状态 */}
      <CollapsibleSection
        title="连接状态"
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          {/* 状态 */}
          <div className="flex items-center gap-3">
            <Badge className={cn(
              'font-mono text-xs px-2 py-0.5',
              isSuccess ? 'text-emerald-600 bg-emerald-50' : 'text-red-600 bg-red-50'
            )}>
              {isSuccess ? '已连接' : '连接失败'}
            </Badge>
            <span className="text-sm text-muted-foreground">
              {isSuccess ? 'WebSocket 连接建立成功' : '无法建立 WebSocket 连接'}
            </span>
          </div>

          {/* 连接 ID */}
          {connectionId && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">连接 ID:</span>
              <code className="text-xs bg-slate-100 px-2 py-1 rounded font-mono">
                {connectionId}
              </code>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 首条消息 */}
      <CollapsibleSection
        title="首条消息"
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
                <span className="text-muted-foreground">类型:</span>
                <Badge variant="outline" className="font-mono text-xs">
                  {message.type}
                </Badge>
              </div>
            )}
            {/* 消息数据 */}
            {message.data !== undefined && (
              <div>
                <div className="text-xs font-medium text-muted-foreground mb-1.5">数据</div>
                <div className="bg-slate-50 rounded p-2">
                  {typeof message.data === 'string' ? (
                    <pre className="text-xs text-slate-700 whitespace-pre-wrap break-all">
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
          <p className="text-sm text-muted-foreground italic">无首条消息</p>
        )}
      </CollapsibleSection>

      {/* 耗时分解 */}
      {(connectDuration !== undefined || waitDuration !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration > 0 && (
            <span className="text-xs font-mono text-violet-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            {connectDuration !== undefined && (
              <div className="flex justify-between items-center p-2 bg-slate-50 rounded">
                <span className="text-muted-foreground">连接耗时</span>
                <span className="font-mono text-slate-700">{connectDuration}ms</span>
              </div>
            )}
            {waitDuration !== undefined && (
              <div className="flex justify-between items-center p-2 bg-slate-50 rounded">
                <span className="text-muted-foreground">等待消息</span>
                <span className="font-mono text-slate-700">{waitDuration}ms</span>
              </div>
            )}
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
