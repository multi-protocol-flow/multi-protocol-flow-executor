/**
 * Kafka Admin 节点配置组件
 * 支持主题管理、消费者组管理、Broker 管理、ACL 管理操作
 */
import { useForm } from 'react-hook-form';
import { useCallback, useEffect, useRef, useMemo } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Settings2, Info } from 'lucide-react';
import type { KafkaAdminConfig as KafkaAdminConfigType, KafkaAdminOperation } from '@/types/node';
import { useFlowStore } from '@/stores';
import { isKafkaConnectConfig } from '@/types/node';

/** 表单数据类型 */
interface KafkaAdminFormData {
  type: 'kafka_admin';
  connection_id: string;
  operation: KafkaAdminOperation;
  topic_name: string;
  partitions: number;
  replication_factor: number;
  topic_config: string;
  consumer_group_id: string;
  acl_principal: string;
  acl_host: string;
  acl_operation: string;
  acl_permission_type: string;
  acl_resource_type: string;
  acl_resource_name: string;
  dry_run: boolean;
}

interface KafkaAdminConfigProps {
  data: KafkaAdminConfigType;
  onSave: (data: KafkaAdminConfigType) => void;
}

function parseTopicConfig(raw: string): Record<string, string> | undefined {
  if (!raw.trim()) return undefined;
  const config: Record<string, string> = {};
  for (const line of raw.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const eqIdx = trimmed.indexOf('=');
    if (eqIdx > 0) {
      const key = trimmed.substring(0, eqIdx).trim();
      const val = trimmed.substring(eqIdx + 1).trim();
      if (key) config[key] = val;
    }
  }
  return Object.keys(config).length > 0 ? config : undefined;
}

function topicConfigToString(config: Record<string, string> | undefined): string {
  if (!config || Object.keys(config).length === 0) return '';
  return Object.entries(config).map(([k, v]) => `${k}=${v}`).join('\n');
}

function toFormData(data: KafkaAdminConfigType): KafkaAdminFormData {
  return {
    type: 'kafka_admin',
    connection_id: data.connection_id || '',
    operation: data.operation || 'topic_list',
    topic_name: data.topic_name || '',
    partitions: data.partitions ?? 3,
    replication_factor: data.replication_factor ?? 1,
    topic_config: topicConfigToString(data.topic_config),
    consumer_group_id: data.consumer_group_id || '',
    acl_principal: data.acl_principal || '',
    acl_host: data.acl_host || '',
    acl_operation: data.acl_operation || '',
    acl_permission_type: data.acl_permission_type || '',
    acl_resource_type: data.acl_resource_type || '',
    acl_resource_name: data.acl_resource_name || '',
    dry_run: data.dry_run ?? false,
  };
}

function fromFormData(formData: KafkaAdminFormData): KafkaAdminConfigType {
  return {
    type: 'kafka_admin',
    connection_id: formData.connection_id,
    operation: formData.operation,
    topic_name: formData.topic_name || undefined,
    partitions: formData.partitions || undefined,
    replication_factor: formData.replication_factor || undefined,
    topic_config: parseTopicConfig(formData.topic_config),
    consumer_group_id: formData.consumer_group_id || undefined,
    acl_principal: formData.acl_principal || undefined,
    acl_host: formData.acl_host || undefined,
    acl_operation: formData.acl_operation || undefined,
    acl_permission_type: formData.acl_permission_type || undefined,
    acl_resource_type: formData.acl_resource_type || undefined,
    acl_resource_name: formData.acl_resource_name || undefined,
    dry_run: formData.dry_run || undefined,
  };
}

const OPERATION_GROUPS: Array<{ label: string; operations: Array<{ value: KafkaAdminOperation; label: string }> }> = [
  {
    label: '主题管理',
    operations: [
      { value: 'topic_create', label: '创建主题' },
      { value: 'topic_delete', label: '删除主题' },
      { value: 'topic_describe', label: '描述主题' },
      { value: 'topic_list', label: '列出主题' },
      { value: 'topic_alter_config', label: '修改主题配置' },
    ],
  },
  {
    label: '消费者组管理',
    operations: [
      { value: 'consumer_group_list', label: '列出消费者组' },
      { value: 'consumer_group_describe', label: '描述消费者组' },
      { value: 'consumer_group_reset_offset', label: '重置消费者偏移量' },
    ],
  },
  {
    label: 'Broker 管理',
    operations: [
      { value: 'broker_list', label: '列出 Broker' },
      { value: 'broker_describe_config', label: '描述 Broker 配置' },
    ],
  },
  {
    label: 'ACL 管理',
    operations: [
      { value: 'acl_create', label: '创建 ACL' },
      { value: 'acl_delete', label: '删除 ACL' },
      { value: 'acl_list', label: '列出 ACL' },
    ],
  },
];

export function KafkaAdminConfig({ data, onSave }: KafkaAdminConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<KafkaAdminFormData>({
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
        console.error('保存 Kafka Admin 配置失败:', error);
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
  const operation = watch('operation') || 'topic_list';

  const isTopicOp = operation.startsWith('topic_');
  const isConsumerGroupOp = operation.startsWith('consumer_group_');
  const isAclOp = operation.startsWith('acl_');
  const showDryRun = operation === 'topic_create' || operation === 'topic_delete' || operation === 'acl_create';
  const showTopicConfig = operation === 'topic_create' || operation === 'topic_alter_config';
  const showTopicFields = isTopicOp && operation !== 'topic_list';

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-orange-200/50 bg-gradient-to-br from-orange-50/80 to-amber-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-orange-900 flex items-center gap-2">
            <Settings2 className="h-4 w-4" />
            Kafka 管理节点
          </h4>
          <p className="text-sm leading-relaxed text-orange-700/90 whitespace-normal break-words">
            执行 Kafka 集群管理操作，包括主题、消费者组、Broker 和 ACL 管理。
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
              <SelectValue placeholder="选择 Kafka 连接" />
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
          <p className="text-xs text-rose-500">连接 ID 不能为空</p>
        )}
        {availableConnections.length === 0 && (
          <p className="text-xs text-muted-foreground">
            未检测到 Kafka Connect 节点，请手动输入连接 ID
          </p>
        )}
      </div>

      {/* 管理操作 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">管理操作</p>
        <div className="space-y-2">
          <Label className="text-sm font-medium text-foreground">操作类型 <span className="text-rose-500">*</span></Label>
          <Select
            value={operation}
            onValueChange={(value) => {
              setValue('operation', value as KafkaAdminOperation, { shouldDirty: true });
              saveData();
            }}
          >
            <SelectTrigger className="font-mono text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {OPERATION_GROUPS.map((group) => (
                <SelectItem key={group.label} value={group.operations[0].value} disabled className="font-semibold">
                  — {group.label} —
                </SelectItem>
              )).filter(() => false)}
              {OPERATION_GROUPS.map((group) => (
                <optgroup key={group.label} label={group.label}>
                  {group.operations.map((op) => (
                    <SelectItem key={op.value} value={op.value}>
                      {op.label}
                    </SelectItem>
                  ))}
                </optgroup>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* 主题配置 — 仅 topic_* 操作显示 */}
      {isTopicOp && (
        <div className="space-y-3">
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">主题配置</p>

          {showTopicFields && (
            <div className="space-y-2">
              <Label htmlFor="topic_name" className="text-sm font-medium text-foreground">主题名称</Label>
              <Input
                id="topic_name"
                {...register('topic_name')}
                placeholder="my-topic"
                className="font-mono text-sm"
              />
            </div>
          )}

          {operation === 'topic_create' && (
            <>
              <div className="space-y-2">
                <Label htmlFor="partitions" className="text-sm font-medium text-foreground">分区数</Label>
                <Input
                  id="partitions"
                  type="number"
                  {...register('partitions', { valueAsNumber: true })}
                  placeholder="3"
                  className="font-mono text-sm"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="replication_factor" className="text-sm font-medium text-foreground">副本因子</Label>
                <Input
                  id="replication_factor"
                  type="number"
                  {...register('replication_factor', { valueAsNumber: true })}
                  placeholder="1"
                  className="font-mono text-sm"
                />
              </div>
            </>
          )}

          {showTopicConfig && (
            <div className="space-y-2">
              <Label htmlFor="topic_config" className="text-sm font-medium text-foreground">主题配置</Label>
              <textarea
                id="topic_config"
                {...register('topic_config')}
                placeholder={"key=value 格式，每行一条\n例如:\nretention.ms=604800000\ncleanup.policy=delete"}
                className="flex min-h-[80px] w-full rounded-md border border-input bg-background px-3 py-2 text-xs font-mono ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                rows={4}
              />
              <p className="text-xs text-muted-foreground">
                每行一个键值对，格式: key=value
              </p>
            </div>
          )}
        </div>
      )}

      {/* 消费者组 — 仅 consumer_group_* 操作显示 */}
      {isConsumerGroupOp && (
        <div className="space-y-3">
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">消费者组</p>

          <div className="space-y-2">
            <Label htmlFor="consumer_group_id" className="text-sm font-medium text-foreground">消费者组 ID</Label>
            <Input
              id="consumer_group_id"
              {...register('consumer_group_id')}
              placeholder="my-consumer-group"
              className="font-mono text-sm"
            />
            {operation !== 'consumer_group_list' && (
              <p className="text-xs text-muted-foreground">
                指定要操作的消费者组 ID
              </p>
            )}
          </div>
        </div>
      )}

      {/* ACL 配置 — 仅 acl_* 操作显示 */}
      {isAclOp && (
        <div className="space-y-3">
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">ACL 配置</p>

          <div className="space-y-2">
            <Label htmlFor="acl_principal" className="text-sm font-medium text-foreground">主体 (Principal)</Label>
            <Input
              id="acl_principal"
              {...register('acl_principal')}
              placeholder="User:alice"
              className="font-mono text-sm"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="acl_host" className="text-sm font-medium text-foreground">主机</Label>
            <Input
              id="acl_host"
              {...register('acl_host')}
              placeholder="*"
              className="font-mono text-sm"
            />
          </div>

          <div className="space-y-2">
            <Label className="text-sm font-medium text-foreground">操作</Label>
            <Select
              value={watch('acl_operation') || ''}
              onValueChange={(value) => {
                setValue('acl_operation', value, { shouldDirty: true });
                saveData();
              }}
            >
              <SelectTrigger className="font-mono text-sm">
                <SelectValue placeholder="选择操作" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="Read">Read</SelectItem>
                <SelectItem value="Write">Write</SelectItem>
                <SelectItem value="Create">Create</SelectItem>
                <SelectItem value="Delete">Delete</SelectItem>
                <SelectItem value="Alter">Alter</SelectItem>
                <SelectItem value="Describe">Describe</SelectItem>
                <SelectItem value="All">All</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label className="text-sm font-medium text-foreground">权限类型</Label>
            <Select
              value={watch('acl_permission_type') || ''}
              onValueChange={(value) => {
                setValue('acl_permission_type', value, { shouldDirty: true });
                saveData();
              }}
            >
              <SelectTrigger className="font-mono text-sm">
                <SelectValue placeholder="选择权限类型" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="Allow">Allow</SelectItem>
                <SelectItem value="Deny">Deny</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label className="text-sm font-medium text-foreground">资源类型</Label>
            <Select
              value={watch('acl_resource_type') || ''}
              onValueChange={(value) => {
                setValue('acl_resource_type', value, { shouldDirty: true });
                saveData();
              }}
            >
              <SelectTrigger className="font-mono text-sm">
                <SelectValue placeholder="选择资源类型" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="Topic">Topic</SelectItem>
                <SelectItem value="Group">Group</SelectItem>
                <SelectItem value="Cluster">Cluster</SelectItem>
                <SelectItem value="TransactionalId">TransactionalId</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label htmlFor="acl_resource_name" className="text-sm font-medium text-foreground">资源名称</Label>
            <Input
              id="acl_resource_name"
              {...register('acl_resource_name')}
              placeholder="my-topic 或 *"
              className="font-mono text-sm"
            />
          </div>
        </div>
      )}

      {/* 选项 */}
      {showDryRun && (
        <div className="space-y-3">
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">选项</p>
          <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
            <div>
              <Label className="text-sm font-medium text-foreground">试运行 (Dry Run)</Label>
              <p className="text-xs text-muted-foreground">仅验证操作而不实际执行</p>
            </div>
            <Switch
              checked={watch('dry_run') ?? false}
              onCheckedChange={(checked) => {
                setValue('dry_run', checked, { shouldDirty: true });
                saveData();
              }}
            />
          </div>
        </div>
      )}

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">管理操作说明：</p>
          <p>主题管理：创建、删除、查看和修改主题配置</p>
          <p>消费者组管理：列出、查看消费者组和重置偏移量</p>
          <p>Broker 管理：查看 Broker 列表和配置信息</p>
          <p>ACL 管理：创建、删除和查看访问控制列表</p>
          <p>开启 Dry Run 可在不实际执行的情况下验证操作参数</p>
        </div>
      </div>
    </form>
  );
}
