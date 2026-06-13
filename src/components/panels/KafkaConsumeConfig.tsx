/**
 * Kafka Consume 节点配置组件
 * 支持连接选择、主题、消费者组、偏移量、反序列化、拉取配置
 */
import { useForm } from 'react-hook-form';
import { useCallback, useEffect, useRef, useMemo } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Inbox, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { KafkaConsumeConfig as KafkaConsumeConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { isKafkaConnectConfig } from '@/types/node';

/** 表单数据类型 */
interface KafkaConsumeFormData {
  type: 'kafka_consume';
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
  fetch_min_bytes: number;
  fetch_max_wait_ms: number;
}

interface KafkaConsumeConfigProps {
  data: KafkaConsumeConfigType;
  onSave: (data: KafkaConsumeConfigType) => void;
}

function toFormData(data: KafkaConsumeConfigType): KafkaConsumeFormData {
  return {
    type: 'kafka_consume',
    connection_id: data.connection_id || '',
    topic: data.topic || '',
    partition: data.partition ?? -1,
    offset: data.offset ?? -1,
    offset_reset: data.offset_reset || 'latest',
    group_id: data.group_id || '',
    enable_auto_commit: data.enable_auto_commit ?? false,
    max_messages: data.max_messages ?? 1,
    poll_timeout_ms: data.poll_timeout_ms ?? 5000,
    deserialization_format: data.deserialization_format || 'json',
    schema_subject: data.schema_subject || '',
    fetch_min_bytes: data.fetch_min_bytes ?? 1,
    fetch_max_wait_ms: data.fetch_max_wait_ms ?? 500,
  };
}

function fromFormData(formData: KafkaConsumeFormData): KafkaConsumeConfigType {
  return {
    type: 'kafka_consume',
    connection_id: formData.connection_id,
    topic: formData.topic,
    partition: formData.partition >= 0 ? formData.partition : undefined,
    offset: formData.offset >= 0 ? formData.offset : undefined,
    offset_reset: formData.offset_reset || undefined,
    group_id: formData.group_id || undefined,
    enable_auto_commit: formData.enable_auto_commit || undefined,
    max_messages: formData.max_messages,
    poll_timeout_ms: formData.poll_timeout_ms,
    deserialization_format: (formData.deserialization_format || undefined) as KafkaConsumeConfigType['deserialization_format'],
    schema_subject: formData.schema_subject || undefined,
    fetch_min_bytes: formData.fetch_min_bytes || undefined,
    fetch_max_wait_ms: formData.fetch_max_wait_ms || undefined,
  };
}

export function KafkaConsumeConfig({ data, onSave }: KafkaConsumeConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<KafkaConsumeFormData>({
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
        console.error('保存 Kafka Consume 配置失败:', error);
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
            <Inbox className="h-4 w-4" />
            {getLL().panels.kafkaConsume.title()}
          </h4>
          <p className="text-sm leading-relaxed text-orange-700/90 whitespace-normal break-words">
            {getLL().panels.kafkaConsume.description()}
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
          <p className="text-xs text-rose-500">{getLL().panels.kafkaConsume.error_connectionIdRequired()}</p>
        )}
        {availableConnections.length === 0 && (
          <p className="text-xs text-muted-foreground">
            {getLL().panels.kafkaConsume.noConnectNode()}
          </p>
        )}
      </div>

      {/* 消费配置 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.kafkaConsume.consumerConfig_sectionTitle()}</p>

        <div className="space-y-2">
          <Label htmlFor="topic" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaConsume.topic_label()} <span className="text-rose-500">*</span>
          </Label>
          <Input
            id="topic"
            {...register('topic')}
            placeholder={getLL().panels.topicPatternPlaceholder()}
            className="font-mono text-sm"
          />
          {errors.topic && (
            <p className="text-xs text-rose-500">{getLL().panels.kafkaConsume.error_topicRequired()}</p>
          )}
        </div>

        <div className="space-y-2">
          <Label htmlFor="partition" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaConsume.partition_label()}
          </Label>
          <Input
            id="partition"
            type="number"
            {...register('partition', { valueAsNumber: true })}
            placeholder={getLL().panels.subscribeAllPartitions()}
            className="font-mono text-sm"
          />
          <p className="text-xs text-muted-foreground">
            {getLL().panels.kafkaConsume.partition_help()}
          </p>
        </div>

        <div className="space-y-2">
          <Label htmlFor="offset" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaConsume.offset_label()}
          </Label>
          <Input
            id="offset"
            type="number"
            {...register('offset', { valueAsNumber: true })}
            placeholder={getLL().panels.useOffsetResetPolicy()}
            className="font-mono text-sm"
          />
        </div>

        <div className="space-y-2">
          <Label className="text-sm font-medium text-foreground">{getLL().panels.kafkaConsume.offsetReset_label()}</Label>
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
              <SelectItem value="earliest">{getLL().panels.kafkaConsume.earliest_option()}</SelectItem>
              <SelectItem value="latest">{getLL().panels.kafkaConsume.latest_option()}</SelectItem>
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">
            {getLL().panels.kafkaConsume.offsetReset_help()}
          </p>
        </div>
      </div>

      {/* 消费者组 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.kafkaConsume.consumerGroup_sectionTitle()}</p>

        <div className="space-y-2">
          <Label htmlFor="group_id" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaConsume.groupId_label()}
          </Label>
          <Input
            id="group_id"
            {...register('group_id')}
            placeholder={getLL().panels.autoAssignHint()}
            className="font-mono text-sm"
          />
          <p className="text-xs text-muted-foreground">
            {getLL().panels.kafkaConsume.groupId_help()}
          </p>
        </div>

        <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
          <div>
            <Label className="text-sm font-medium text-foreground">{getLL().panels.kafkaConsume.autoCommit_label()}</Label>
            <p className="text-xs text-muted-foreground">{getLL().panels.kafkaConsume.autoCommit_desc()}</p>
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
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.kafkaConsume.deserialization_sectionTitle()}</p>

        <div className="space-y-2">
          <Label className="text-sm font-medium text-foreground">{getLL().panels.kafkaConsume.deserialization_label()}</Label>
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
              <SelectItem value="raw">{getLL().panels.kafkaConsume.raw_option()}</SelectItem>
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
              {getLL().panels.kafkaConsume.schemaSubject_help()}
            </p>
          </div>
        )}
      </div>

      {/* 高级设置 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.advancedSettings()}</p>

        <div className="space-y-2">
          <Label htmlFor="max_messages" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaConsume.maxRecords()}
          </Label>
          <Input
            id="max_messages"
            type="number"
            {...register('max_messages', { valueAsNumber: true })}
            placeholder="1"
            className="font-mono text-sm"
          />
          <p className="text-xs text-muted-foreground">
            {getLL().panels.kafkaConsume.maxRecords_help()}
          </p>
        </div>

        <div className="space-y-2">
          <Label htmlFor="poll_timeout_ms" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaConsume.pollTimeout()}
          </Label>
          <Input
            id="poll_timeout_ms"
            type="number"
            {...register('poll_timeout_ms', { valueAsNumber: true })}
            placeholder="5000"
            className="font-mono text-sm"
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="fetch_min_bytes" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaConsume.fetchMinBytes_label()}
          </Label>
          <Input
            id="fetch_min_bytes"
            type="number"
            {...register('fetch_min_bytes', { valueAsNumber: true })}
            placeholder="1"
            className="font-mono text-sm"
          />
          <p className="text-xs text-muted-foreground">
            {getLL().panels.kafkaConsume.fetchMinBytes_help()}
          </p>
        </div>

        <div className="space-y-2">
          <Label htmlFor="fetch_max_wait_ms" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaConsume.fetchMaxWait_label()}
          </Label>
          <Input
            id="fetch_max_wait_ms"
            type="number"
            {...register('fetch_max_wait_ms', { valueAsNumber: true })}
            placeholder="500"
            className="font-mono text-sm"
          />
          <p className="text-xs text-muted-foreground">
            {getLL().panels.kafkaConsume.fetchMaxWait_help()}
          </p>
        </div>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.kafkaConsume.help_title()}</p>
          <p>{getLL().panels.kafkaConsume.help_line1()}</p>
          <p>{getLL().panels.kafkaConsume.help_line2()}</p>
          <p>{getLL().panels.kafkaConsume.help_line3()}</p>
        </div>
      </div>
    </form>
  );
}
