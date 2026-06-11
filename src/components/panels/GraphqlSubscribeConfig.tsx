/**
 * GraphQL Subscribe 节点配置组件
 * 支持连接 ID、订阅编辑器、变量编辑器、操作名称、最大事件数、超时
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  graphqlSubscribeConfigSchema,
  type GraphqlSubscribeConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { Activity, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { GraphqlSubscribeConfig as GraphqlSubscribeConfigType, GraphqlConnectConfig as GraphqlConnectConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { isGraphqlConnectConfig } from '@/types/node';
import { buildOperationFieldTrees } from '@/lib/graphqlSchema';
import type { FieldNode } from '@/lib/graphqlSchema';
import { GraphqlFieldTree } from '@/components/ui/GraphqlFieldTree';

interface GraphqlSubscribeConfigProps {
  data: GraphqlSubscribeConfigType;
  onSave: (data: GraphqlSubscribeConfigType) => void;
}

function variablesToString(variables?: Record<string, unknown>): string {
  if (!variables) return '';
  try {
    return JSON.stringify(variables, null, 2);
  } catch {
    return '';
  }
}

function variablesFromString(json: string | undefined): Record<string, unknown> | undefined {
  if (!json || !json.trim()) return undefined;
  try {
    return JSON.parse(json) as Record<string, unknown>;
  } catch {
    return undefined;
  }
}

function toFormData(data: GraphqlSubscribeConfigType): GraphqlSubscribeConfigFormData {
  return {
    type: 'graphql_subscribe',
    connection_id: data.connection_id || '',
    subscription: data.subscription || '',
    variables: variablesToString(data.variables),
    operation_name: data.operation_name || '',
    max_events: data.max_events ?? 0,
    timeout_ms: data.timeout_ms ?? 30000,
  };
}

function fromFormData(formData: GraphqlSubscribeConfigFormData): GraphqlSubscribeConfigType {
  return {
    type: 'graphql_subscribe',
    connection_id: formData.connection_id,
    subscription: formData.subscription,
    variables: variablesFromString(formData.variables),
    operation_name: formData.operation_name || undefined,
    max_events: formData.max_events || undefined,
    timeout_ms: formData.timeout_ms || undefined,
  };
}

export function GraphqlSubscribeConfig({ data, onSave }: GraphqlSubscribeConfigProps) {
  const {
    register,
    handleSubmit,
    reset,
    getValues,
    setValue,
    watch,
    formState: { errors, isDirty },
  } = useForm<GraphqlSubscribeConfigFormData>({
    resolver: zodResolver(graphqlSubscribeConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

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

  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  const flow = useFlowStore((state) => state.flow);

  const availableConnections = useMemo(() => {
    if (!flow) return [];
    const connections: Array<{ uuid: string; name: string }> = [];
    flow.nodes.forEach(node => {
      if (isGraphqlConnectConfig(node.data)) {
        connections.push({ uuid: node.uuid, name: node.name });
      }
    });
    return connections;
  }, [flow]);

  // 获取选中的 connect 节点中的 discovered_schema，构建订阅字段树
  const selectedSchema = useMemo<{
    subscriptionFields: FieldNode[];
    hasSchema: boolean;
  }>(() => {
    if (!flow || !watch('connection_id')) {
      return { subscriptionFields: [], hasSchema: false };
    }
    const connectNode = flow.nodes.find(
      n => n.uuid === watch('connection_id') && isGraphqlConnectConfig(n.data)
    );
    if (!connectNode) return { subscriptionFields: [], hasSchema: false };
    const config = connectNode.data as GraphqlConnectConfigType;
    if (!config.discovered_schema) return { subscriptionFields: [], hasSchema: false };

    const subscription = buildOperationFieldTrees(config.discovered_schema, 'subscription');
    return {
      subscriptionFields: subscription.rootFields,
      hasSchema: true,
    };
  }, [flow, watch('connection_id')]);

  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 GraphQL Subscribe 配置失败:', error);
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

  const [variablesError, setVariablesError] = useState<string | null>(null);

  const handleVariablesBlur = useCallback(() => {
    const val = getValues('variables');
    if (val && val.trim()) {
      try {
        JSON.parse(val);
        setVariablesError(null);
      } catch {
        setVariablesError('JSON 格式无效');
        return;
      }
    } else {
      setVariablesError(null);
    }
    safeSetTimeout(() => {
      if (!isActuallyDirtyRef.current) return;
      saveData();
    }, 100);
  }, [getValues, safeSetTimeout, saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-indigo-200/50 bg-gradient-to-br from-indigo-50/80 to-violet-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-indigo-900 flex items-center gap-2">
            <Activity className="h-4 w-4" />
            {getLL().panels.graphqlSubscribe.title()}
          </h4>
          <p className="text-sm leading-relaxed text-indigo-700/90 whitespace-normal break-words">
            {getLL().panels.graphqlSubscribe.description()}
          </p>
        </div>
      </div>

      {/* 连接选择 */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          {getLL().panels.connection()} <span className="text-indigo-500">*</span>
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
          <p className="text-xs text-indigo-500">{errors.connection_id.message}</p>
        )}
        {availableConnections.length === 0 && (
          <p className="text-xs text-muted-foreground">
            {getLL().panels.graphqlSubscribe.noConnectNode()}
          </p>
        )}
      </div>

      {/* Schema 字段浏览器 */}
      {selectedSchema.hasSchema && selectedSchema.subscriptionFields.length > 0 && (
        <div className="space-y-3">
          <Label className="text-sm font-medium text-foreground">
            Schema 字段浏览器
          </Label>
          <p className="text-xs text-muted-foreground">
            勾选需要的订阅字段，点击"生成订阅"自动填充订阅编辑器
          </p>
          <GraphqlFieldTree
            title="Subscription"
            fields={selectedSchema.subscriptionFields}
            onGenerate={(q) => {
              if (!q) return;
              const vals = getValues();
              const updated = { ...vals, subscription: q };
              reset(updated as GraphqlSubscribeConfigFormData);
              onSave(fromFormData(updated));
            }}
          />
        </div>
      )}

      {/* 订阅编辑器 */}
      <div className="space-y-2">
        <Label htmlFor="subscription" className="text-sm font-medium text-foreground">
          {getLL().panels.graphqlSubscribe.subscription()} <span className="text-indigo-500">*</span>
        </Label>
        <Textarea
          id="subscription"
          {...register('subscription')}
          placeholder={`subscription OnMessage($roomId: ID!) {\n  messageAdded(roomId: $roomId) {\n    id\n    text\n    sender\n  }\n}`}
          className="min-h-[160px] font-mono text-sm resize-y"
          style={{ tabSize: 2 }}
        />
        {errors.subscription && (
          <p className="text-xs text-indigo-500">{errors.subscription.message}</p>
        )}
      </div>

      {/* 变量编辑器 */}
      <div className="space-y-2">
        <Label htmlFor="variables" className="text-sm font-medium text-foreground">
          {getLL().panels.graphqlQuery.variables()}
        </Label>
        <Textarea
          id="variables"
          {...register('variables')}
          onBlur={handleVariablesBlur}
          placeholder={'{\n  "roomId": "room-123"\n}'}
          className="min-h-[80px] font-mono text-sm resize-y"
          style={{ tabSize: 2 }}
        />
        {variablesError && (
          <p className="text-xs text-indigo-500">{variablesError}</p>
        )}
      </div>

      {/* 操作名称 */}
      <div className="space-y-2">
        <Label htmlFor="operation_name" className="text-sm font-medium text-foreground">
          操作名称
        </Label>
        <Input
          id="operation_name"
          {...register('operation_name')}
          placeholder="Optional, e.g. OnMessage"
          className="font-mono text-sm"
        />
      </div>

      {/* 最大事件数 */}
      <div className="space-y-2">
        <Label htmlFor="max_events" className="text-sm font-medium text-foreground">
          最大事件数
        </Label>
        <Input
          id="max_events"
          type="number"
          {...register('max_events', { valueAsNumber: true })}
          placeholder="0 = Unlimited"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          达到最大事件数后自动停止订阅，0 表示不限制
        </p>
      </div>

      {/* 超时 */}
      <div className="space-y-2">
        <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground">
          超时（毫秒）
        </Label>
        <Input
          id="timeout_ms"
          type="number"
          {...register('timeout_ms', { valueAsNumber: true })}
          placeholder="30000"
          className="font-mono text-sm"
        />
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-indigo-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">GraphQL 订阅说明：</p>
          <p>订阅需要连接配置了 WebSocket 端点</p>
          <p>实时接收服务端推送的订阅事件</p>
          <p>设置最大事件数或超时时间可自动结束订阅</p>
        </div>
      </div>
    </form>
  );
}
