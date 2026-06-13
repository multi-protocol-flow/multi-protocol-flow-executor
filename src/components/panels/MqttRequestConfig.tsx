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
import { getLL } from '@/i18n/i18n-lazy';

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
            {getLL().panels.mqttRequest.title()}
          </h4>
          <p className="text-sm leading-relaxed text-cyan-700/90 whitespace-normal break-words">
            {getLL().panels.mqttRequest.description()}
          </p>
        </div>
      </div>

      {/* 连接选择 */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          {getLL().panels.mqttRequest.connection_label()} <span className="text-rose-500">*</span>
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
            {getLL().panels.mqttPublish.noConnectNode()}
          </p>
        )}
      </div>

      {/* 请求主题 */}
      <div className="space-y-2">
        <Label htmlFor="request_topic" className="text-sm font-medium text-foreground">
          {getLL().panels.mqttRequest.requestTopic()} <span className="text-rose-500">*</span>
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
          {getLL().panels.mqttRequest.responseTopic()} <span className="text-rose-500">*</span>
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
          {getLL().panels.mqttRequest.requestPayload()}
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
        <Label className="text-sm font-medium text-foreground">{getLL().panels.mqttRequest.qos_label()}</Label>
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
            <SelectItem value="0">{getLL().panels.mqttRequest.qos0_option()}</SelectItem>
            <SelectItem value="1">{getLL().panels.mqttRequest.qos1_option()}</SelectItem>
            <SelectItem value="2">{getLL().panels.mqttRequest.qos2_option()}</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* 关联数据 */}
      <div className="space-y-2">
        <Label htmlFor="correlation_data" className="text-sm font-medium text-foreground">
          {getLL().panels.mqttRequest.correlationData()}
        </Label>
        <Input
          id="correlation_data"
          {...register('correlation_data')}
          placeholder={getLL().panels.mqttRequest.correlationPlaceholder()}
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          {getLL().panels.mqttRequest.correlationHelp()}
        </p>
      </div>

      {/* 超时 */}
      <div className="space-y-2">
        <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground">
          {getLL().panels.mqttRequest.responseTimeout()}
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
          <p className="font-medium text-foreground">{getLL().panels.mqttRequest.help_title()}</p>
          <p>{getLL().panels.mqttRequest.help_line1()}</p>
          <p>{getLL().panels.mqttRequest.help_line2()}</p>
          <p>{getLL().panels.mqttRequest.help_line3()}</p>
        </div>
      </div>
    </form>
  );
}
