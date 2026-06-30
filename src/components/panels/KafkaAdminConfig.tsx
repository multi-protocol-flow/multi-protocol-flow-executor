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
import { getLL } from '@/i18n/i18n-lazy';
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

function getOperationGroups(): Array<{ label: string; operations: Array<{ value: KafkaAdminOperation; label: string }> }> {
  return [
    {
      label: getLL().panels.kafkaAdmin.operations_topicManagement(),
      operations: [
        { value: 'topic_create', label: getLL().panels.kafkaAdmin.operations_createTopic() },
        { value: 'topic_delete', label: getLL().panels.kafkaAdmin.operations_deleteTopic() },
        { value: 'topic_describe', label: getLL().panels.kafkaAdmin.operations_describeTopic() },
        { value: 'topic_list', label: getLL().panels.kafkaAdmin.operations_listTopics() },
        { value: 'topic_alter_config', label: getLL().panels.kafkaAdmin.operations_modifyTopicConfig() },
      ],
    },
    {
      label: getLL().panels.kafkaAdmin.operations_consumerGroupManagement(),
      operations: [
        { value: 'consumer_group_list', label: getLL().panels.kafkaAdmin.operations_listConsumerGroups() },
        { value: 'consumer_group_describe', label: getLL().panels.kafkaAdmin.operations_describeConsumerGroups() },
        { value: 'consumer_group_reset_offset', label: getLL().panels.kafkaAdmin.operations_resetOffsets() },
      ],
    },
    {
      label: getLL().panels.kafkaAdmin.operations_brokerManagement(),
      operations: [
        { value: 'broker_list', label: getLL().panels.kafkaAdmin.operations_listBrokers() },
        { value: 'broker_describe_config', label: getLL().panels.kafkaAdmin.operations_describeBrokerConfig() },
      ],
    },
    {
      label: getLL().panels.kafkaAdmin.operations_aclManagement(),
      operations: [
        { value: 'acl_create', label: getLL().panels.kafkaAdmin.operations_createAcl() },
        { value: 'acl_delete', label: getLL().panels.kafkaAdmin.operations_deleteAcl() },
        { value: 'acl_list', label: getLL().panels.kafkaAdmin.operations_listAcls() },
      ],
    },
  ];
}

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

  // 操作分组（懒计算，依赖 getLL）
  const operationGroups = useMemo(() => getOperationGroups(), []);
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
            {getLL().panels.kafkaAdmin.title()}
          </h4>
          <p className="text-sm leading-relaxed text-orange-700/90 whitespace-normal break-words">
            {getLL().panels.kafkaAdmin.description()}
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
          <p className="text-xs text-rose-500">{getLL().panels.kafkaAdmin.error_connectionIdRequired()}</p>
        )}
        {availableConnections.length === 0 && (
          <p className="text-xs text-muted-foreground">
            {getLL().panels.kafkaAdmin.noConnectNode()}
          </p>
        )}
      </div>

      {/* 管理操作 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.selectOperation()}</p>
        <div className="space-y-2">
          <Label className="text-sm font-medium text-foreground">{getLL().panels.kafkaAdmin.operation()} <span className="text-rose-500">*</span></Label>
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
              {operationGroups.map((group) => (
                <SelectItem key={group.label} value={group.operations[0].value} disabled className="font-semibold">
                  — {group.label} —
                </SelectItem>
              )).filter(() => false)}
              {operationGroups.map((group) => (
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
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.kafkaAdmin.topicConfig_sectionTitle()}</p>

          {showTopicFields && (
            <div className="space-y-2">
              <Label htmlFor="topic_name" className="text-sm font-medium text-foreground">{getLL().panels.kafkaAdmin.topic()}</Label>
              <Input
                id="topic_name"
                {...register('topic_name')}
                placeholder={getLL().panels.topicPatternPlaceholder()}
                className="font-mono text-sm"
              />
            </div>
          )}

          {operation === 'topic_create' && (
            <>
              <div className="space-y-2">
                <Label htmlFor="partitions" className="text-sm font-medium text-foreground">{getLL().panels.kafkaAdmin.partitions()}</Label>
                <Input
                  id="partitions"
                  type="number"
                  {...register('partitions', { valueAsNumber: true })}
                  placeholder="3"
                  className="font-mono text-sm"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="replication_factor" className="text-sm font-medium text-foreground">{getLL().panels.kafkaAdmin.replicationFactor()}</Label>
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
              <Label htmlFor="topic_config" className="text-sm font-medium text-foreground">{getLL().panels.kafkaAdmin.topicConfig_label()}</Label>
              <textarea
                id="topic_config"
                {...register('topic_config')}
                placeholder={getLL().panels.keyValueFormatHint()}
                className="flex min-h-[80px] w-full rounded-md border border-input bg-background px-3 py-2 text-xs font-mono ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                rows={4}
              />
              <p className="text-xs text-muted-foreground">
                {getLL().panels.kafkaAdmin.topicConfig_help()}
              </p>
            </div>
          )}
        </div>
      )}

      {/* 消费者组 — 仅 consumer_group_* 操作显示 */}
      {isConsumerGroupOp && (
        <div className="space-y-3">
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.kafkaAdmin.consumerGroup_sectionTitle()}</p>

          <div className="space-y-2">
            <Label htmlFor="consumer_group_id" className="text-sm font-medium text-foreground">{getLL().panels.kafkaAdmin.groupId_label()}</Label>
            <Input
              id="consumer_group_id"
              {...register('consumer_group_id')}
              placeholder="my-consumer-group"
              className="font-mono text-sm"
            />
            {operation !== 'consumer_group_list' && (
              <p className="text-xs text-muted-foreground">
                {getLL().panels.kafkaAdmin.groupId_help()}
              </p>
            )}
          </div>
        </div>
      )}

      {/* ACL 配置 — 仅 acl_* 操作显示 */}
      {isAclOp && (
        <div className="space-y-3">
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.kafkaAdmin.aclConfig_sectionTitle()}</p>

          <div className="space-y-2">
            <Label htmlFor="acl_principal" className="text-sm font-medium text-foreground">{getLL().panels.kafkaAdmin.aclPrincipal_label()}</Label>
            <Input
              id="acl_principal"
              {...register('acl_principal')}
              placeholder="User:alice"
              className="font-mono text-sm"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="acl_host" className="text-sm font-medium text-foreground">{getLL().panels.kafkaAdmin.aclHost_label()}</Label>
            <Input
              id="acl_host"
              {...register('acl_host')}
              placeholder="*"
              className="font-mono text-sm"
            />
          </div>

          <div className="space-y-2">
            <Label className="text-sm font-medium text-foreground">{getLL().panels.kafkaAdmin.aclOperation_label()}</Label>
            <Select
              value={watch('acl_operation') || ''}
              onValueChange={(value) => {
                setValue('acl_operation', value, { shouldDirty: true });
                saveData();
              }}
            >
              <SelectTrigger className="font-mono text-sm">
                <SelectValue placeholder={getLL().panels.selectOperation()} />
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
            <Label className="text-sm font-medium text-foreground">{getLL().panels.kafkaAdmin.aclPermissionType_label()}</Label>
            <Select
              value={watch('acl_permission_type') || ''}
              onValueChange={(value) => {
                setValue('acl_permission_type', value, { shouldDirty: true });
                saveData();
              }}
            >
              <SelectTrigger className="font-mono text-sm">
                <SelectValue placeholder={getLL().panels.selectPermissionType()} />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="Allow">Allow</SelectItem>
                <SelectItem value="Deny">Deny</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label className="text-sm font-medium text-foreground">{getLL().panels.kafkaAdmin.aclResourceType_label()}</Label>
            <Select
              value={watch('acl_resource_type') || ''}
              onValueChange={(value) => {
                setValue('acl_resource_type', value, { shouldDirty: true });
                saveData();
              }}
            >
              <SelectTrigger className="font-mono text-sm">
                <SelectValue placeholder={getLL().panels.selectResourceType()} />
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
            <Label htmlFor="acl_resource_name" className="text-sm font-medium text-foreground">{getLL().panels.kafkaAdmin.aclResourceName_label()}</Label>
            <Input
              id="acl_resource_name"
              {...register('acl_resource_name')}
              placeholder={getLL().panels.topicPatternPlaceholder()}
              className="font-mono text-sm"
            />
          </div>
        </div>
      )}

      {/* 选项 */}
      {showDryRun && (
        <div className="space-y-3">
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.kafkaAdmin.options_sectionTitle()}</p>
          <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
            <div>
              <Label className="text-sm font-medium text-foreground">{getLL().panels.kafkaAdmin.dryRun_label()}</Label>
              <p className="text-xs text-muted-foreground">{getLL().panels.kafkaAdmin.dryRun_desc()}</p>
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
          <p className="font-medium text-foreground">{getLL().panels.kafkaAdmin.help_title()}</p>
          <p>{getLL().panels.kafkaAdmin.help_line1()}</p>
          <p>{getLL().panels.kafkaAdmin.help_line2()}</p>
          <p>{getLL().panels.kafkaAdmin.help_line3()}</p>
          <p>{getLL().panels.kafkaAdmin.help_line4()}</p>
          <p>{getLL().panels.kafkaAdmin.help_line5()}</p>
        </div>
      </div>
    </form>
  );
}
