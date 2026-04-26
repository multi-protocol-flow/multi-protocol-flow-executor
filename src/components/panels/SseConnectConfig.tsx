/**
 * SSE Connect 节点配置组件
 * 支持 URL、请求头、Last-Event-ID、超时、自动重连、TLS 模式
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef } from 'react';
import {
  sseConnectConfigSchema,
  type SseConnectConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Radio, Plus, Trash2, ChevronDown, Shield, Info } from 'lucide-react';
import type { SseConnectConfig as SseConnectConfigType } from '@/types';
import { cn } from '@/lib/utils';

interface SseConnectConfigProps {
  data: SseConnectConfigType;
  onSave: (data: SseConnectConfigType) => void;
}

/** 请求头条目本地状态 */
interface HeaderEntry {
  key: string;
  value: string;
}

function toFormData(data: SseConnectConfigType): SseConnectConfigFormData {
  return {
    type: 'sse_connect',
    url: data.url || '',
    headers: data.headers,
    last_event_id: data.last_event_id || '',
    timeout_ms: data.timeout_ms ?? 30000,
    auto_reconnect: data.auto_reconnect ?? false,
    reconnect_interval_ms: data.reconnect_interval_ms ?? 3000,
    max_reconnect_attempts: data.max_reconnect_attempts ?? 5,
    tls_mode: data.tls_mode || 'none',
  };
}

function fromFormData(formData: SseConnectConfigFormData): SseConnectConfigType {
  const headers = formData.headers;
  const hasHeaders = headers && Object.keys(headers).some(k => k.trim());

  return {
    type: 'sse_connect',
    url: formData.url,
    headers: hasHeaders ? headers : undefined,
    last_event_id: formData.last_event_id || undefined,
    timeout_ms: formData.timeout_ms,
    auto_reconnect: formData.auto_reconnect || undefined,
    reconnect_interval_ms: formData.auto_reconnect ? (formData.reconnect_interval_ms || undefined) : undefined,
    max_reconnect_attempts: formData.auto_reconnect ? (formData.max_reconnect_attempts || undefined) : undefined,
    tls_mode: formData.tls_mode === 'none' ? undefined : formData.tls_mode,
  };
}

export function SseConnectConfig({ data, onSave }: SseConnectConfigProps) {
  const {
    register,
    handleSubmit,
    reset,
    getValues,
    setValue,
    watch,
    formState: { errors, isDirty },
  } = useForm<SseConnectConfigFormData>({
    resolver: zodResolver(sseConnectConfigSchema),
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

  // 请求头本地状态
  const [headerEntries, setHeaderEntries] = useState<HeaderEntry[]>(() => {
    if (!data.headers) return [];
    return Object.entries(data.headers).map<HeaderEntry>(([key, value]) => ({ key, value }));
  });

  useEffect(() => {
    if (data.headers) {
      setHeaderEntries(Object.entries(data.headers).map<HeaderEntry>(([key, value]) => ({ key, value })));
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
        console.error('保存 SSE Connect 配置失败:', error);
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

  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set());

  const toggleSection = useCallback((section: string) => {
    setExpandedSections(prev => {
      const next = new Set(prev);
      if (next.has(section)) next.delete(section);
      else next.add(section);
      return next;
    });
  }, []);

  const currentAutoReconnect = watch('auto_reconnect') ?? false;

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-rose-200/50 bg-gradient-to-br from-rose-50/80 to-pink-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-rose-900 flex items-center gap-2">
            <Radio className="h-4 w-4" />
            SSE 连接节点
          </h4>
          <p className="text-sm leading-relaxed text-rose-700/90 whitespace-normal break-words">
            建立 Server-Sent Events (SSE) 连接，用于接收服务器推送的事件流。
          </p>
        </div>
      </div>

      {/* SSE URL */}
      <div className="space-y-2">
        <Label htmlFor="url" className="text-sm font-medium text-foreground">
          SSE 端点 URL <span className="text-rose-500">*</span>
        </Label>
        <div className="relative">
          <Radio className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <Input
            id="url"
            {...register('url')}
            placeholder="https://api.example.com/events"
            className="pl-9 font-mono text-sm"
          />
        </div>
        {errors.url && (
          <p className="text-xs text-rose-500">{errors.url.message}</p>
        )}
      </div>

      {/* 请求头 */}
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
                  className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            ))
          )}
        </div>
      </div>

      {/* Last-Event-ID */}
      <div className="space-y-2">
        <Label htmlFor="last_event_id" className="text-sm font-medium text-foreground">
          Last-Event-ID
        </Label>
        <Input
          id="last_event_id"
          {...register('last_event_id')}
          placeholder="可选，用于恢复断开前的事件流"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          指定上次接收事件的 ID，服务器将从该事件之后继续推送
        </p>
      </div>

      {/* 连接超时 */}
      <div className="space-y-2">
        <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground">
          连接超时（毫秒）
        </Label>
        <Input
          id="timeout_ms"
          type="number"
          {...register('timeout_ms', { valueAsNumber: true })}
          placeholder="30000"
          className="font-mono text-sm"
        />
      </div>

      {/* 自动重连 */}
      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div>
          <Label className="text-sm font-medium text-foreground">自动重连</Label>
          <p className="text-xs text-muted-foreground">连接断开时自动尝试重新连接</p>
        </div>
        <Switch
          checked={currentAutoReconnect}
          onCheckedChange={(checked) => {
            setValue('auto_reconnect', checked, { shouldDirty: true });
            saveData();
          }}
        />
      </div>

      {/* 重连选项 */}
      {currentAutoReconnect && (
        <div className="space-y-3 rounded-md border bg-muted/20 p-3">
          <p className="text-xs font-medium text-foreground">重连配置</p>
          <div className="space-y-2">
            <Label htmlFor="reconnect_interval_ms" className="text-xs font-medium text-foreground">重连间隔（毫秒）</Label>
            <Input
              id="reconnect_interval_ms"
              type="number"
              {...register('reconnect_interval_ms', { valueAsNumber: true })}
              placeholder="3000"
              className="h-8 text-xs font-mono"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="max_reconnect_attempts" className="text-xs font-medium text-foreground">最大重连次数</Label>
            <Input
              id="max_reconnect_attempts"
              type="number"
              {...register('max_reconnect_attempts', { valueAsNumber: true })}
              placeholder="5"
              className="h-8 text-xs font-mono"
            />
          </div>
        </div>
      )}

      {/* TLS 选项 */}
      <div className="border rounded-lg overflow-hidden">
        <button
          type="button"
          onClick={() => toggleSection('tls')}
          className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-2">
            <Shield className="h-4 w-4 text-rose-600" />
            <span className="text-sm font-medium text-foreground">TLS 选项</span>
          </div>
          <ChevronDown className={cn(
            "h-3.5 w-3.5 text-muted-foreground transition-transform",
            expandedSections.has('tls') && "rotate-180"
          )} />
        </button>
        {expandedSections.has('tls') && (
          <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
            <div className="space-y-2">
              <Label className="text-xs font-medium text-foreground">TLS 模式</Label>
              <Select
                value={watch('tls_mode') || 'none'}
                onValueChange={(value) => {
                  setValue('tls_mode', value as 'none' | 'standard' | 'skip-verify', { shouldDirty: true });
                  saveData();
                }}
              >
                <SelectTrigger className="h-8 text-xs font-mono">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">不使用 TLS</SelectItem>
                  <SelectItem value="standard">标准 TLS</SelectItem>
                  <SelectItem value="skip-verify">跳过证书验证</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        )}
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-rose-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">SSE 连接说明：</p>
          <p>SSE (Server-Sent Events) 是基于 HTTP 的单向实时推送协议</p>
          <p>服务器通过 Content-Type: text/event-stream 推送事件</p>
          <p>设置 Last-Event-ID 可在重连时恢复事件流</p>
        </div>
      </div>
    </form>
  );
}
