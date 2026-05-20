/**
 * WsCloseViewer - WebSocket Close 节点执行详情查看器
 * 结构化展示 WS Close 连接信息和关闭状态
 */
import { useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  XCircle,
  Link,
  CheckCircle2,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { VariableChangesCard } from './VariableChangesCard';
import { getLL } from '@/i18n/i18n-lazy';

interface WsCloseViewerProps {
  /** 节点执行报告 */
  report: NodeExecutionReport;
}

/**
 * 获取 WebSocket 关闭码对应的描述
 */
const getCloseCodeDescription = (code: number): string => {
  const descriptions: Record<number, string> = {
    1000: '正常关闭',
    1001: '端点离开',
    1002: '协议错误',
    1003: '不支持的数据类型',
    1005: '无状态码',
    1006: '连接异常关闭',
    1007: '数据类型不一致',
    1008: '违反策略',
    1009: '消息过大',
    1010: '缺少扩展',
    1011: '内部错误',
    1012: '服务重启',
    1013: '稍后重试',
    1015: 'TLS 握手失败',
  };
  return descriptions[code] || '未知状态码';
};

/**
 * 获取 WebSocket 关闭码对应的颜色类名
 */
const getCloseCodeColor = (code: number): string => {
  if (code === 1000) return 'text-emerald-600 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/50';
  if (code >= 1001 && code <= 1015) return 'text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-950/50';
  return 'text-muted-foreground bg-muted';
};

/**
 * 折叠面板组件
 */
interface CollapsibleSectionProps {
  title: string;
  icon: React.ReactNode;
  defaultOpen?: boolean;
  children: React.ReactNode;
}

function CollapsibleSection({ title, icon, defaultOpen = false, children }: CollapsibleSectionProps) {
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
      </button>
      {isOpen && <div className="p-3 border-t border-border bg-background">{children}</div>}
    </div>
  );
}

/**
 * WebSocket Close 输出数据结构
 */
interface WsCloseOutputData {
  success?: boolean;
  url?: string;
  close_code?: number;
  close_reason?: string;
}

export function WsCloseViewer({ report }: WsCloseViewerProps) {
  const ll = getLL();
  // 从 output_data 提取 WS Close 数据
  const outputData = report.output_data as WsCloseOutputData | undefined;
  
  const success = outputData?.success;
  const url = outputData?.url;
  const closeCode = outputData?.close_code;
  const closeReason = outputData?.close_reason;

  return (
    <div className="space-y-3">
      {/* 连接信息 */}
      <CollapsibleSection
        title={ll.execution.viewers.connection_info()}
        icon={<Link className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          {url ? (
            <div className="flex items-start gap-2">
              <span className="text-xs font-medium text-muted-foreground shrink-0">URL</span>
              <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
                {url}
              </code>
            </div>
          ) : (
            <p className="text-sm text-muted-foreground italic">{ll.execution.viewers.connection_id_unknown()}</p>
          )}
        </div>
      </CollapsibleSection>

      {/* 关闭状态 */}
      <CollapsibleSection
        title={ll.execution.viewers.close_status()}
        icon={<XCircle className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* 执行状态 */}
          <div className="flex items-center gap-3">
            {success !== undefined ? (
              <>
                 <Badge className={cn(
                   'font-mono text-base px-3 py-1',
                  success ? 'text-emerald-600 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/50' : 'text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/50'
                 )}>
                  {success ? ll.execution.viewers.success() : ll.execution.viewers.fail()}
                </Badge>
                <div className="text-muted-foreground">
                  {success ? (
                    <CheckCircle2 className="w-4 h-4 text-emerald-600" />
                  ) : (
                     <XCircle className="w-4 h-4 text-red-600 dark:text-red-400" />
                  )}
                </div>
              </>
            ) : (
              <span className="text-sm text-muted-foreground italic">{ll.execution.viewers.no_execution_status()}</span>
            )}
          </div>

          {/* 关闭码 */}
          {closeCode !== undefined && (
            <div className="flex items-start gap-3">
              <span className="text-xs font-medium text-muted-foreground shrink-0 mt-1">{ll.execution.viewers.close_code()}</span>
              <div className="flex-1">
                <Badge className={cn('font-mono text-sm px-2 py-0.5', getCloseCodeColor(closeCode))}>
                  {closeCode}
                </Badge>
                <p className="text-sm text-muted-foreground mt-1">
                  {getCloseCodeDescription(closeCode)}
                </p>
              </div>
            </div>
          )}

          {/* 关闭原因 */}
          {closeReason && (
            <div className="flex items-start gap-3">
              <span className="text-xs font-medium text-muted-foreground shrink-0 mt-1">{ll.execution.viewers.close_reason()}</span>
              <div className="flex-1 bg-muted rounded px-3 py-2">
                <p className="text-sm text-foreground">{closeReason}</p>
              </div>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
