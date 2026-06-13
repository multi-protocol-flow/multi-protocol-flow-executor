/**
 * Kafka Produce 节点配置组件
 * 支持连接选择、主题、键值、消息头、序列化、压缩
 */
import { useForm } from 'react-hook-form';
import { useState, useCallback, useEffect, useRef, useMemo } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Send, Info, Plus, Trash2 } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { KafkaProduceConfig as KafkaProduceConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { isKafkaConnectConfig } from '@/types/node';

/** 表单数据类型 */
interface KafkaProduceFormData {
  type: 'kafka_produce';
  connection_id: string;
  topic: string;
  key: string;
  value: string;
  headers: [string, string][];
  partition: number;
  serialization_format: string;
  schema_subject: string;
  ack_timeout_ms: number;
  compression: string;
}

interface KafkaProduceConfigProps {
  data: KafkaProduceConfigType;
  onSave: (data: KafkaProduceConfigType) => void;
}

function toFormData(data: KafkaProduceConfigType): KafkaProduceFormData {
  return {
    type: 'kafka_produce',
    connection_id: data.connection_id || '',
    topic: data.topic || '',
    key: data.key || '',
    value: data.value || '',
    headers: data.headers || [],
    partition: data.partition ?? -1,
    serialization_format: data.serialization_format || 'json',
    schema_subject: data.schema_subject || '',
    ack_timeout_ms: data.ack_timeout_ms ?? 5000,
    compression: data.compression || 'none',
  };
}

function fromFormData(formData: KafkaProduceFormData): KafkaProduceConfigType {
  const headers = formData.headers?.filter(pair => pair[0].trim());

  return {
    type: 'kafka_produce',
    connection_id: formData.connection_id,
    topic: formData.topic,
    key: formData.key || undefined,
    value: formData.value || undefined,
    headers: headers && headers.length > 0 ? headers : undefined,
    partition: formData.partition >= 0 ? formData.partition : undefined,
    serialization_format: (formData.serialization_format || undefined) as KafkaProduceConfigType['serialization_format'],
    schema_subject: formData.schema_subject || undefined,
    ack_timeout_ms: formData.ack_timeout_ms,
    compression: (formData.compression || undefined) as KafkaProduceConfigType['compression'],
  };
}

export function KafkaProduceConfig({ data, onSave }: KafkaProduceConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<KafkaProduceFormData>({
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
        console.error('保存 Kafka Produce 配置失败:', error);
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

  // 消息头本地状态
  const [headerPairs, setHeaderPairs] = useState<Array<[string, string]>>(
    data.headers || []
  );

  useEffect(() => {
    setHeaderPairs(data.headers || []);
  }, [data.headers]);

  const syncHeaders = useCallback((newHeaders: Array<[string, string]>) => {
    setHeaderPairs(newHeaders);
    setValue('headers', newHeaders, { shouldDirty: true });
  }, [setValue]);

  const handleAddHeader = useCallback(() => {
    const newHeaders = [...headerPairs, ['', ''] as [string, string]];
    syncHeaders(newHeaders);
    safeSetTimeout(saveData, 100);
  }, [headerPairs, syncHeaders, safeSetTimeout, saveData]);

  const handleRemoveHeader = useCallback((index: number) => {
    const newHeaders = headerPairs.filter((_, i) => i !== index);
    syncHeaders(newHeaders);
    safeSetTimeout(saveData, 100);
  }, [headerPairs, syncHeaders, safeSetTimeout, saveData]);

  const handleHeaderChange = useCallback((index: number, position: 0 | 1, value: string) => {
    const newHeaders = headerPairs.map((pair, i) => {
      if (i !== index) return pair;
      const updated: [string, string] = [...pair];
      updated[position] = value;
      return updated;
    });
    syncHeaders(newHeaders);
  }, [headerPairs, syncHeaders]);

  // 监听序列化格式
  const serializationFormat = watch('serialization_format') || 'json';
  const needsSchemaSubject = serializationFormat === 'avro' || serializationFormat === 'protobuf';

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-orange-200/50 bg-gradient-to-br from-orange-50/80 to-amber-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-orange-900 flex items-center gap-2">
            <Send className="h-4 w-4" />
            {getLL().panels.kafkaProduce.title()}
          </h4>
          <p className="text-sm leading-relaxed text-orange-700/90 whitespace-normal break-words">
            {getLL().panels.kafkaProduce.description()}
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
          <p className="text-xs text-rose-500">{getLL().panels.kafkaProduce.error_connectionIdRequired()}</p>
        )}
        {availableConnections.length === 0 && (
          <p className="text-xs text-muted-foreground">
            {getLL().panels.kafkaProduce.noConnectNode()}
          </p>
        )}
      </div>

      {/* 消息配置 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.kafkaProduce.msgConfig_sectionTitle()}</p>

        <div className="space-y-2">
          <Label htmlFor="topic" className="text-sm font-medium text-foreground">{getLL().panels.kafkaProduce.topic()} <span className="text-rose-500">*</span>
          </Label>
          <Input
            id="topic"
            {...register('topic')}
            placeholder={getLL().panels.topicPatternPlaceholder()}
            className="font-mono text-sm"
          />
          {errors.topic && (
            <p className="text-xs text-rose-500">{getLL().panels.kafkaProduce.error_topicRequired()}</p>
          )}
        </div>

        <div className="space-y-2">
          <Label htmlFor="key" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaProduce.key()}
          </Label>
          <Input
            id="key"
            {...register('key')}
            placeholder={getLL().panels.partitionRoutingHint()}
            className="font-mono text-sm"
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="value" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaProduce.value()}
          </Label>
          <Textarea
            id="value"
            {...register('value')}
            placeholder='{"key": "value"}'
            className="min-h-[80px] resize-y font-mono text-xs"
            rows={4}
          />
        </div>
      </div>

      {/* 消息头 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">{getLL().panels.kafkaProduce.headers()}</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddHeader}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />{getLL().panels.add()}</Button>
        </div>
        <div className="space-y-2">
          {headerPairs.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">{getLL().panels.kafkaProduce.noHeaders()}</p>
            </div>
          ) : (
            headerPairs.map((pair, index) => (
              <div key={index} className="flex items-center gap-2 p-2 rounded-md border bg-muted/30">
                <Input
                  value={pair[0]}
                  onChange={(e) => handleHeaderChange(index, 0, e.target.value)}
                  onBlur={() => safeSetTimeout(saveData, 100)}
                  placeholder="Key"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Input
                  value={pair[1]}
                  onChange={(e) => handleHeaderChange(index, 1, e.target.value)}
                  onBlur={() => safeSetTimeout(saveData, 100)}
                  placeholder="Value"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  onClick={() => handleRemoveHeader(index)}
                  className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            ))
          )}
        </div>
      </div>

      {/* 序列化 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.kafkaProduce.serialization_sectionTitle()}</p>

        <div className="space-y-2">
          <Label className="text-sm font-medium text-foreground">{getLL().panels.kafkaProduce.serialization_label()}</Label>
          <Select
            value={serializationFormat}
            onValueChange={(value) => {
              setValue('serialization_format', value, { shouldDirty: true });
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
              <SelectItem value="raw">{getLL().panels.kafkaProduce.raw_option()}</SelectItem>
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
              {getLL().panels.kafkaProduce.schemaSubject_help()}
            </p>
          </div>
        )}
      </div>

      {/* 高级设置 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.advancedSettings()}</p>

        <div className="space-y-2">
          <Label htmlFor="partition" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaProduce.partition()}
          </Label>
          <Input
            id="partition"
            type="number"
            {...register('partition', { valueAsNumber: true })}
            placeholder={getLL().panels.autoSelectHint()}
            className="font-mono text-sm"
          />
          <p className="text-xs text-muted-foreground">
            {getLL().panels.kafkaProduce.partition_help()}
          </p>
        </div>

        <div className="space-y-2">
          <Label htmlFor="ack_timeout_ms" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaProduce.ackTimeout_label()}
          </Label>
          <Input
            id="ack_timeout_ms"
            type="number"
            {...register('ack_timeout_ms', { valueAsNumber: true })}
            placeholder="5000"
            className="font-mono text-sm"
          />
        </div>

        <div className="space-y-2">
          <Label className="text-sm font-medium text-foreground">{getLL().panels.kafkaProduce.compression_label()}</Label>
          <Select
            value={watch('compression') || 'none'}
            onValueChange={(value) => {
              setValue('compression', value, { shouldDirty: true });
              saveData();
            }}
          >
            <SelectTrigger className="font-mono text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">{getLL().panels.kafkaProduce.noCompression_option()}</SelectItem>
              <SelectItem value="gzip">Gzip</SelectItem>
              <SelectItem value="snappy">Snappy</SelectItem>
              <SelectItem value="lz4">LZ4</SelectItem>
              <SelectItem value="zstd">Zstd</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.kafkaProduce.help_title()}</p>
          <p>{getLL().panels.kafkaProduce.help_line1()}</p>
          <p>{getLL().panels.kafkaProduce.help_line2()}</p>
          <p>{getLL().panels.kafkaProduce.help_line3()}</p>
        </div>
      </div>
    </form>
  );
}
