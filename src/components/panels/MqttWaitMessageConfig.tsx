/**
 * MQTT Wait Message 节点配置组件
 * 支持连接选择、主题、匹配条件、超时和最大消息数
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { mqttWaitMessageConfigSchema, type MqttWaitMessageConfigFormData } from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Clock, Info, ChevronDown } from 'lucide-react';
import type { MqttWaitMessageConfig as MqttWaitMessageConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { isMqttConnectConfig } from '@/types/node';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';

interface MqttWaitMessageConfigProps {
  data: MqttWaitMessageConfigType;
  onSave: (data: MqttWaitMessageConfigType) => void;
}

function toFormData(data: MqttWaitMessageConfigType): MqttWaitMessageConfigFormData {
  return {
    type: 'mqtt_wait_message',
    connection_id: data.connection_id || '',
    topic: data.topic || '',
    match_condition: data.match_condition ? JSON.stringify(data.match_condition) : undefined,
    timeout_ms: data.timeout_ms ?? 5000,
    max_messages: data.max_messages ?? 1,
  };
}

function fromFormData(formData: MqttWaitMessageConfigFormData): MqttWaitMessageConfigType {
  let matchCondition: Record<string, unknown> | undefined;
  if (formData.match_condition) {
    if (typeof formData.match_condition === 'string') {
      try {
        matchCondition = JSON.parse(formData.match_condition);
      } catch {
        matchCondition = undefined;
      }
    } else {
      matchCondition = formData.match_condition as Record<string, unknown>;
    }
  }

  return {
    type: 'mqtt_wait_message',
    connection_id: formData.connection_id,
    topic: formData.topic,
    match_condition: matchCondition,
    timeout_ms: formData.timeout_ms || undefined,
    max_messages: formData.max_messages || undefined,
  };
}

export function MqttWaitMessageConfig({ data, onSave }: MqttWaitMessageConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<MqttWaitMessageConfigFormData>({
    resolver: zodResolver(mqttWaitMessageConfigSchema),
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
        console.error('保存 MQTT WaitMessage 配置失败:', error);
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

  // match_condition 为字符串时的显示值
  const matchConditionStr = (() => {
    const mc = watch('match_condition');
    if (!mc) return '';
    if (typeof mc === 'string') return mc;
    try {
      return JSON.stringify(mc, null, 2);
    } catch {
      return '';
    }
  })();

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-cyan-200/50 bg-gradient-to-br from-cyan-50/80 to-sky-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-cyan-900 flex items-center gap-2">
            <Clock className="h-4 w-4" />
            {getLL().panels.mqttWaitMessage.title()}
          </h4>
          <p className="text-sm leading-relaxed text-cyan-700/90 whitespace-normal break-words">
            {getLL().panels.mqttWaitMessage.description()}
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
          {getLL().panels.mqttWaitMessage.topicLabel()} <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="topic"
          {...register('topic')}
          placeholder="response/topic"
          className="font-mono text-sm"
        />
        {errors.topic && (
          <p className="text-xs text-rose-500">{errors.topic.message}</p>
        )}
      </div>

      {/* 匹配条件 */}
      <div className="border rounded-lg overflow-hidden">
        <button
          type="button"
          onClick={() => toggleSection('match')}
          className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-2">
            <Info className="h-4 w-4 text-cyan-600" />
            <span className="text-sm font-medium text-foreground">匹配条件（可选）</span>
          </div>
          <ChevronDown className={cn(
            "h-3.5 w-3.5 text-muted-foreground transition-transform",
            expandedSections.has('match') && "rotate-180"
          )} />
        </button>
        {expandedSections.has('match') && (
          <div className="border-t bg-muted/20 px-3 py-3 space-y-2">
            <p className="text-xs text-muted-foreground">
              {getLL().panels.mqttWaitMessage.matchConditionHelp()}
            </p>
            <Textarea
              defaultValue={matchConditionStr}
              onChange={(e) => {
                const val = e.target.value.trim();
                if (val) {
                  try {
                    setValue('match_condition', JSON.parse(val) as Record<string, unknown>, { shouldDirty: true });
                  } catch {
                    // JSON 解析失败时保留原始字符串
                  }
                } else {
                  setValue('match_condition' as 'topic', undefined as unknown as string, { shouldDirty: true });
                }
              }}
              placeholder='{"match_type": "json_path", "expression": "$.status"}'
              className="min-h-[80px] resize-y font-mono text-xs"
              rows={4}
            />
          </div>
        )}
      </div>

      {/* 超时 */}
      <div className="space-y-2">
        <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground">
          {getLL().panels.mqttWaitMessage.timeout()}
        </Label>
        <Input
          id="timeout_ms"
          type="number"
          {...register('timeout_ms', { valueAsNumber: true })}
          placeholder="5000"
          className="font-mono text-sm"
        />
      </div>

      {/* 最大消息数 */}
      <div className="space-y-2">
        <Label htmlFor="max_messages" className="text-sm font-medium text-foreground">
          {getLL().panels.mqttWaitMessage.maxMessages()}
        </Label>
        <Input
          id="max_messages"
          type="number"
          {...register('max_messages', { valueAsNumber: true })}
          placeholder="1"
          className="font-mono text-sm"
        />
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">等待消息说明：</p>
          <p>等待指定主题上的 MQTT 消息到达</p>
          <p>匹配条件支持 json_path、regex、expression 等模式</p>
          <p>超时后未收到消息将返回超时错误</p>
        </div>
      </div>
    </form>
  );
}
