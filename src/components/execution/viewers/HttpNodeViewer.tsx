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
import type { NodeExecutionReport, FormField } from '@/types';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { RawJsonViewer } from './RawJsonViewer';
import { VariableChangesCard } from './VariableChangesCard';
import { getLL } from '@/i18n/i18n-lazy';

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
  return 'text-muted-foreground bg-muted';
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
  return methodColors[method.toUpperCase()] || 'text-muted-foreground bg-muted';
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
  const ll = getLL();
  if (!data || Object.keys(data).length === 0) {
    return <p className="text-sm text-muted-foreground italic">{ll.execution.viewers.no_data()}</p>;
  }

  return (
    <div className="space-y-1.5">
      {Object.entries(data).map(([key, value]) => (
        <div key={key} className="flex items-start gap-2 text-sm">
          <span className="font-mono text-muted-foreground min-w-0 shrink-0">{key}:</span>
          <span className="text-foreground break-all">
            {typeof value === 'object' ? JSON.stringify(value) : String(value)}
          </span>
        </div>
      ))}
    </div>
  );
}

// ============================================================================
// Body 类型辅助工具
// ============================================================================

/** 类型守卫：带 fields 数组的 body 对象 */
function isFieldsBody(body: unknown): body is { fields: FormField[] } {
  if (typeof body !== 'object' || body === null) return false;
  const obj = body as Record<string, unknown>;
  return 'fields' in obj && Array.isArray(obj.fields);
}

/** 类型守卫：二进制 body 信息 */
function isBinaryBody(body: unknown): body is { type?: string; path?: string; size?: number } {
  return typeof body === 'object' && body !== null;
}

/** 从 body 和 form_fields 中提取 form-data 字段 */
function extractFormFields(body: unknown, formFields?: FormField[]): FormField[] {
  if (formFields && formFields.length > 0) return formFields;
  if (isFieldsBody(body)) return body.fields;
  return [];
}

/** 从 body 中提取 urlencoded 字段 */
function extractUrlEncodedFields(body: unknown): Array<{ key: string; value: string }> {
  if (isFieldsBody(body)) return body.fields;
  return [];
}

/** 从 body 中提取二进制文件信息 */
function extractBinaryInfo(body: unknown, filePath?: string): { path?: string; size?: number } {
  const result: { path?: string; size?: number } = {};
  if (filePath) result.path = filePath;
  if (isBinaryBody(body)) {
    if (!result.path && body.path) result.path = body.path;
    if (body.size !== undefined) result.size = body.size;
  }
  return result;
}

/** 格式化文件大小 */
function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * Form-Data 字段表格
 */
function FormDataFieldsTable({ fields }: { fields: FormField[] }) {
  const ll = getLL();
  return (
    <div className="border border-border rounded overflow-hidden text-sm">
      <div className="grid grid-cols-[1fr_70px_2fr] bg-muted/50">
        <div className="px-2.5 py-1.5 font-medium border-r border-b border-border text-xs text-muted-foreground">Key</div>
        <div className="px-2.5 py-1.5 font-medium border-r border-b border-border text-xs text-muted-foreground text-center">Type</div>
        <div className="px-2.5 py-1.5 font-medium border-b border-border text-xs text-muted-foreground">Value</div>
      </div>
      {fields.map((field, i) => (
        <div key={`${field.key}-${i}`} className="grid grid-cols-[1fr_70px_2fr]">
          <div className="px-2.5 py-1.5 border-r border-b border-border font-mono text-foreground break-all">{field.key}</div>
          <div className="px-2.5 py-1.5 border-r border-b border-border text-center">
            <span className={cn(
              'text-[10px] px-1.5 py-0.5 rounded font-mono',
              field.type === 'file' ? 'text-amber-600 bg-amber-50' : 'text-muted-foreground bg-muted'
            )}>
              {field.type === 'file' ? 'file' : 'text'}
            </span>
          </div>
          <div className="px-2.5 py-1.5 border-b border-border break-all">
            {field.type === 'file' ? (
              <span className="font-mono text-muted-foreground">{field.file_path || ll.execution.viewers.file_not_selected()}</span>
            ) : (
              <span className="text-foreground">{field.value}</span>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}

/**
 * URL-Encoded 字段列表
 */
function UrlEncodedFieldsList({ fields }: { fields: Array<{ key: string; value: string }> }) {
  return (
    <div className="space-y-1.5">
      {fields.map((field) => (
        <div key={field.key} className="flex items-start gap-2 text-sm">
          <span className="font-mono text-muted-foreground min-w-0 shrink-0">{field.key}:</span>
          <span className="text-foreground break-all">{field.value}</span>
        </div>
      ))}
    </div>
  );
}

/**
 * 二进制文件信息展示
 */
function BinaryFileDisplay({ path, size }: { path?: string; size?: number }) {
  const ll = getLL();
  return (
    <div className="bg-muted rounded p-2 space-y-1 text-sm">
      {path && (
        <div className="flex items-start gap-2">
          <span className="text-muted-foreground shrink-0">{ll.execution.viewers.file_path()}</span>
          <code className="text-foreground break-all">{path}</code>
        </div>
      )}
      {size !== undefined && (
        <div className="flex items-start gap-2">
          <span className="text-muted-foreground shrink-0">{ll.execution.viewers.file_size()}</span>
          <span className="text-foreground">{formatFileSize(size)}</span>
        </div>
      )}
      {!path && size === undefined && (
        <span className="text-muted-foreground italic">{ll.execution.viewers.no_file_info()}</span>
      )}
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
  content_type?: string;
  form_fields?: FormField[];
  file_path?: string;
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
    tls_ms?: number | null;
    ttfb_ms?: number;
    download_ms?: number;
    total_ms?: number;
  };
  /** 重定向历史 */
  redirect_history?: Array<{ status: number; location: string }>;
  /** 响应 Cookie */
  cookies_received?: Array<{ name: string; value: string; domain?: string; path?: string }>;
  /** 重试次数 */
  retry_count?: number;
}

export function HttpNodeViewer({ report }: HttpNodeViewerProps) {
  const ll = getLL();
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
    content_type: resolvedConfig?.content_type || requestData?.content_type,
    form_fields: resolvedConfig?.form_fields || requestData?.form_fields,
    file_path: resolvedConfig?.file_path || requestData?.file_path,
  };

  const status = outputData?.status;
  const headers = outputData?.headers;
  const body = outputData?.body;
  const timing = outputData?.timing;

  // 请求体类型相关计算
  const bodyType = config.body_type || 'body';
  const bodyLabel = bodyType === 'body' && config.content_type
    ? `body · ${config.content_type}`
    : bodyType;

  const hasBody = config.body !== undefined && config.body !== null;

  const formDataFields = bodyType === 'form-data'
    ? extractFormFields(config.body, config.form_fields) : null;
  const urlEncodedFields = bodyType === 'x-www-form-urlencoded'
    ? extractUrlEncodedFields(config.body) : null;
  const binaryInfo = bodyType === 'binary'
    ? extractBinaryInfo(config.body, config.file_path) : null;

  const showBody = bodyType !== 'none' && (
    hasBody
    || (formDataFields !== null && formDataFields.length > 0)
    || (urlEncodedFields !== null && urlEncodedFields.length > 0)
    || binaryInfo !== null
  );

  return (
    <div className="space-y-3">
      {/* 请求配置 */}
      <CollapsibleSection
        title={ll.execution.viewers.request_config()}
        icon={<Send className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* URL 和 Method */}
          <div className="flex items-center gap-2">
            <Badge className={cn('font-mono text-xs shrink-0', getMethodColor(config.method || 'GET'))}>
              {config.method}
            </Badge>
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {config.url || <span className="text-muted-foreground italic">{ll.execution.viewers.not_configured_url()}</span>}
            </code>
          </div>

          {/* Headers */}
          {config.headers && Object.keys(config.headers).length > 0 && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">{ll.execution.viewers.request_headers()}</div>
              <KeyValueList data={config.headers} />
            </div>
          )}

          {/* Body */}
          {showBody && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">
                {ll.execution.viewers.request_body()} <span className="font-normal">({bodyLabel})</span>
              </div>
              {bodyType === 'body' && hasBody && (
                <div className="bg-muted rounded p-2">
                  <RawJsonViewer data={config.body} />
                </div>
              )}
              {bodyType === 'form-data' && formDataFields && formDataFields.length > 0 && (
                <FormDataFieldsTable fields={formDataFields} />
              )}
              {bodyType === 'form-data' && (!formDataFields || formDataFields.length === 0) && hasBody && (
                <div className="bg-muted rounded p-2">
                  <RawJsonViewer data={config.body} />
                </div>
              )}
              {bodyType === 'x-www-form-urlencoded' && urlEncodedFields && urlEncodedFields.length > 0 && (
                <UrlEncodedFieldsList fields={urlEncodedFields} />
              )}
              {bodyType === 'x-www-form-urlencoded' && (!urlEncodedFields || urlEncodedFields.length === 0) && hasBody && (
                <div className="bg-muted rounded p-2">
                  <RawJsonViewer data={config.body} />
                </div>
              )}
              {bodyType === 'binary' && binaryInfo && (
                <BinaryFileDisplay path={binaryInfo.path} size={binaryInfo.size} />
              )}
            </div>
          )}

          {/* 重试信息 */}
          {outputData?.retry_count != null && outputData.retry_count > 0 && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{ll.execution.viewers.retry_label()}</span>
              <Badge variant="outline" className="text-xs bg-blue-50 text-blue-700 border-blue-200">
                {ll.execution.viewers.retry_count({ count: outputData.retry_count })}
              </Badge>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 响应状态 */}
      <CollapsibleSection
        title={ll.execution.viewers.response_status()}
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
                {status >= 200 && status < 300 && ll.execution.viewers.status_text.success()}
                {status >= 300 && status < 400 && ll.execution.viewers.status_text.redirect()}
                {status >= 400 && status < 500 && ll.execution.viewers.status_text.client_error()}
                {status >= 500 && ll.execution.viewers.status_text.server_error()}
              </span>
            </>
          ) : (
            <span className="text-sm text-muted-foreground italic">{ll.execution.viewers.no_response_status()}</span>
          )}
        </div>
      </CollapsibleSection>

      {/* 重定向历史 */}
      {outputData?.redirect_history && outputData.redirect_history.length > 0 && (
        <CollapsibleSection
          title={ll.execution.viewers.redirect_chain({ count: outputData.redirect_history.length })}
          icon={<Globe className="w-4 h-4" />}
          defaultOpen={false}
        >
          <div className="space-y-1.5">
            {outputData.redirect_history.map((redirect, index) => (
              <div key={index} className="flex items-center gap-2 text-sm">
                <Badge variant="outline" className="text-xs bg-amber-50 text-amber-700 border-amber-200 shrink-0">
                  {redirect.status}
                </Badge>
                <span className="text-muted-foreground shrink-0">→</span>
                <code className="text-xs bg-muted px-1.5 py-0.5 rounded break-all">
                  {redirect.location}
                </code>
              </div>
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 响应头 */}
      <CollapsibleSection
        title={ll.execution.viewers.response_headers()}
        icon={<ArrowDownToLine className="w-4 h-4" />}
        defaultOpen={false}
      >
        <KeyValueList data={headers} />
      </CollapsibleSection>

      {/* 响应 Cookie */}
      {outputData?.cookies_received && outputData.cookies_received.length > 0 && (
        <CollapsibleSection
          title={ll.execution.viewers.response_cookie({ count: outputData.cookies_received.length })}
          icon={<ArrowDownToLine className="w-4 h-4" />}
          defaultOpen={false}
        >
          <div className="border border-border rounded overflow-hidden text-sm">
            <div className="grid grid-cols-[1fr_1.5fr_0.8fr_0.5fr] bg-muted/50">
              <div className="px-2.5 py-1.5 font-medium border-r border-b border-border text-xs text-muted-foreground">{ll.execution.viewers.cookie_name()}</div>
              <div className="px-2.5 py-1.5 font-medium border-r border-b border-border text-xs text-muted-foreground">{ll.execution.viewers.cookie_value()}</div>
              <div className="px-2.5 py-1.5 font-medium border-r border-b border-border text-xs text-muted-foreground">{ll.execution.viewers.cookie_domain()}</div>
              <div className="px-2.5 py-1.5 font-medium border-b border-border text-xs text-muted-foreground">{ll.execution.viewers.cookie_path()}</div>
            </div>
            {outputData.cookies_received.map((cookie, index) => (
              <div key={index} className="grid grid-cols-[1fr_1.5fr_0.8fr_0.5fr]">
                <div className="px-2.5 py-1.5 border-r border-b border-border font-mono text-foreground break-all">{cookie.name}</div>
                <div className="px-2.5 py-1.5 border-r border-b border-border font-mono text-muted-foreground break-all">{cookie.value}</div>
                <div className="px-2.5 py-1.5 border-r border-b border-border text-muted-foreground">{cookie.domain ?? '-'}</div>
                <div className="px-2.5 py-1.5 border-b border-border text-muted-foreground">{cookie.path ?? '-'}</div>
              </div>
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 响应体 */}
      {body !== undefined && (
        <CollapsibleSection
          title={ll.execution.viewers.response_body()}
          icon={<ArrowDownToLine className="w-4 h-4" />}
          defaultOpen={false}
        >
          <div className="bg-muted rounded p-2">
            <RawJsonViewer data={body} />
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {timing && (
        <CollapsibleSection
          title={ll.execution.viewers.timing_breakdown()}
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={timing.total_ms !== undefined && (
            <span className="text-xs font-mono text-violet-600 font-medium">
              {ll.execution.viewers.timing_total({ total: timing.total_ms })}
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            {timing.dns_ms !== undefined && (
              <div className="flex justify-between items-center p-2 bg-muted rounded">
                <span className="text-muted-foreground">{ll.execution.viewers.timing_dns()}</span>
                <span className="font-mono text-foreground">{timing.dns_ms}ms</span>
              </div>
            )}
            {timing.connect_ms !== undefined && (
              <div className="flex justify-between items-center p-2 bg-muted rounded">
                <span className="text-muted-foreground">{ll.execution.viewers.timing_tcp()}</span>
                <span className="font-mono text-foreground">{timing.connect_ms}ms</span>
              </div>
            )}
            {timing.tls_ms !== undefined && (
              <div className="flex justify-between items-center p-2 bg-muted rounded">
                <span className="text-muted-foreground">{ll.execution.viewers.timing_tls()}</span>
                <span className="font-mono text-foreground">{timing.tls_ms}ms</span>
              </div>
            )}
            {timing.ttfb_ms !== undefined && (
              <div className="flex justify-between items-center p-2 bg-muted rounded">
                <span className="text-muted-foreground">{ll.execution.viewers.timing_ttfb()}</span>
                <span className="font-mono text-foreground">{timing.ttfb_ms}ms</span>
              </div>
            )}
            {timing.download_ms !== undefined && (
              <div className="flex justify-between items-center p-2 bg-muted rounded">
                <span className="text-muted-foreground">{ll.execution.viewers.timing_download()}</span>
                <span className="font-mono text-foreground">{timing.download_ms}ms</span>
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
