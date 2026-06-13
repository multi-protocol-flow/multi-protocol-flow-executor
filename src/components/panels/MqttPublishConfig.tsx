/**
 * MQTT Publish 节点配置组件
 * 支持连接选择、主题、载荷、QoS、MQTT 5.0 选项、用户属性
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef, useMemo } from 'react';
import { mqttPublishConfigSchema, type MqttPublishConfigFormData } from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Send, Info, Plus, Trash2, ChevronDown } from 'lucide-react';
import type { MqttPublishConfig as MqttPublishConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { isMqttConnectConfig } from '@/types/node';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';

interface MqttPublishConfigProps {
  data: MqttPublishConfigType;
  onSave: (data: MqttPublishConfigType) => void;
}

function toFormData(data: MqttPublishConfigType): MqttPublishConfigFormData {
  return {
    type: 'mqtt_publish',
    connection_id: data.connection_id || '',
    topic: data.topic || '',
    payload: data.payload || '',
    qos: data.qos ?? 0,
    retain: data.retain ?? false,
    payload_format: data.payload_format || 'text',
    content_type: data.content_type || '',
    message_expiry_interval: data.message_expiry_interval,
    response_topic: data.response_topic || '',
    correlation_data: data.correlation_data || '',
    topic_alias: data.topic_alias,
    user_properties: data.user_properties || [],
  };
}

function fromFormData(formData: MqttPublishConfigFormData): MqttPublishConfigType {
  const userProps = formData.user_properties?.filter(pair => pair[0].trim());

  return {
    type: 'mqtt_publish',
    connection_id: formData.connection_id,
    topic: formData.topic,
    payload: formData.payload || undefined,
    qos: formData.qos,
    retain: formData.retain || undefined,
    payload_format: formData.payload_format || undefined,
    content_type: formData.content_type || undefined,
    message_expiry_interval: formData.message_expiry_interval || undefined,
    response_topic: formData.response_topic || undefined,
    correlation_data: formData.correlation_data || undefined,
    topic_alias: formData.topic_alias || undefined,
    user_properties: userProps && userProps.length > 0
      ? userProps
      : undefined,
  };
}

export function MqttPublishConfig({ data, onSave }: MqttPublishConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<MqttPublishConfigFormData>({
    resolver: zodResolver(mqttPublishConfigSchema),
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
        console.error('保存 MQTT Publish 配置失败:', error);
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

  // 用户属性本地状态
  const [userProps, setUserProps] = useState<Array<[string, string]>>(
    data.user_properties || []
  );

  useEffect(() => {
    setUserProps(data.user_properties || []);
  }, [data.user_properties]);

  const syncUserProps = useCallback((newProps: Array<[string, string]>) => {
    setUserProps(newProps);
    setValue('user_properties', newProps, { shouldDirty: true });
  }, [setValue]);

  const handleAddUserProp = useCallback(() => {
    const newProps = [...userProps, ['', ''] as [string, string]];
    syncUserProps(newProps);
    safeSetTimeout(saveData, 100);
  }, [userProps, syncUserProps, safeSetTimeout, saveData]);

  const handleRemoveUserProp = useCallback((index: number) => {
    const newProps = userProps.filter((_, i) => i !== index);
    syncUserProps(newProps);
    safeSetTimeout(saveData, 100);
  }, [userProps, syncUserProps, safeSetTimeout, saveData]);

  const handleUserPropChange = useCallback((index: number, position: 0 | 1, value: string) => {
    const newProps = userProps.map((pair, i) => {
      if (i !== index) return pair;
      const updated: [string, string] = [...pair];
      updated[position] = value;
      return updated;
    });
    syncUserProps(newProps);
  }, [userProps, syncUserProps]);

  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set());

  const toggleSection = useCallback((section: string) => {
    setExpandedSections(prev => {
      const next = new Set(prev);
      if (next.has(section)) next.delete(section);
      else next.add(section);
      return next;
    });
  }, []);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-cyan-200/50 bg-gradient-to-br from-cyan-50/80 to-sky-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-cyan-900 flex items-center gap-2">
            <Send className="h-4 w-4" />
            {getLL().panels.mqttPublish.title()}
          </h4>
          <p className="text-sm leading-relaxed text-cyan-700/90 whitespace-normal break-words">
            {getLL().panels.mqttPublish.description()}
          </p>
        </div>
      </div>

      {/* 连接选择 */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          {getLL().panels.mqttPublish.connection_label()} <span className="text-rose-500">*</span>
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

      {/* 主题 */}
      <div className="space-y-2">
        <Label htmlFor="topic" className="text-sm font-medium text-foreground">
          {getLL().panels.mqttPublish.topicLabel()} <span className="text-rose-500">*</span>
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

      {/* 载荷 */}
      <div className="space-y-2">
        <Label htmlFor="payload" className="text-sm font-medium text-foreground">
          {getLL().panels.mqttPublish.payload()}
        </Label>
        <Textarea
          id="payload"
          {...register('payload')}
          placeholder='{"temperature": 25.5}'
          className="min-h-[80px] resize-y font-mono text-xs"
          rows={4}
        />
      </div>

      {/* 载荷格式 */}
      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">{getLL().panels.mqttPublish.payloadFormat_label()}</Label>
        <Select
          value={watch('payload_format') || 'text'}
          onValueChange={(value) => {
            setValue('payload_format', value as 'text' | 'json' | 'hex' | 'base64', { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger className="font-mono text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="text">{getLL().panels.mqttPublish.text_option()}</SelectItem>
            <SelectItem value="json">JSON</SelectItem>
            <SelectItem value="hex">{getLL().panels.mqttPublish.hex_option()}</SelectItem>
            <SelectItem value="base64">Base64</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* QoS */}
      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">{getLL().panels.mqttPublish.qos_label()}</Label>
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
            <SelectItem value="0">{getLL().panels.mqttPublish.qos0_option()}</SelectItem>
            <SelectItem value="1">{getLL().panels.mqttPublish.qos1_option()}</SelectItem>
            <SelectItem value="2">{getLL().panels.mqttPublish.qos2_option()}</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* 保留标志 */}
      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div>
          <Label className="text-sm font-medium text-foreground">{getLL().panels.mqttPublish.retain_label()}</Label>
          <p className="text-xs text-muted-foreground">{getLL().panels.mqttPublish.retain_desc()}</p>
        </div>
        <Switch
          checked={watch('retain') ?? false}
          onCheckedChange={(checked) => {
            setValue('retain', checked, { shouldDirty: true });
            saveData();
          }}
        />
      </div>

      {/* MQTT 5.0 高级选项 */}
      <div className="border rounded-lg overflow-hidden">
        <button
          type="button"
          onClick={() => toggleSection('mqtt5')}
          className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-2">
            <Info className="h-4 w-4 text-cyan-600" />
            <span className="text-sm font-medium text-foreground">{getLL().panels.mqttPublish.mqtt5_sectionTitle()}</span>
          </div>
          <ChevronDown className={cn(
            "h-3.5 w-3.5 text-muted-foreground transition-transform",
            expandedSections.has('mqtt5') && "rotate-180"
          )} />
        </button>
        {expandedSections.has('mqtt5') && (
          <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
            <div className="space-y-2">
              <Label htmlFor="content_type" className="text-xs font-medium text-foreground">{getLL().panels.mqttPublish.contentType_label()}</Label>
              <Input
                id="content_type"
                {...register('content_type')}
                placeholder="application/json"
                className="h-8 text-xs font-mono"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="message_expiry_interval" className="text-xs font-medium text-foreground">{getLL().panels.mqttPublish.messageExpiry_label()}</Label>
              <Input
                id="message_expiry_interval"
                type="number"
                {...register('message_expiry_interval', { valueAsNumber: true })}
                placeholder={getLL().panels.optional()}
                className="h-8 text-xs font-mono"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="response_topic" className="text-xs font-medium text-foreground">{getLL().panels.mqttPublish.responseTopic_label()}</Label>
              <Input
                id="response_topic"
                {...register('response_topic')}
                placeholder="response/topic"
                className="h-8 text-xs font-mono"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="correlation_data" className="text-xs font-medium text-foreground">{getLL().panels.mqttPublish.correlationData_label()}</Label>
              <Input
                id="correlation_data"
                {...register('correlation_data')}
                placeholder={getLL().panels.optional()}
                className="h-8 text-xs font-mono"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="topic_alias" className="text-xs font-medium text-foreground">{getLL().panels.mqttPublish.topicAlias_label()}</Label>
              <Input
                id="topic_alias"
                type="number"
                {...register('topic_alias', { valueAsNumber: true })}
                placeholder={getLL().panels.optional()}
                className="h-8 text-xs font-mono"
              />
            </div>
          </div>
        )}
      </div>

      {/* 用户属性 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">{getLL().panels.mqttPublish.userProperties_label()}</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddUserProp}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            {getLL().panels.add()}
          </Button>
        </div>
        <div className="space-y-2">
          {userProps.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">{getLL().panels.mqttPublish.noUserProperties()}</p>
            </div>
          ) : (
            userProps.map((pair, index) => (
              <div key={index} className="flex items-center gap-2 p-2 rounded-md border bg-muted/30">
                <Input
                  value={pair[0]}
                  onChange={(e) => handleUserPropChange(index, 0, e.target.value)}
                  onBlur={() => safeSetTimeout(saveData, 100)}
                  placeholder="Key"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Input
                  value={pair[1]}
                  onChange={(e) => handleUserPropChange(index, 1, e.target.value)}
                  onBlur={() => safeSetTimeout(saveData, 100)}
                  placeholder="Value"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  onClick={() => handleRemoveUserProp(index)}
                  className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            ))
          )}
        </div>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.mqttPublish.help_title()}</p>
          <p>{getLL().panels.mqttPublish.help_line1()}</p>
          <p>{getLL().panels.mqttPublish.help_line2()}</p>
        </div>
      </div>
    </form>
  );
}
