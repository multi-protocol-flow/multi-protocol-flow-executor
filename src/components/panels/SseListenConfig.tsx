/**
 * SSE Listen 节点配置组件
 * 支持连接 ID、事件过滤、最大事件数、超时、数据包含选项、JSON 解码
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import {
  sseListenConfigSchema,
  type SseListenConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Ear, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { SseListenConfig as SseListenConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { isSseConnectConfig } from '@/types/node';

interface SseListenConfigProps {
  data: SseListenConfigType;
  onSave: (data: SseListenConfigType) => void;
}

function toFormData(data: SseListenConfigType): SseListenConfigFormData {
  return {
    type: 'sse_listen',
    connection_id: data.connection_id || '',
    event_filter: data.event_filter || '',
    max_events: data.max_events ?? 0,
    timeout_ms: data.timeout_ms ?? 30000,
    include_data: data.include_data ?? true,
    decode_json: data.decode_json ?? false,
  };
}

function fromFormData(formData: SseListenConfigFormData): SseListenConfigType {
  return {
    type: 'sse_listen',
    connection_id: formData.connection_id,
    event_filter: formData.event_filter || undefined,
    max_events: formData.max_events || undefined,
    timeout_ms: formData.timeout_ms || undefined,
    include_data: formData.include_data,
    decode_json: formData.decode_json || undefined,
  };
}

export function SseListenConfig({ data, onSave }: SseListenConfigProps) {
  const {
    register,
    handleSubmit,
    reset,
    getValues,
    setValue,
    watch,
    formState: { errors, isDirty },
  } = useForm<SseListenConfigFormData>({
    resolver: zodResolver(sseListenConfigSchema),
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

  const flow = useFlowStore((state) => state.flow);

  const availableConnections = useMemo(() => {
    if (!flow) return [];
    const connections: Array<{ uuid: string; name: string }> = [];
    flow.nodes.forEach(node => {
      if (isSseConnectConfig(node.data)) {
        connections.push({ uuid: node.uuid, name: node.name });
      }
    });
    return connections;
  }, [flow]);

  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 SSE Listen 配置失败:', error);
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
      <div className="rounded-lg border border-rose-200/50 bg-gradient-to-br from-rose-50/80 to-pink-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-rose-900 flex items-center gap-2">
            <Ear className="h-4 w-4" />
            {getLL().panels.sseListen.title()}
          </h4>
          <p className="text-sm leading-relaxed text-rose-700/90 whitespace-normal break-words">
            {getLL().panels.sseListen.description()}
          </p>
        </div>
      </div>

      {/* 连接选择 */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          {getLL().panels.connection()} <span className="text-rose-500">*</span>
        </Label>
        {availableConnections.length > 0 ? (
          <Select
            value={watch('connection_id') || ''}
            onValueChange={(value) => {
              setValue('connection_id', value, { shouldDirty: true });
              saveData();
            }}
          >
            <SelectTrigger id="connection_id" className="font-mono">
              <SelectValue placeholder={getLL().panels.selectConnection()} />
            </SelectTrigger>
            <SelectContent>
              {availableConnections.map((conn) => (
                <SelectItem key={conn.uuid} value={conn.uuid}>
                  <span className="font-mono">{conn.name}</span>
                  <span className="ml-2 text-xs text-muted-foreground">
                    ({conn.uuid.substring(0, 8)}...)
                  </span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        ) : (
          <Input
            id="connection_id"
            {...register('connection_id')}
            placeholder={getLL().panels.enterConnectionId()}
            className="font-mono text-sm"
          />
        )}
        {errors.connection_id && (
          <p className="text-xs text-rose-500">{errors.connection_id.message}</p>
        )}
        {availableConnections.length === 0 && (
          <p className="text-xs text-muted-foreground">
            {getLL().panels.sseListen.noConnectNode()}
          </p>
        )}
      </div>

      {/* 事件过滤 */}
      <div className="space-y-2">
        <Label htmlFor="event_filter" className="text-sm font-medium text-foreground">
          {getLL().panels.sseListen.eventFilter()}
        </Label>
        <Input
          id="event_filter"
          {...register('event_filter')}
          placeholder={getLL().panels.sseListen.eventFilterPlaceholder()}
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          {getLL().panels.sseListen.eventFilterHelp()}
        </p>
      </div>

      {/* 最大事件数 */}
      <div className="space-y-2">
        <Label htmlFor="max_events" className="text-sm font-medium text-foreground">
          {getLL().panels.sseListen.maxEvents()}
        </Label>
        <Input
          id="max_events"
          type="number"
          {...register('max_events', { valueAsNumber: true })}
          placeholder="0 = Unlimited"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          {getLL().panels.sseListen.maxEventsHint()}
        </p>
      </div>

      {/* 监听超时 */}
      <div className="space-y-2">
        <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground">
          {getLL().panels.sseListen.timeout()}
        </Label>
        <Input
          id="timeout_ms"
          type="number"
          {...register('timeout_ms', { valueAsNumber: true })}
          placeholder="30000"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          {getLL().panels.sseListen.timeoutHint()}
        </p>
      </div>

      {/* 包含数据 */}
      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div>
          <Label className="text-sm font-medium text-foreground">包含事件数据</Label>
          <p className="text-xs text-muted-foreground">记录事件的 data 字段内容</p>
        </div>
        <Switch
          checked={watch('include_data') ?? true}
          onCheckedChange={(checked) => {
            setValue('include_data', checked, { shouldDirty: true });
            saveData();
          }}
        />
      </div>

      {/* JSON 解码 */}
      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div>
          <Label className="text-sm font-medium text-foreground">自动解码 JSON</Label>
          <p className="text-xs text-muted-foreground">自动将 data 字段解析为 JSON 对象</p>
        </div>
        <Switch
          checked={watch('decode_json') ?? false}
          onCheckedChange={(checked) => {
            setValue('decode_json', checked, { shouldDirty: true });
            saveData();
          }}
        />
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-rose-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">SSE 监听说明：</p>
          <p>监听指定 SSE 连接上的实时事件</p>
          <p>事件过滤器对应 SSE 协议中的 event 字段</p>
          <p>设置最大事件数或超时时间可自动结束监听</p>
        </div>
      </div>
    </form>
  );
}
