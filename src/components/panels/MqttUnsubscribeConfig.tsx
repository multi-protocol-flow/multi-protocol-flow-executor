/**
 * MQTT Unsubscribe 节点配置组件
 * 支持连接选择和取消订阅主题
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import { mqttUnsubscribeConfigSchema, type MqttUnsubscribeConfigFormData } from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Inbox, Info } from 'lucide-react';
import type { MqttUnsubscribeConfig as MqttUnsubscribeConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { isMqttConnectConfig } from '@/types/node';

interface MqttUnsubscribeConfigProps {
  data: MqttUnsubscribeConfigType;
  onSave: (data: MqttUnsubscribeConfigType) => void;
}

function toFormData(data: MqttUnsubscribeConfigType): MqttUnsubscribeConfigFormData {
  return {
    type: 'mqtt_unsubscribe',
    connection_id: data.connection_id || '',
    topic: data.topic || '',
  };
}

function fromFormData(formData: MqttUnsubscribeConfigFormData): MqttUnsubscribeConfigType {
  return {
    type: 'mqtt_unsubscribe',
    connection_id: formData.connection_id,
    topic: formData.topic,
  };
}

export function MqttUnsubscribeConfig({ data, onSave }: MqttUnsubscribeConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<MqttUnsubscribeConfigFormData>({
    resolver: zodResolver(mqttUnsubscribeConfigSchema),
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
        console.error('保存 MQTT Unsubscribe 配置失败:', error);
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
            <Inbox className="h-4 w-4" />
            MQTT 取消订阅节点
          </h4>
          <p className="text-sm leading-relaxed text-cyan-700/90 whitespace-normal break-words">
            取消订阅指定的 MQTT 主题。
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

      {/* 主题 */}
      <div className="space-y-2">
        <Label htmlFor="topic" className="text-sm font-medium text-foreground">
          主题 <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="topic"
          {...register('topic')}
          placeholder="sensor/temperature"
          className="font-mono text-sm"
        />
        {errors.topic && (
          <p className="text-xs text-rose-500">{errors.topic.message}</p>
        )}
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">取消订阅说明：</p>
          <p>取消订阅后，该连接将不再接收对应主题的消息</p>
        </div>
      </div>
    </form>
  );
}
