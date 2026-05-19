/**
 * Kafka ConsumeBatch 节点配置组件
 * 支持连接选择、主题、消费者组、偏移量、反序列化、批量策略配置
 */
import { useForm } from 'react-hook-form';
import { useCallback, useEffect, useRef, useMemo } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Layers, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { KafkaConsumeBatchConfig as KafkaConsumeBatchConfigType } from '@/types/node';
import { useFlowStore } from '@/stores';
import { isKafkaConnectConfig } from '@/types/node';

/** 表单数据类型 */
interface KafkaConsumeBatchFormData {
  type: 'kafka_consume_batch';
  connection_id: string;
  topic: string;
  partition: number;
  offset: number;
  offset_reset: string;
  group_id: string;
  enable_auto_commit: boolean;
  max_messages: number;
  poll_timeout_ms: number;
  deserialization_format: string;
  schema_subject: string;
  batch_strategy: string;
}

interface KafkaConsumeBatchConfigProps {
  data: KafkaConsumeBatchConfigType;
  onSave: (data: KafkaConsumeBatchConfigType) => void;
}

function toFormData(data: KafkaConsumeBatchConfigType): KafkaConsumeBatchFormData {
  return {
    type: 'kafka_consume_batch',
    connection_id: data.connection_id || '',
    topic: data.topic || '',
    partition: data.partition ?? -1,
    offset: data.offset ?? -1,
    offset_reset: data.offset_reset || 'latest',
    group_id: data.group_id || '',
    enable_auto_commit: data.enable_auto_commit ?? false,
    max_messages: data.max_messages ?? 10,
    poll_timeout_ms: data.poll_timeout_ms ?? 10000,
    deserialization_format: data.deserialization_format || 'json',
    schema_subject: data.schema_subject || '',
    batch_strategy: data.batch_strategy || 'count_timeout',
  };
}

function fromFormData(formData: KafkaConsumeBatchFormData): KafkaConsumeBatchConfigType {
  return {
    type: 'kafka_consume_batch',
    connection_id: formData.connection_id,
    topic: formData.topic,
    partition: formData.partition >= 0 ? formData.partition : undefined,
    offset: formData.offset >= 0 ? formData.offset : undefined,
    offset_reset: formData.offset_reset || undefined,
    group_id: formData.group_id || undefined,
    enable_auto_commit: formData.enable_auto_commit || undefined,
    max_messages: formData.max_messages,
    poll_timeout_ms: formData.poll_timeout_ms,
    deserialization_format: (formData.deserialization_format || undefined) as KafkaConsumeBatchConfigType['deserialization_format'],
    schema_subject: formData.schema_subject || undefined,
    batch_strategy: (formData.batch_strategy || undefined) as KafkaConsumeBatchConfigType['batch_strategy'],
  };
}

export function KafkaConsumeBatchConfig({ data, onSave }: KafkaConsumeBatchConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<KafkaConsumeBatchFormData>({
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
      if (isKafkaConnectConfig(node.data)) {
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
        console.error('保存 Kafka ConsumeBatch 配置失败:', error);
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

  // 监听条件字段
  const deserializationFormat = watch('deserialization_format') || 'json';
  const needsSchemaSubject = deserializationFormat === 'avro' || deserializationFormat === 'protobuf';

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-orange-200/50 bg-gradient-to-br from-orange-50/80 to-amber-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-orange-900 flex items-center gap-2">
            <Layers className="h-4 w-4" />
            {getLL().panels.kafkaConsumeBatch.title()}
          </h4>
          <p className="text-sm leading-relaxed text-orange-700/90 whitespace-normal break-words">
            {getLL().panels.kafkaConsumeBatch.description()}
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
              <SelectValue placeholder="{getLL().panels.selectConnection()}" />
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
            placeholder="{getLL().panels.enterConnectionId()}"
            className="font-mono text-sm"
          />
        )}
        {errors.connection_id && (
          <p className="text-xs text-rose-500">连接 ID 不能为空</p>
        )}
        {availableConnections.length === 0 && (
          <p className="text-xs text-muted-foreground">
            未检测到 Kafka Connect 节点，请手动输入连接 ID
          </p>
        )}
      </div>

      {/* 消费配置 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">消费配置</p>

        <div className="space-y-2">
          <Label htmlFor="topic" className="text-sm font-medium text-foreground">
            主题 <span className="text-rose-500">*</span>
          </Label>
          <Input
            id="topic"
            {...register('topic')}
            placeholder="my-topic"
            className="font-mono text-sm"
          />
          {errors.topic && (
            <p className="text-xs text-rose-500">主题不能为空</p>
          )}
        </div>

        <div className="space-y-2">
          <Label htmlFor="partition" className="text-sm font-medium text-foreground">
            分区
          </Label>
          <Input
            id="partition"
            type="number"
            {...register('partition', { valueAsNumber: true })}
            placeholder="-1 表示订阅所有分区"
            className="font-mono text-sm"
          />
          <p className="text-xs text-muted-foreground">
            设为 -1 订阅主题的所有分区
          </p>
        </div>

        <div className="space-y-2">
          <Label htmlFor="offset" className="text-sm font-medium text-foreground">
            起始偏移量
          </Label>
          <Input
            id="offset"
            type="number"
            {...register('offset', { valueAsNumber: true })}
            placeholder="-1 使用偏移重置策略"
            className="font-mono text-sm"
          />
        </div>

        <div className="space-y-2">
          <Label className="text-sm font-medium text-foreground">偏移重置策略</Label>
          <Select
            value={watch('offset_reset') || 'latest'}
            onValueChange={(value) => {
              setValue('offset_reset', value, { shouldDirty: true });
              saveData();
            }}
          >
            <SelectTrigger className="font-mono text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="earliest">earliest — 从最早消息开始</SelectItem>
              <SelectItem value="latest">latest — 从最新消息开始</SelectItem>
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">
            当消费者组无已提交偏移量时生效
          </p>
        </div>
      </div>

      {/* 消费者组 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">消费者组</p>

        <div className="space-y-2">
          <Label htmlFor="group_id" className="text-sm font-medium text-foreground">
            消费者组 ID
          </Label>
          <Input
            id="group_id"
            {...register('group_id')}
            placeholder="可选，留空使用自动分配"
            className="font-mono text-sm"
          />
          <p className="text-xs text-muted-foreground">
            同一消费者组内的消费者共同分担主题分区
          </p>
        </div>

        <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
          <div>
            <Label className="text-sm font-medium text-foreground">自动提交偏移量</Label>
            <p className="text-xs text-muted-foreground">自动周期性提交已消费的偏移量</p>
          </div>
          <Switch
            checked={watch('enable_auto_commit') ?? false}
            onCheckedChange={(checked) => {
              setValue('enable_auto_commit', checked, { shouldDirty: true });
              saveData();
            }}
          />
        </div>
      </div>

      {/* 反序列化 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">反序列化</p>

        <div className="space-y-2">
          <Label className="text-sm font-medium text-foreground">反序列化格式</Label>
          <Select
            value={deserializationFormat}
            onValueChange={(value) => {
              setValue('deserialization_format', value, { shouldDirty: true });
              saveData();
            }}
          >
            <SelectTrigger className="font-mono text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="json">JSON</SelectItem>
              <SelectItem value="avro">Avro</SelectItem>
              <SelectItem value="protobuf">Protobuf</SelectItem>
              <SelectItem value="raw">Raw (原始字节)</SelectItem>
            </SelectContent>
          </Select>
        </div>

        {needsSchemaSubject && (
          <div className="space-y-2">
            <Label htmlFor="schema_subject" className="text-xs font-medium text-foreground">Schema Subject</Label>
            <Input
              id="schema_subject"
              {...register('schema_subject')}
              placeholder="my-topic-value"
              className="h-8 text-xs font-mono"
            />
            <p className="text-xs text-muted-foreground">
              Schema Registry 中的 Subject 名称
            </p>
          </div>
        )}
      </div>

      {/* 高级设置 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.advancedSettings()}</p>

        <div className="space-y-2">
          <Label htmlFor="max_messages" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaConsumeBatch.maxRecords()}
          </Label>
          <Input
            id="max_messages"
            type="number"
            {...register('max_messages', { valueAsNumber: true })}
            placeholder="10"
            className="font-mono text-sm"
          />
          <p className="text-xs text-muted-foreground">
            批量消费的最大消息数量，达到后自动停止
          </p>
        </div>

        <div className="space-y-2">
          <Label htmlFor="poll_timeout_ms" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaConsumeBatch.pollTimeout()}
          </Label>
          <Input
            id="poll_timeout_ms"
            type="number"
            {...register('poll_timeout_ms', { valueAsNumber: true })}
            placeholder="10000"
            className="font-mono text-sm"
          />
        </div>

        <div className="space-y-2">
          <Label className="text-sm font-medium text-foreground">批量策略</Label>
          <Select
            value={watch('batch_strategy') || 'count_timeout'}
            onValueChange={(value) => {
              setValue('batch_strategy', value, { shouldDirty: true });
              saveData();
            }}
          >
            <SelectTrigger className="font-mono text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="count_timeout">count_timeout — 消息数 + 超时</SelectItem>
              <SelectItem value="count_only">count_only — 仅消息数</SelectItem>
              <SelectItem value="timeout_only">timeout_only — 仅超时</SelectItem>
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">
            控制批量消费的结束条件
          </p>
        </div>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">批量消费说明：</p>
          <p>默认最大消费 10 条消息（与单条消费的默认 1 条不同）</p>
          <p>count_timeout 策略：达到消息数或超时即返回</p>
          <p>count_only 策略：仅按消息数判断</p>
          <p>timeout_only 策略：仅按超时时间判断</p>
          <p>使用 Avro/Protobuf 反序列化时需先配置 Schema Registry 连接</p>
        </div>
      </div>
    </form>
  );
}
