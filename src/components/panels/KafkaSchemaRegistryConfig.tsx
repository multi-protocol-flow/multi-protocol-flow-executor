/**
 * Kafka Schema Registry 节点配置组件
 * 支持 Schema 管理、版本查询、兼容性检查、Schema 注册
 */
import { useForm } from 'react-hook-form';
import { useCallback, useEffect, useRef, useMemo } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Database, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { KafkaSchemaRegistryConfig as KafkaSchemaRegistryConfigType, KafkaSchemaOperation } from '@/types/node';
import { useFlowStore } from '@/stores';
import { isKafkaConnectConfig } from '@/types/node';

/** 表单数据类型 */
interface KafkaSchemaRegistryFormData {
  type: 'kafka_schema_registry';
  connection_id: string;
  operation: KafkaSchemaOperation;
  subject: string;
  version: number;
  schema_definition: string;
  schema_type: string;
  compatibility_level: string;
}

interface KafkaSchemaRegistryConfigProps {
  data: KafkaSchemaRegistryConfigType;
  onSave: (data: KafkaSchemaRegistryConfigType) => void;
}

function toFormData(data: KafkaSchemaRegistryConfigType): KafkaSchemaRegistryFormData {
  return {
    type: 'kafka_schema_registry',
    connection_id: data.connection_id || '',
    operation: data.operation || 'list_subjects',
    subject: data.subject || '',
    version: data.version ?? -1,
    schema_definition: data.schema_definition || '',
    schema_type: data.schema_type || 'avro',
    compatibility_level: data.compatibility_level || 'BACKWARD',
  };
}

function fromFormData(formData: KafkaSchemaRegistryFormData): KafkaSchemaRegistryConfigType {
  return {
    type: 'kafka_schema_registry',
    connection_id: formData.connection_id,
    operation: formData.operation,
    subject: formData.subject || undefined,
    version: formData.version >= 0 ? formData.version : undefined,
    schema_definition: formData.schema_definition || undefined,
    schema_type: formData.schema_type || undefined,
    compatibility_level: formData.compatibility_level || undefined,
  };
}

function getSchemaOperations(): Array<{ value: KafkaSchemaOperation; label: string }> {
  return [
    { value: 'list_subjects', label: getLL().panels.kafkaSchemaRegistry.operations_listSubjects() },
    { value: 'get_subject_versions', label: getLL().panels.kafkaSchemaRegistry.operations_getSubjectVersions() },
    { value: 'get_schema_by_version', label: getLL().panels.kafkaSchemaRegistry.operations_getSchemaByVersion() },
    { value: 'check_compatibility', label: getLL().panels.kafkaSchemaRegistry.operations_checkCompatibility() },
    { value: 'register_schema', label: getLL().panels.kafkaSchemaRegistry.operations_registerSchema() },
  ];
}

export function KafkaSchemaRegistryConfig({ data, onSave }: KafkaSchemaRegistryConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<KafkaSchemaRegistryFormData>({
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
        console.error('保存 Kafka Schema Registry 配置失败:', error);
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

  // Schema 操作选项（懒计算）
  const schemaOperations = useMemo(() => getSchemaOperations(), []);
  // 监听操作类型
  const operation = watch('operation') || 'list_subjects';
  const showSubject = operation !== 'list_subjects';
  const showVersion = operation === 'get_schema_by_version';
  const showSchemaDefinition = operation === 'register_schema' || operation === 'check_compatibility';
  const showSchemaType = operation === 'register_schema';
  const showCompatibilityLevel = operation === 'check_compatibility';

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-orange-200/50 bg-gradient-to-br from-orange-50/80 to-amber-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-orange-900 flex items-center gap-2">
            <Database className="h-4 w-4" />
            {getLL().panels.kafkaSchemaRegistry.title()}
          </h4>
          <p className="text-sm leading-relaxed text-orange-700/90 whitespace-normal break-words">
            {getLL().panels.kafkaSchemaRegistry.description()}
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
          <p className="text-xs text-rose-500">{getLL().panels.kafkaSchemaRegistry.error_connectionIdRequired()}</p>
        )}
        {availableConnections.length === 0 && (
          <p className="text-xs text-muted-foreground">
            {getLL().panels.kafkaSchemaRegistry.noConnectNode()}
          </p>
        )}
      </div>

      {/* 操作选择 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.selectOperation()}</p>
        <div className="space-y-2">
          <Label className="text-sm font-medium text-foreground">{getLL().panels.kafkaAdmin.operation()} <span className="text-rose-500">*</span></Label>
          <Select
            value={operation}
            onValueChange={(value) => {
              setValue('operation', value as KafkaSchemaOperation, { shouldDirty: true });
              saveData();
            }}
          >
            <SelectTrigger className="font-mono text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {schemaOperations.map((op) => (
                <SelectItem key={op.value} value={op.value}>
                  {op.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* Subject */}
      {showSubject && (
        <div className="space-y-3">
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.kafkaSchemaRegistry.schemaSubject()}</p>
          <div className="space-y-2">
            <Label htmlFor="subject" className="text-sm font-medium text-foreground">{getLL().panels.kafkaSchemaRegistry.schemaSubject()}</Label>
            <Input
              id="subject"
              {...register('subject')}
              placeholder="my-topic-value"
              className="font-mono text-sm"
            />
            <p className="text-xs text-muted-foreground">
              {getLL().panels.kafkaSchemaRegistry.subjectDesc()}
            </p>
          </div>
        </div>
      )}

      {/* 版本 */}
      {showVersion && (
        <div className="space-y-3">
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.kafkaSchemaRegistry.schemaVersion()}</p>
          <div className="space-y-2">
            <Label htmlFor="version" className="text-sm font-medium text-foreground">{getLL().panels.kafkaSchemaRegistry.schemaVersion()}</Label>
            <Input
              id="version"
              type="number"
              {...register('version', { valueAsNumber: true })}
              placeholder={getLL().panels.latestVersionHint()}
              className="font-mono text-sm"
            />
            <p className="text-xs text-muted-foreground">
              {getLL().panels.kafkaSchemaRegistry.versionHint()}
            </p>
          </div>
        </div>
      )}

      {/* Schema 定义 */}
      {showSchemaDefinition && (
        <div className="space-y-3">
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.kafkaSchemaRegistry.schemaSubject()}</p>
          <div className="space-y-2">
            <Label htmlFor="schema_definition" className="text-sm font-medium text-foreground">Schema 内容</Label>
            <textarea
              id="schema_definition"
              {...register('schema_definition')}
              placeholder={getLL().panels.schemaDefinitionHint()}
              className="flex min-h-[120px] w-full rounded-md border border-input bg-background px-3 py-2 text-xs font-mono ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
              rows={6}
            />
          </div>
        </div>
      )}

      {/* Schema 类型 */}
      {showSchemaType && (
        <div className="space-y-3">
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">Schema 类型</p>
          <div className="space-y-2">
            <Label className="text-sm font-medium text-foreground">{getLL().panels.kafkaSchemaRegistry.schemaType()}</Label>
            <Select
              value={watch('schema_type') || 'avro'}
              onValueChange={(value) => {
                setValue('schema_type', value, { shouldDirty: true });
                saveData();
              }}
            >
              <SelectTrigger className="font-mono text-sm">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="avro">Avro</SelectItem>
                <SelectItem value="protobuf">Protobuf</SelectItem>
                <SelectItem value="json_schema">JSON Schema</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
      )}

      {/* 兼容性级别 */}
      {showCompatibilityLevel && (
        <div className="space-y-3">
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">兼容性检查</p>
          <div className="space-y-2">
            <Label className="text-sm font-medium text-foreground">{getLL().panels.kafkaSchemaRegistry.compatLevel_label()}</Label>
            <Select
              value={watch('compatibility_level') || 'BACKWARD'}
              onValueChange={(value) => {
                setValue('compatibility_level', value, { shouldDirty: true });
                saveData();
              }}
            >
              <SelectTrigger className="font-mono text-sm">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="BACKWARD">{getLL().panels.kafkaSchemaRegistry.backward_option()}</SelectItem>
                <SelectItem value="FORWARD">{getLL().panels.kafkaSchemaRegistry.forward_option()}</SelectItem>
                <SelectItem value="FULL">{getLL().panels.kafkaSchemaRegistry.full_option()}</SelectItem>
                <SelectItem value="NONE">{getLL().panels.kafkaSchemaRegistry.none_option()}</SelectItem>
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              {getLL().panels.kafkaSchemaRegistry.compatHint()}
            </p>
          </div>
        </div>
      )}

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.kafkaSchemaRegistry.help_title()}</p>
          <p>{getLL().panels.kafkaSchemaRegistry.help_line1()}</p>
          <p>{getLL().panels.kafkaSchemaRegistry.help_line2()}</p>
          <p>{getLL().panels.kafkaSchemaRegistry.help_line3()}</p>
          <p>{getLL().panels.kafkaSchemaRegistry.help_line4()}</p>
        </div>
      </div>
    </form>
  );
}
