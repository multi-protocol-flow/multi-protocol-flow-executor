/**
 * HttpNodeViewer - HTTP 节点执行详情查看器
 * 结构化展示 HTTP 请求配置、响应状态、响应头、响应体和耗时分解
 */
import { useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Send,
  Globe,
  Clock,
  ArrowDownToLine,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { RawJsonViewer } from './RawJsonViewer';
import { VariableChangesCard } from './VariableChangesCard';

interface HttpNodeViewerProps {
  /** 节点执行报告 */
  report: NodeExecutionReport;
}

/**
 * 获取 HTTP 状态码对应的颜色类名
 */
const getStatusColor = (status: number): string => {
  if (status >= 200 && status < 300) return 'text-emerald-600 bg-emerald-50';
  if (status >= 400 && status < 500) return 'text-amber-600 bg-amber-50';
  if (status >= 500) return 'text-red-600 bg-red-50';
  return 'text-slate-600 bg-slate-50';
};

/**
 * 获取 HTTP 方法对应的颜色类名
 */
const getMethodColor = (method: string): string => {
  const methodColors: Record<string, string> = {
    GET: 'text-blue-600 bg-blue-50',
    POST: 'text-emerald-600 bg-emerald-50',
    PUT: 'text-amber-600 bg-amber-50',
    PATCH: 'text-purple-600 bg-purple-50',
    DELETE: 'text-red-600 bg-red-50',
  };
  return methodColors[method.toUpperCase()] || 'text-slate-600 bg-slate-50';
};

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
 * Key-Value 列表组件
 */
interface KeyValueListProps {
  data: Record<string, unknown> | undefined | null;
}

function KeyValueList({ data }: KeyValueListProps) {
  if (!data || Object.keys(data).length === 0) {
    return <p className="text-sm text-muted-foreground italic">无数据</p>;
  }

  return (
    <div className="space-y-1.5">
      {Object.entries(data).map(([key, value]) => (
        <div key={key} className="flex items-start gap-2 text-sm">
          <span className="font-mono text-slate-600 min-w-0 shrink-0">{key}:</span>
          <span className="text-slate-800 break-all">
            {typeof value === 'object' ? JSON.stringify(value) : String(value)}
          </span>
        </div>
      ))}
    </div>
  );
}

/**
 * HTTP 请求配置数据
 */
interface HttpRequestConfig {
  url?: string;
  method?: string;
  headers?: Record<string, unknown>;
  body?: unknown;
  body_type?: string;
}

/**
 * HTTP 响应数据
 */
interface HttpOutputData {
  success?: boolean;
  status?: number;
  headers?: Record<string, unknown>;
  body?: unknown;
  request?: HttpRequestConfig;
  timing?: {
    dns_ms?: number;
    connect_ms?: number;
    tls_ms?: number;
    ttfb_ms?: number;
    download_ms?: number;
    total_ms?: number;
  };
}

export function HttpNodeViewer({ report }: HttpNodeViewerProps) {
  // 从 output_data 提取 HTTP 数据
  const outputData = report.output_data as HttpOutputData | undefined;
  const requestData = outputData?.request;
  const resolvedConfig = report.resolved_config as HttpRequestConfig | undefined;

  // 优先使用 resolved_config，否则使用 output_data.request
  const config: HttpRequestConfig = {
    url: resolvedConfig?.url || requestData?.url || '',
    method: resolvedConfig?.method || requestData?.method || 'GET',
    headers: resolvedConfig?.headers || requestData?.headers || {},
    body: resolvedConfig?.body ?? requestData?.body,
    body_type: resolvedConfig?.body_type || requestData?.body_type || 'none',
  };

  const status = outputData?.status;
  const headers = outputData?.headers;
  const body = outputData?.body;
  const timing = outputData?.timing;

  return (
    <div className="space-y-3">
      {/* 请求配置 */}
      <CollapsibleSection
        title="请求配置"
        icon={<Send className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* URL 和 Method */}
          <div className="flex items-center gap-2">
            <Badge className={cn('font-mono text-xs shrink-0', getMethodColor(config.method || 'GET'))}>
              {config.method}
            </Badge>
            <code className="flex-1 text-sm font-mono text-slate-700 break-all bg-slate-50 px-2 py-1 rounded">
              {config.url || <span className="text-muted-foreground italic">未配置 URL</span>}
            </code>
          </div>

          {/* Headers */}
          {config.headers && Object.keys(config.headers).length > 0 && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">请求头</div>
              <KeyValueList data={config.headers} />
            </div>
          )}

          {/* Body */}
          {config.body_type !== 'none' && config.body !== undefined && config.body !== null && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">
                请求体 <span className="font-normal">({config.body_type})</span>
              </div>
              <div className="bg-slate-50 rounded p-2">
                <RawJsonViewer data={config.body} />
              </div>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 响应状态 */}
      <CollapsibleSection
        title="响应状态"
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="flex items-center gap-3">
          {status !== undefined ? (
            <>
              <Badge className={cn('font-mono text-xs px-2 py-0.5', getStatusColor(status))}>
                {status}
              </Badge>
              <span className="text-sm text-muted-foreground">
                {status >= 200 && status < 300 && '请求成功'}
                {status >= 300 && status < 400 && '重定向'}
                {status >= 400 && status < 500 && '客户端错误'}
                {status >= 500 && '服务器错误'}
              </span>
            </>
          ) : (
            <span className="text-sm text-muted-foreground italic">无响应状态</span>
          )}
        </div>
      </CollapsibleSection>

      {/* 响应头 */}
      <CollapsibleSection
        title="响应头"
        icon={<ArrowDownToLine className="w-4 h-4" />}
        defaultOpen={false}
      >
        <KeyValueList data={headers} />
      </CollapsibleSection>

      {/* 响应体 */}
      {body !== undefined && (
        <CollapsibleSection
          title="响应体"
          icon={<ArrowDownToLine className="w-4 h-4" />}
          defaultOpen={false}
        >
          <div className="bg-slate-50 rounded p-2">
            <RawJsonViewer data={body} />
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {timing && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={timing.total_ms !== undefined && (
            <span className="text-xs font-mono text-violet-600 font-medium">
              总计 {timing.total_ms}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            {timing.dns_ms !== undefined && (
              <div className="flex justify-between items-center p-2 bg-slate-50 rounded">
                <span className="text-muted-foreground">DNS 解析</span>
                <span className="font-mono text-slate-700">{timing.dns_ms}ms</span>
              </div>
            )}
            {timing.connect_ms !== undefined && (
              <div className="flex justify-between items-center p-2 bg-slate-50 rounded">
                <span className="text-muted-foreground">TCP 连接</span>
                <span className="font-mono text-slate-700">{timing.connect_ms}ms</span>
              </div>
            )}
            {timing.tls_ms !== undefined && (
              <div className="flex justify-between items-center p-2 bg-slate-50 rounded">
                <span className="text-muted-foreground">TLS 握手</span>
                <span className="font-mono text-slate-700">{timing.tls_ms}ms</span>
              </div>
            )}
            {timing.ttfb_ms !== undefined && (
              <div className="flex justify-between items-center p-2 bg-slate-50 rounded">
                <span className="text-muted-foreground">首字节时间</span>
                <span className="font-mono text-slate-700">{timing.ttfb_ms}ms</span>
              </div>
            )}
            {timing.download_ms !== undefined && (
              <div className="flex justify-between items-center p-2 bg-slate-50 rounded">
                <span className="text-muted-foreground">下载耗时</span>
                <span className="font-mono text-slate-700">{timing.download_ms}ms</span>
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
