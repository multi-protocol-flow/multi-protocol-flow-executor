/**
 * WebSocket Connect 节点执行数据展示组件
 * 结构化展示 WS Connect 节点的连接信息、耗时分解和首条消息
 */
import { Wifi, Clock, MessageSquare, CheckCircle2, XCircle } from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';

interface WsConnectViewerProps {
  report: NodeExecutionReport;
}

/**
 * WS Connect 输出数据类型
 */
interface WsConnectOutput {
  success?: boolean;
  url?: string;
  connection_id?: string;
  message?: {
    data?: unknown;
    type?: string;
  };
  connect_duration_ms?: number;
  wait_message_duration_ms?: number;
}

export function WsConnectViewer({ report }: WsConnectViewerProps) {
  const output = report.output_data as WsConnectOutput | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  return (
    <div className="space-y-4">
      {/* 连接信息 */}
      <div className="space-y-2">
        <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
          <Wifi className="w-4 h-4" />
          <span>连接信息</span>
        </div>
        <div className="pl-6 space-y-2">
          {/* 连接状态 */}
          <div className="flex items-center gap-2">
            <span className="text-sm text-muted-foreground">状态:</span>
            {isSuccess ? (
              <Badge className="bg-emerald-100 text-emerald-700 border-emerald-200">
                <CheckCircle2 className="w-3 h-3 mr-1" />
                已连接
              </Badge>
            ) : (
              <Badge className="bg-red-100 text-red-700 border-red-200">
                <XCircle className="w-3 h-3 mr-1" />
                连接失败
              </Badge>
            )}
          </div>

          {/* URL */}
          {output?.url && (
            <div className="flex items-start gap-2">
              <span className="text-sm text-muted-foreground whitespace-nowrap">URL:</span>
              <code className="text-xs bg-slate-100 px-2 py-1 rounded font-mono break-all">
                {output.url}
              </code>
            </div>
          )}

          {/* 连接 ID */}
          {output?.connection_id && (
            <div className="flex items-center gap-2">
              <span className="text-sm text-muted-foreground">连接 ID:</span>
              <code className="text-xs bg-slate-100 px-2 py-1 rounded font-mono">
                {output.connection_id}
              </code>
            </div>
          )}
        </div>
      </div>

      {/* 耗时分解 */}
      {(output?.connect_duration_ms !== undefined || output?.wait_message_duration_ms !== undefined) && (
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
            <Clock className="w-4 h-4" />
            <span>耗时分解</span>
          </div>
          <div className="pl-6 space-y-1.5">
            {output?.connect_duration_ms !== undefined && (
              <div className="flex items-center gap-2">
                <span className="text-sm text-muted-foreground">连接耗时:</span>
                <Badge variant="secondary" className="font-mono text-xs">
                  {output.connect_duration_ms}ms
                </Badge>
              </div>
            )}
            {output?.wait_message_duration_ms !== undefined && (
              <div className="flex items-center gap-2">
                <span className="text-sm text-muted-foreground">等待消息耗时:</span>
                <Badge variant="secondary" className="font-mono text-xs">
                  {output.wait_message_duration_ms}ms
                </Badge>
              </div>
            )}
          </div>
        </div>
      )}

      {/* 首条消息 */}
      <div className="space-y-2">
        <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
          <MessageSquare className="w-4 h-4" />
          <span>首条消息</span>
        </div>
        <div className="pl-6">
          {output?.message ? (
            <div className="space-y-2">
              {/* 消息类型 */}
              {output.message.type && (
                <div className="flex items-center gap-2">
                  <span className="text-sm text-muted-foreground">类型:</span>
                  <Badge variant="outline" className="font-mono text-xs">
                    {output.message.type}
                  </Badge>
                </div>
              )}
              {/* 消息数据 */}
              {output.message.data !== undefined && (
                <div className="space-y-1">
                  <span className="text-sm text-muted-foreground">数据:</span>
                  <pre className="p-3 bg-slate-100 border border-black rounded-lg text-xs overflow-x-auto text-slate-800 max-w-full whitespace-pre-wrap break-all">
                    {typeof output.message.data === 'string'
                      ? output.message.data
                      : JSON.stringify(output.message.data, null, 2)}
                  </pre>
                </div>
              )}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground italic">无首条消息</p>
          )}
        </div>
      </div>
    </div>
  );
}
