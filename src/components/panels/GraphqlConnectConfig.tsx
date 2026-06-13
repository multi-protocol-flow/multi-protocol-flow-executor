/**
 * GraphQL Connect 节点配置组件
 * 支持 HTTP 端点、WebSocket 端点、请求头、连接超时、连接 ID
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef, useMemo } from 'react';
import {
  graphqlConnectConfigSchema,
  type GraphqlConnectConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Plug, Plus, Trash2, Info, Loader2, FileJson } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { GraphqlConnectConfig as GraphqlConnectConfigType } from '@/types';
import { cn } from '@/lib/utils';
import { introspectSchema } from '@/services/tauri/graphql';

interface GraphqlConnectConfigProps {
  data: GraphqlConnectConfigType;
  onSave: (data: GraphqlConnectConfigType) => void;
}

interface HeaderEntry {
  key: string;
  value: string;
}

function toFormData(data: GraphqlConnectConfigType): GraphqlConnectConfigFormData {
  return {
    type: 'graphql_connect',
    endpoint: data.endpoint || '',
    ws_endpoint: data.ws_endpoint || '',
    headers: data.headers,
    connection_timeout_ms: data.connection_timeout_ms ?? 30000,
    connection_id: data.connection_id || '',
  };
}

function fromFormData(formData: GraphqlConnectConfigFormData): GraphqlConnectConfigType {
  const headers = formData.headers;
  const hasHeaders = headers && Object.keys(headers).some(k => k.trim());

  return {
    type: 'graphql_connect',
    endpoint: formData.endpoint,
    ws_endpoint: formData.ws_endpoint || undefined,
    headers: hasHeaders ? headers : undefined,
    connection_timeout_ms: formData.connection_timeout_ms || undefined,
    connection_id: formData.connection_id || undefined,
  };
}

export function GraphqlConnectConfig({ data, onSave }: GraphqlConnectConfigProps) {
  const {
    register,
    handleSubmit,
    reset,
    getValues,
    setValue,
    watch,
    formState: { errors, isDirty },
  } = useForm<GraphqlConnectConfigFormData>({
    resolver: zodResolver(graphqlConnectConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

  useEffect(() => {
    return () => {
      if (!isActuallyDirtyRef.current) return;
      try {
        const formData = getValues();
        const config = fromFormData(formData);
        onSave(config);
      } catch {
        // 静默处理卸载时的保存错误
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const safeSetTimeout = useCallback((fn: () => void, ms: number) => {
    const id = setTimeout(() => {
      pendingTimersRef.current.delete(id);
      fn();
    }, ms);
    pendingTimersRef.current.add(id);
  }, []);

  const [isIntrospecting, setIsIntrospecting] = useState(false);
  const [introspectResult, setIntrospectResult] = useState<{ success: boolean; message: string; schema?: Record<string, unknown> } | null>(null);

  const [headerEntries, setHeaderEntries] = useState<HeaderEntry[]>(() => {
    if (!data.headers) return [];
    return Object.entries(data.headers).map<HeaderEntry>(([key, value]) => ({ key, value: String(value) }));
  });

  /** 将 headerEntries 转换为 Record<string, string> 用于 IPC 调用 */
  const headerMap = useMemo(() => {
    const map: Record<string, string> = {};
    headerEntries.forEach(entry => {
      if (entry.key.trim()) map[entry.key.trim()] = entry.value;
    });
    return map;
  }, [headerEntries]);

  const handleIntrospectSchema = useCallback(async () => {
    const endpoint = watch('endpoint')?.trim();
    if (!endpoint) {
      setIntrospectResult({
        success: false,
        message: getLL().panels.graphqlConnect.introspect_error(),
      });
      return;
    }

    setIsIntrospecting(true);
    setIntrospectResult(null);

    try {
      const result = await introspectSchema({ endpoint, headers: Object.keys(headerMap).length > 0 ? headerMap : undefined });

      if (result.success) {
        const typeCount = result.schema ? Object.keys(result.schema).length : 0;
        setIntrospectResult({
          success: true,
          message: getLL().panels.graphqlConnect.introspect_success({ typeCount }),
          schema: result.schema || undefined,
        });
        // 保存发现的 Schema 和查询模板到节点配置
        const currentData = getValues();
        onSave({
          type: 'graphql_connect',
          endpoint: currentData.endpoint,
          ws_endpoint: currentData.ws_endpoint || undefined,
          headers: Object.keys(headerMap).length > 0 ? headerMap : undefined,
          connection_timeout_ms: currentData.connection_timeout_ms || undefined,
          connection_id: currentData.connection_id || undefined,
          discovered_schema: result.schema ?? undefined,
          discovered_queries: result.query_templates,
        });
      } else {
        setIntrospectResult({
          success: false,
          message: result.error || getLL().panels.graphqlConnect.introspect_failed(),
        });
      }
    } catch (error) {
      setIntrospectResult({
        success: false,
        message: typeof error === 'string' ? error : (error instanceof Error ? error.message : String(error)),
      });
    } finally {
      setIsIntrospecting(false);
    }
  }, [watch, headerMap, getValues, onSave]);

  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  useEffect(() => {
    if (data.headers) {
      setHeaderEntries(Object.entries(data.headers).map<HeaderEntry>(([key, value]) => ({ key, value: String(value) })));
    } else {
      setHeaderEntries([]);
    }
  }, [data.headers]);

  const syncHeaders = useCallback((entries: HeaderEntry[]) => {
    setHeaderEntries(entries);
    const record: Record<string, string> = {};
    entries.forEach(e => {
      if (e.key.trim()) record[e.key.trim()] = e.value;
    });
    setValue('headers', Object.keys(record).length > 0 ? record : undefined, { shouldDirty: true });
  }, [setValue]);

  const handleAddHeader = useCallback(() => {
    const newEntries = [...headerEntries, { key: '', value: '' }];
    syncHeaders(newEntries);
    safeSetTimeout(saveData, 100);
  }, [headerEntries, syncHeaders, safeSetTimeout]);

  const handleRemoveHeader = useCallback((index: number) => {
    const newEntries = headerEntries.filter((_, i) => i !== index);
    syncHeaders(newEntries);
    safeSetTimeout(saveData, 100);
  }, [headerEntries, syncHeaders, safeSetTimeout]);

  const handleHeaderChange = useCallback((index: number, field: 'key' | 'value', val: string) => {
    const newEntries = headerEntries.map((entry, i) =>
      i === index ? { ...entry, [field]: val } : entry
    );
    syncHeaders(newEntries);
  }, [headerEntries, syncHeaders]);

  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 GraphQL Connect 配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  const handleBlur = useCallback(() => {
    safeSetTimeout(() => {
      if (!isActuallyDirtyRef.current) return;
      saveData();
    }, 100);
  }, [safeSetTimeout, saveData]);

  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-indigo-200/50 bg-gradient-to-br from-indigo-50/80 to-violet-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-indigo-900 flex items-center gap-2">
            <Plug className="h-4 w-4" />
            {getLL().panels.graphqlConnect.title()}
          </h4>
          <p className="text-sm leading-relaxed text-indigo-700/90 whitespace-normal break-words">
            {getLL().panels.graphqlConnect.description()}
          </p>
        </div>
      </div>

      {/* HTTP 端点 */}
      <div className="space-y-2">
        <Label htmlFor="endpoint" className="text-sm font-medium text-foreground">
          {getLL().panels.graphqlConnect.url()} <span className="text-indigo-500">*</span>
        </Label>
        <div className="relative">
          <Plug className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <Input
            id="endpoint"
            {...register('endpoint')}
            placeholder={getLL().panels.graphqlConnect.urlPlaceholder()}
            className="pl-9 font-mono text-sm"
          />
        </div>
        {errors.endpoint && (
          <p className="text-xs text-indigo-500">{errors.endpoint.message}</p>
        )}
      </div>

      {/* WebSocket 端点 */}
      <div className="space-y-2">
        <Label htmlFor="ws_endpoint" className="text-sm font-medium text-foreground">
          {getLL().panels.graphqlConnect.wsUrl_label()}
        </Label>
        <Input
          id="ws_endpoint"
          {...register('ws_endpoint')}
          placeholder={getLL().panels.graphqlSubscriptionUrl()}
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          {getLL().panels.graphqlConnect.wsUrl_help()}
        </p>
      </div>

      {/* 自定义请求头 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">{getLL().panels.graphqlConnect.headers()}</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddHeader}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            {getLL().panels.add()}
          </Button>
        </div>
        <div className="space-y-2">
          {headerEntries.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">{getLL().panels.graphqlConnect.noHeaders()}</p>
            </div>
          ) : (
            headerEntries.map((entry, index) => (
              <div key={index} className="flex items-center gap-2 p-2 rounded-md border bg-muted/30">
                <Input
                  value={entry.key}
                  onChange={(e) => handleHeaderChange(index, 'key', e.target.value)}
                  onBlur={() => safeSetTimeout(saveData, 100)}
                  placeholder={getLL().panels.http.headerNamePlaceholder()}
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Input
                  value={entry.value}
                  onChange={(e) => handleHeaderChange(index, 'value', e.target.value)}
                  onBlur={() => safeSetTimeout(saveData, 100)}
                  placeholder={getLL().panels.http.value()}
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  onClick={() => handleRemoveHeader(index)}
                  className="h-8 w-8 shrink-0 text-muted-foreground hover:text-indigo-500"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            ))
          )}
        </div>
      </div>

      {/* 连接超时 */}
      <div className="space-y-2">
        <Label htmlFor="connection_timeout_ms" className="text-sm font-medium text-foreground">
          {getLL().panels.graphqlConnect.timeout()}
        </Label>
        <Input
          id="connection_timeout_ms"
          type="number"
          {...register('connection_timeout_ms', { valueAsNumber: true })}
          placeholder="30000"
          className="font-mono text-sm"
        />
      </div>

      {/* 连接 ID */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          {getLL().panels.graphqlConnect.connectionId_label()}
        </Label>
        <Input
          id="connection_id"
          {...register('connection_id')}
          placeholder={getLL().panels.optionalAutoGenerated()}
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          {getLL().panels.graphqlConnect.connectionId_help()}
        </p>
      </div>

      {/* Schema 探索 */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">
            {getLL().panels.graphqlConnect.schemaExplore_sectionTitle()}
          </Label>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={handleIntrospectSchema}
            disabled={isIntrospecting}
            className="h-8 gap-1.5 text-xs"
          >
            {isIntrospecting ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <FileJson className="h-3.5 w-3.5" />
            )}
            {isIntrospecting ? getLL().panels.graphqlConnect.exploring_button() : getLL().panels.graphqlConnect.explore_button()}
          </Button>
        </div>

        {introspectResult && (
          <div className={cn(
            "rounded-md border px-3 py-2 text-xs",
            introspectResult.success
              ? "border-green-200 bg-green-50 text-green-700"
              : "border-red-200 bg-red-50 text-red-700"
          )}>
            {introspectResult.message}
          </div>
        )}

        <p className="text-xs text-muted-foreground">
          {getLL().panels.graphqlConnect.introspect_help()}
        </p>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-indigo-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.graphqlConnect.help_title()}</p>
          <p>{getLL().panels.graphqlConnect.help_line1()}</p>
          <p>{getLL().panels.graphqlConnect.help_line2()}</p>
          <p>{getLL().panels.graphqlConnect.help_line3()}</p>
        </div>
      </div>
    </form>
  );
}
