/**
 * GraphQL Introspect 节点配置组件
 * 支持连接 ID、超时
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import {
  graphqlIntrospectConfigSchema,
  type GraphqlIntrospectConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Search, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { GraphqlIntrospectConfig as GraphqlIntrospectConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { isGraphqlConnectConfig } from '@/types/node';

interface GraphqlIntrospectConfigProps {
  data: GraphqlIntrospectConfigType;
  onSave: (data: GraphqlIntrospectConfigType) => void;
}

function toFormData(data: GraphqlIntrospectConfigType): GraphqlIntrospectConfigFormData {
  return {
    type: 'graphql:introspect',
    connection_id: data.connection_id || '',
    timeout_ms: data.timeout_ms ?? 30000,
  };
}

function fromFormData(formData: GraphqlIntrospectConfigFormData): GraphqlIntrospectConfigType {
  return {
    type: 'graphql:introspect',
    connection_id: formData.connection_id,
    timeout_ms: formData.timeout_ms || undefined,
  };
}

export function GraphqlIntrospectConfig({ data, onSave }: GraphqlIntrospectConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<GraphqlIntrospectConfigFormData>({
    resolver: zodResolver(graphqlIntrospectConfigSchema),
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
      if (isGraphqlConnectConfig(node.data)) {
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
        console.error('保存 GraphQL Introspect 配置失败:', error);
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

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-indigo-200/50 bg-gradient-to-br from-indigo-50/80 to-violet-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-indigo-900 flex items-center gap-2">
            <Search className="h-4 w-4" />
            {getLL().panels.graphqlIntrospect.title()}
          </h4>
          <p className="text-sm leading-relaxed text-indigo-700/90 whitespace-normal break-words">
            {getLL().panels.graphqlIntrospect.description()}
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
            {getLL().panels.graphqlIntrospect.noConnectNode()}
          </p>
        )}
      </div>

      {/* 超时 */}
      <div className="space-y-2">
        <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground">
          {getLL().panels.timeout()}
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
          <p className="font-medium text-foreground">{getLL().panels.graphqlIntrospect.help_title()}</p>
          <p>{getLL().panels.graphqlIntrospect.help_line1()}</p>
          <p>{getLL().panels.graphqlIntrospect.help_line2()}</p>
          <p>{getLL().panels.graphqlIntrospect.help_line3()}</p>
        </div>
      </div>
    </form>
  );
}
