/**
 * MQTT Subscribe 节点配置组件
 * 支持连接选择、主题、QoS、采集配置、MQTT 5.0 选项、共享订阅
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef, useMemo } from 'react';
import { mqttSubscribeConfigSchema, type MqttSubscribeConfigFormData } from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Inbox, Info, ChevronDown } from 'lucide-react';
import type { MqttSubscribeConfig as MqttSubscribeConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { isMqttConnectConfig } from '@/types/node';
import { cn } from '@/lib/utils';

interface MqttSubscribeConfigProps {
  data: MqttSubscribeConfigType;
  onSave: (data: MqttSubscribeConfigType) => void;
}

function toFormData(data: MqttSubscribeConfigType): MqttSubscribeConfigFormData {
  return {
    type: 'mqtt_subscribe',
    connection_id: data.connection_id || '',
    topic: data.topic || '',
    qos: data.qos ?? 0,
    max_messages: data.max_messages,
    collect_timeout_ms: data.collect_timeout_ms,
    output_mode: data.output_mode || 'first',
    no_local: data.no_local ?? false,
    retain_as_published: data.retain_as_published ?? false,
    retain_handling: data.retain_handling ?? 0,
    subscription_identifier: data.subscription_identifier,
    share_group: data.share_group || '',
  };
}

function fromFormData(formData: MqttSubscribeConfigFormData): MqttSubscribeConfigType {
  return {
    type: 'mqtt_subscribe',
    connection_id: formData.connection_id,
    topic: formData.topic,
    qos: formData.qos,
    max_messages: formData.max_messages || undefined,
    collect_timeout_ms: formData.collect_timeout_ms || undefined,
    output_mode: formData.output_mode || undefined,
    no_local: formData.no_local || undefined,
    retain_as_published: formData.retain_as_published || undefined,
    retain_handling: formData.retain_handling,
    subscription_identifier: formData.subscription_identifier || undefined,
    share_group: formData.share_group || undefined,
  };
}

export function MqttSubscribeConfig({ data, onSave }: MqttSubscribeConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<MqttSubscribeConfigFormData>({
    resolver: zodResolver(mqttSubscribeConfigSchema),
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
        console.error('保存 MQTT Subscribe 配置失败:', error);
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

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-cyan-200/50 bg-gradient-to-br from-cyan-50/80 to-sky-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-cyan-900 flex items-center gap-2">
            <Inbox className="h-4 w-4" />
            MQTT 订阅节点
          </h4>
          <p className="text-sm leading-relaxed text-cyan-700/90 whitespace-normal break-words">
            订阅 MQTT 主题并接收消息，支持通配符和共享订阅。
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
          placeholder="sensor/#"
          className="font-mono text-sm"
        />
        {errors.topic && (
          <p className="text-xs text-rose-500">{errors.topic.message}</p>
        )}
        <p className="text-xs text-muted-foreground">
          支持通配符：+（单层）和 #（多层）
        </p>
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

      {/* 输出模式 */}
      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">输出模式</Label>
        <Select
          value={watch('output_mode') || 'first'}
          onValueChange={(value) => {
            setValue('output_mode', value as 'first' | 'all', { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger className="font-mono text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="first">首条消息</SelectItem>
            <SelectItem value="all">全部消息</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* 采集配置 */}
      <div className="space-y-3 rounded-md border bg-muted/20 p-3">
        <p className="text-xs font-medium text-foreground">采集配置</p>
        <div className="space-y-2">
          <Label htmlFor="max_messages" className="text-xs font-medium text-foreground">最大消息数</Label>
          <Input
            id="max_messages"
            type="number"
            {...register('max_messages', { valueAsNumber: true })}
            placeholder="可选"
            className="h-8 text-xs font-mono"
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="collect_timeout_ms" className="text-xs font-medium text-foreground">采集超时（毫秒）</Label>
          <Input
            id="collect_timeout_ms"
            type="number"
            {...register('collect_timeout_ms', { valueAsNumber: true })}
            placeholder="可选"
            className="h-8 text-xs font-mono"
          />
        </div>
      </div>

      {/* 共享订阅 */}
      <div className="space-y-2">
        <Label htmlFor="share_group" className="text-sm font-medium text-foreground">
          共享订阅组名
        </Label>
        <Input
          id="share_group"
          {...register('share_group')}
          placeholder="可选，如 group1"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          设置后将使用共享订阅：<code className="rounded bg-muted px-1">$share/{'{'}group{'}'}/{'{'}topic{'}'}</code>
        </p>
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
            <span className="text-sm font-medium text-foreground">MQTT 5.0 选项</span>
          </div>
          <ChevronDown className={cn(
            "h-3.5 w-3.5 text-muted-foreground transition-transform",
            expandedSections.has('mqtt5') && "rotate-180"
          )} />
        </button>
        {expandedSections.has('mqtt5') && (
          <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
            <div className="flex items-center justify-between">
              <div>
                <Label className="text-xs font-medium text-foreground">不接收自身消息</Label>
                <p className="text-[10px] text-muted-foreground">no_local: 不转发自己发布的消息</p>
              </div>
              <Switch
                checked={watch('no_local') ?? false}
                onCheckedChange={(checked) => {
                  setValue('no_local', checked, { shouldDirty: true });
                  saveData();
                }}
              />
            </div>
            <div className="flex items-center justify-between">
              <div>
                <Label className="text-xs font-medium text-foreground">保留消息按原样发布</Label>
                <p className="text-[10px] text-muted-foreground">retain_as_published</p>
              </div>
              <Switch
                checked={watch('retain_as_published') ?? false}
                onCheckedChange={(checked) => {
                  setValue('retain_as_published', checked, { shouldDirty: true });
                  saveData();
                }}
              />
            </div>
            <div className="space-y-2">
              <Label className="text-xs font-medium text-foreground">保留消息处理策略</Label>
              <Select
                value={String(watch('retain_handling') ?? 0)}
                onValueChange={(value) => {
                  setValue('retain_handling', Number(value) as 0 | 1 | 2, { shouldDirty: true });
                  saveData();
                }}
              >
                <SelectTrigger className="h-8 text-xs font-mono">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="0">0 — 发送保留消息</SelectItem>
                  <SelectItem value="1">1 — 仅新订阅时发送</SelectItem>
                  <SelectItem value="2">2 — 不发送保留消息</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="subscription_identifier" className="text-xs font-medium text-foreground">订阅标识符</Label>
              <Input
                id="subscription_identifier"
                type="number"
                {...register('subscription_identifier', { valueAsNumber: true })}
                placeholder="可选"
                className="h-8 text-xs font-mono"
              />
            </div>
          </div>
        )}
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">订阅说明：</p>
          <p>主题支持 MQTT 通配符：+ 匹配单层，# 匹配多层（必须在末尾）</p>
          <p>输出模式「首条消息」等待第一条即完成，「全部消息」需配合采集超时使用</p>
          <p>共享订阅需要 Broker 支持此特性</p>
        </div>
      </div>
    </form>
  );
}
