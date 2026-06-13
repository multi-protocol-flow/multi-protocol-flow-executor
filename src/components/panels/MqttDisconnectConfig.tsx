/**
 * MQTT Disconnect 节点配置组件
 * 支持连接选择和 MQTT 5.0 断开选项
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import { mqttDisconnectConfigSchema, type MqttDisconnectConfigFormData } from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Unplug, Info } from 'lucide-react';
import type { MqttDisconnectConfig as MqttDisconnectConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { isMqttConnectConfig } from '@/types/node';
import { getLL } from '@/i18n/i18n-lazy';

interface MqttDisconnectConfigProps {
  data: MqttDisconnectConfigType;
  onSave: (data: MqttDisconnectConfigType) => void;
}

function toFormData(data: MqttDisconnectConfigType): MqttDisconnectConfigFormData {
  return {
    type: 'mqtt_disconnect',
    connection_id: data.connection_id || '',
    reason_code: data.reason_code,
    session_expiry_interval: data.session_expiry_interval,
  };
}

function fromFormData(formData: MqttDisconnectConfigFormData): MqttDisconnectConfigType {
  return {
    type: 'mqtt_disconnect',
    connection_id: formData.connection_id,
    reason_code: formData.reason_code || undefined,
    session_expiry_interval: formData.session_expiry_interval || undefined,
  };
}

export function MqttDisconnectConfig({ data, onSave }: MqttDisconnectConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<MqttDisconnectConfigFormData>({
    resolver: zodResolver(mqttDisconnectConfigSchema),
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
        console.error('保存 MQTT Disconnect 配置失败:', error);
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
            <Unplug className="h-4 w-4" />
            {getLL().panels.mqttDisconnect.title()}
          </h4>
          <p className="text-sm leading-relaxed text-cyan-700/90 whitespace-normal break-words">
            {getLL().panels.mqttDisconnect.description()}
          </p>
        </div>
      </div>

      {/* 连接选择 */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          {getLL().panels.mqttDisconnect.connection_label()} <span className="text-rose-500">*</span>
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

      {/* MQTT 5.0 断开选项 */}
      <div className="space-y-3 rounded-md border bg-muted/20 p-3">
        <p className="text-xs font-medium text-foreground">{getLL().panels.mqttDisconnect.mqtt5_sectionTitle()}</p>
        <div className="space-y-2">
          <Label htmlFor="reason_code" className="text-xs font-medium text-foreground">{getLL().panels.mqttDisconnect.reasonCode_label()}</Label>
          <Input
            id="reason_code"
            type="number"
            {...register('reason_code', { valueAsNumber: true })}
            placeholder={getLL().panels.closeCodeExample()}
            className="h-8 text-xs font-mono"
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="session_expiry_interval" className="text-xs font-medium text-foreground">{getLL().panels.mqttDisconnect.sessionExpiry_label()}</Label>
          <Input
            id="session_expiry_interval"
            type="number"
            {...register('session_expiry_interval', { valueAsNumber: true })}
            placeholder={getLL().panels.optional()}
            className="h-8 text-xs font-mono"
          />
        </div>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.mqttDisconnect.help_title()}</p>
          <p>{getLL().panels.mqttDisconnect.help_line1()}</p>
          <p>{getLL().panels.mqttDisconnect.help_line2()}</p>
          <p>{getLL().panels.mqttDisconnect.help_line3()}</p>
        </div>
      </div>
    </form>
  );
}
