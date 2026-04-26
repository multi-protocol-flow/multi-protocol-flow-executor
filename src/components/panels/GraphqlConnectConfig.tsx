/**
 * GraphQL Connect 节点配置组件
 * 支持 HTTP 端点、WebSocket 端点、请求头、连接超时、连接 ID
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef } from 'react';
import {
  graphqlConnectConfigSchema,
  type GraphqlConnectConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Plug, Plus, Trash2, Info } from 'lucide-react';
import type { GraphqlConnectConfig as GraphqlConnectConfigType } from '@/types';

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

  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  const [headerEntries, setHeaderEntries] = useState<HeaderEntry[]>(() => {
    if (!data.headers) return [];
    return Object.entries(data.headers).map<HeaderEntry>(([key, value]) => ({ key, value: String(value) }));
  });

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
            GraphQL 连接节点
          </h4>
          <p className="text-sm leading-relaxed text-indigo-700/90 whitespace-normal break-words">
            建立 GraphQL 连接，配置 HTTP 端点和可选的 WebSocket 端点（用于订阅）。
          </p>
        </div>
      </div>

      {/* HTTP 端点 */}
      <div className="space-y-2">
        <Label htmlFor="endpoint" className="text-sm font-medium text-foreground">
          HTTP 端点 URL <span className="text-indigo-500">*</span>
        </Label>
        <div className="relative">
          <Plug className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <Input
            id="endpoint"
            {...register('endpoint')}
            placeholder="https://api.example.com/graphql"
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
          WebSocket 端点 URL
        </Label>
        <Input
          id="ws_endpoint"
          {...register('ws_endpoint')}
          placeholder="wss://api.example.com/graphql (用于订阅)"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          用于 GraphQL 订阅的 WebSocket 端点，留空则使用 HTTP 端点
        </p>
      </div>

      {/* 自定义请求头 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">自定义请求头（可选）</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddHeader}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            添加
          </Button>
        </div>
        <div className="space-y-2">
          {headerEntries.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">暂无自定义请求头</p>
            </div>
          ) : (
            headerEntries.map((entry, index) => (
              <div key={index} className="flex items-center gap-2 p-2 rounded-md border bg-muted/30">
                <Input
                  value={entry.key}
                  onChange={(e) => handleHeaderChange(index, 'key', e.target.value)}
                  onBlur={() => safeSetTimeout(saveData, 100)}
                  placeholder="Header Name"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Input
                  value={entry.value}
                  onChange={(e) => handleHeaderChange(index, 'value', e.target.value)}
                  onBlur={() => safeSetTimeout(saveData, 100)}
                  placeholder="Value"
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
          连接超时（毫秒）
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
          连接 ID
        </Label>
        <Input
          id="connection_id"
          {...register('connection_id')}
          placeholder="可选，自动生成"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          留空将自动生成连接 ID
        </p>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-indigo-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">GraphQL 连接说明：</p>
          <p>HTTP 端点用于查询 (Query) 和变更 (Mutation)</p>
          <p>WebSocket 端点用于实时订阅 (Subscription)</p>
          <p>可在请求头中配置认证信息 (Authorization)</p>
        </div>
      </div>
    </form>
  );
}
