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

const SCHEMA_OPERATIONS: Array<{ value: KafkaSchemaOperation; label: string }> = [
  { value: 'list_subjects', label: '列出 Subjects' },
  { value: 'get_subject_versions', label: '获取 Subject 版本列表' },
  { value: 'get_schema_by_version', label: '按版本获取 Schema' },
  { value: 'check_compatibility', label: '检查兼容性' },
  { value: 'register_schema', label: '注册 Schema' },
];

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

      {/* 操作选择 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">操作类型</p>
        <div className="space-y-2">
          <Label className="text-sm font-medium text-foreground">操作 <span className="text-rose-500">*</span></Label>
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
              {SCHEMA_OPERATIONS.map((op) => (
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
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">Schema Subject</p>
          <div className="space-y-2">
            <Label htmlFor="subject" className="text-sm font-medium text-foreground">{getLL().panels.kafkaSchemaRegistry.schemaSubject()}</Label>
            <Input
              id="subject"
              {...register('subject')}
              placeholder="my-topic-value"
              className="font-mono text-sm"
            />
            <p className="text-xs text-muted-foreground">
              Schema Registry 中的 Subject 名称
            </p>
          </div>
        </div>
      )}

      {/* 版本 */}
      {showVersion && (
        <div className="space-y-3">
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">版本信息</p>
          <div className="space-y-2">
            <Label htmlFor="version" className="text-sm font-medium text-foreground">{getLL().panels.kafkaSchemaRegistry.schemaVersion()}</Label>
            <Input
              id="version"
              type="number"
              {...register('version', { valueAsNumber: true })}
              placeholder="-1 = Latest version"
              className="font-mono text-sm"
            />
            <p className="text-xs text-muted-foreground">
              设为 -1 获取最新版本
            </p>
          </div>
        </div>
      )}

      {/* Schema 定义 */}
      {showSchemaDefinition && (
        <div className="space-y-3">
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">Schema 定义</p>
          <div className="space-y-2">
            <Label htmlFor="schema_definition" className="text-sm font-medium text-foreground">Schema 内容</Label>
            <textarea
              id="schema_definition"
              {...register('schema_definition')}
              placeholder={'Enter Schema definition (JSON format)\ne.g.:\n{"type": "record", "name": "User", "fields": [...]}'}
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
            <Label className="text-sm font-medium text-foreground">兼容性级别</Label>
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
                <SelectItem value="BACKWARD">BACKWARD — 向后兼容</SelectItem>
                <SelectItem value="FORWARD">FORWARD — 向前兼容</SelectItem>
                <SelectItem value="FULL">FULL — 完全兼容</SelectItem>
                <SelectItem value="NONE">NONE — 无兼容性要求</SelectItem>
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              指定检查 Schema 兼容性时使用的级别
            </p>
          </div>
        </div>
      )}

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">Schema Registry 说明：</p>
          <p>需先在 Kafka Connect 节点中配置 Schema Registry URL</p>
          <p>列出 Subjects 操作无需指定 Subject 名称</p>
          <p>注册 Schema 时需提供完整的 Schema 定义和格式</p>
          <p>兼容性检查会验证新 Schema 是否与现有版本兼容</p>
        </div>
      </div>
    </form>
  );
}
