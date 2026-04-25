/**
 * MQTT Request 节点配置组件（请求-响应模式）
 * 支持连接选择、请求/响应主题、载荷、QoS、关联数据、超时
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import { mqttRequestConfigSchema, type MqttRequestConfigFormData } from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { ArrowRightLeft, Info } from 'lucide-react';
import type { MqttRequestConfig as MqttRequestConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { isMqttConnectConfig } from '@/types/node';

interface MqttRequestConfigProps {
  data: MqttRequestConfigType;
  onSave: (data: MqttRequestConfigType) => void;
}

function toFormData(data: MqttRequestConfigType): MqttRequestConfigFormData {
  return {
    type: 'mqtt_request',
    connection_id: data.connection_id || '',
    request_topic: data.request_topic || '',
    response_topic: data.response_topic || '',
    payload: data.payload || '',
    qos: data.qos ?? 0,
    correlation_data: data.correlation_data || '',
    timeout_ms: data.timeout_ms ?? 5000,
  };
}

function fromFormData(formData: MqttRequestConfigFormData): MqttRequestConfigType {
  return {
    type: 'mqtt_request',
    connection_id: formData.connection_id,
    request_topic: formData.request_topic,
    response_topic: formData.response_topic,
    payload: formData.payload || undefined,
    qos: formData.qos,
    correlation_data: formData.correlation_data || undefined,
    timeout_ms: formData.timeout_ms || undefined,
  };
}

export function MqttRequestConfig({ data, onSave }: MqttRequestConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<MqttRequestConfigFormData>({
    resolver: zodResolver(mqttRequestConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  const flow = useFlowStore((state) => state.flow);

  const availableConnections = useMemo(() => {
    if (!flow) return [];
    const connections: Array<{ uuid: string; name: string }> = [];
    flow.nodes.forEach(node => {
      if (isMqttConnectConfig(node.data)) {
        connections.push({ uuid: node.uuid, name: node.name });
      }
    });
    return connections;
  }, [flow]);

  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

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

  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 MQTT Request 配置失败:', error);
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
      <div className="rounded-lg border border-cyan-200/50 bg-gradient-to-br from-cyan-50/80 to-sky-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-cyan-900 flex items-center gap-2">
            <ArrowRightLeft className="h-4 w-4" />
            MQTT 请求-响应节点
          </h4>
          <p className="text-sm leading-relaxed text-cyan-700/90 whitespace-normal break-words">
            发送请求到指定主题并等待响应，实现请求-响应模式。
          </p>
        </div>
      </div>

      {/* 连接选择 */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          连接 <span className="text-rose-500">*</span>
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
              <SelectValue placeholder="选择 MQTT 连接" />
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
            placeholder="输入连接 ID"
            className="font-mono text-sm"
          />
        )}
        {errors.connection_id && (
          <p className="text-xs text-rose-500">{errors.connection_id.message}</p>
        )}
        {availableConnections.length === 0 && (
          <p className="text-xs text-muted-foreground">
            未检测到 MQTT Connect 节点，请手动输入连接 ID
          </p>
        )}
      </div>

      {/* 请求主题 */}
      <div className="space-y-2">
        <Label htmlFor="request_topic" className="text-sm font-medium text-foreground">
          请求主题 <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="request_topic"
          {...register('request_topic')}
          placeholder="request/topic"
          className="font-mono text-sm"
        />
        {errors.request_topic && (
          <p className="text-xs text-rose-500">{errors.request_topic.message}</p>
        )}
      </div>

      {/* 响应主题 */}
      <div className="space-y-2">
        <Label htmlFor="response_topic" className="text-sm font-medium text-foreground">
          响应主题 <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="response_topic"
          {...register('response_topic')}
          placeholder="response/topic"
          className="font-mono text-sm"
        />
        {errors.response_topic && (
          <p className="text-xs text-rose-500">{errors.response_topic.message}</p>
        )}
      </div>

      {/* 载荷 */}
      <div className="space-y-2">
        <Label htmlFor="payload" className="text-sm font-medium text-foreground">
          请求载荷
        </Label>
        <Textarea
          id="payload"
          {...register('payload')}
          placeholder='{"action": "get_status"}'
          className="min-h-[80px] resize-y font-mono text-xs"
          rows={4}
        />
      </div>

      {/* QoS */}
      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">QoS 等级</Label>
        <Select
          value={String(watch('qos') ?? 0)}
          onValueChange={(value) => {
            setValue('qos', Number(value), { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger className="font-mono text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="0">QoS 0 — 最多一次</SelectItem>
            <SelectItem value="1">QoS 1 — 至少一次</SelectItem>
            <SelectItem value="2">QoS 2 — 恰好一次</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* 关联数据 */}
      <div className="space-y-2">
        <Label htmlFor="correlation_data" className="text-sm font-medium text-foreground">
          关联数据
        </Label>
        <Input
          id="correlation_data"
          {...register('correlation_data')}
          placeholder="留空自动生成 UUID"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          用于匹配请求和响应，留空时自动生成唯一 UUID
        </p>
      </div>

      {/* 超时 */}
      <div className="space-y-2">
        <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground">
          响应超时（毫秒）
        </Label>
        <Input
          id="timeout_ms"
          type="number"
          {...register('timeout_ms', { valueAsNumber: true })}
          placeholder="5000"
          className="font-mono text-sm"
        />
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">请求-响应说明：</p>
          <p>向请求主题发送消息，同时订阅响应主题等待回复</p>
          <p>关联数据用于匹配请求和响应，确保不混淆</p>
          <p>超时后未收到响应将返回超时错误</p>
        </div>
      </div>
    </form>
  );
}
