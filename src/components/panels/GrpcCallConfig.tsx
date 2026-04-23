/**
 * gRPC Call 节点配置组件
 * 支持连接选择、服务/方法选择、请求 JSON、Metadata、超时
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import {
  grpcCallConfigSchema,
  type GrpcCallConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Plus, Trash2, Radio, Info } from 'lucide-react';
import type { GrpcCallConfig as GrpcCallConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { isGrpcConnectConfig } from '@/types/node';

interface GrpcCallConfigProps {
  /** 当前节点数据 */
  data: GrpcCallConfigType;
  /** 保存回调 */
  onSave: (data: GrpcCallConfigType) => void;
}

/**
 * 将 GrpcCallConfigType 转换为表单数据
 */
function toFormData(data: GrpcCallConfigType): GrpcCallConfigFormData {
  return {
    type: 'grpc_call',
    connection_id: data.connection_id || '',
    service_name: data.service_name || '',
    method_name: data.method_name || '',
    request_json: data.request_json || '{}',
    timeout_ms: data.timeout_ms ?? 30000,
    metadata: data.metadata || [],
  };
}

/**
 * 将表单数据转换为 GrpcCallConfigType
 */
function fromFormData(formData: GrpcCallConfigFormData): GrpcCallConfigType {
  const metadata = formData.metadata?.filter(m => m.key.trim());

  return {
    type: 'grpc_call',
    connection_id: formData.connection_id,
    service_name: formData.service_name,
    method_name: formData.method_name,
    request_json: formData.request_json,
    timeout_ms: formData.timeout_ms,
    metadata: metadata && metadata.length > 0 ? metadata : undefined,
  };
}

export function GrpcCallConfig({ data, onSave }: GrpcCallConfigProps) {
  const {
    register,
    handleSubmit,
    control,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<GrpcCallConfigFormData>({
    resolver: zodResolver(grpcCallConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const metadataFields = useFieldArray({
    control,
    name: 'metadata' as never,
  });

  // 获取 flow 并提取 grpc_connect 节点
  const flow = useFlowStore((state) => state.flow);

  // 提取可用的 gRPC 连接（从 grpc_connect 节点）
  const availableConnections = useMemo(() => {
    if (!flow) return [];
    const connections: Array<{ uuid: string; name: string; url?: string }> = [];
    flow.nodes.forEach(node => {
      if (isGrpcConnectConfig(node.data)) {
        connections.push({
          uuid: node.uuid,
          name: node.name,
          url: node.data.url,
        });
      }
    });
    return connections;
  }, [flow]);

  // 追踪待执行的 setTimeout，组件卸载时清除
  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

  // 追踪表单实际修改状态
  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  // 组件卸载时同步保存
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

  // 当外部数据变化时，重置表单
  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  /**
   * 保存表单数据
   */
  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 gRPC Call 配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  /**
   * 处理失焦事件
   */
  const handleBlur = useCallback(() => {
    safeSetTimeout(() => {
      if (!isActuallyDirtyRef.current) return;
      saveData();
    }, 100);
  }, [safeSetTimeout, saveData]);

  // 监听 Ctrl+S 刷新事件
  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  /**
   * 添加 Metadata 条目
   */
  const handleAddMetadata = useCallback(() => {
    metadataFields.append({ key: '', value: '' });
  }, [metadataFields]);

  /**
   * 删除 Metadata 条目
   */
  const handleRemoveMetadata = useCallback((index: number) => {
    metadataFields.remove(index);
    safeSetTimeout(saveData, 100);
  }, [metadataFields, saveData, safeSetTimeout]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-violet-200/50 bg-gradient-to-br from-violet-50/80 to-purple-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-violet-900 flex items-center gap-2">
            <Radio className="h-4 w-4" />
            gRPC 调用节点
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90 whitespace-normal break-words">
            通过已建立的 gRPC 连接调用指定服务方法，发送请求并接收响应。
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
              <SelectValue placeholder="选择 gRPC 连接" />
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
          <p className="text-xs text-rose-500">{errors.connection_id.message}</p>
        )}
        {availableConnections.length === 0 && (
          <p className="text-xs text-muted-foreground">
            未检测到 gRPC Connect 节点，请手动输入连接 ID
          </p>
        )}
      </div>

      {/* 服务选择 */}
      <div className="space-y-2">
        <Label htmlFor="service_name" className="text-sm font-medium text-foreground">
          服务名称 <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="service_name"
          {...register('service_name')}
          placeholder="package.ServiceName"
          className="font-mono text-sm"
        />
        {errors.service_name && (
          <p className="text-xs text-rose-500">{errors.service_name.message}</p>
        )}
        <p className="text-xs text-muted-foreground">
          完整的服务名称（含包名），如 <code className="rounded bg-muted px-1">mypackage.MyService</code>
        </p>
      </div>

      {/* 方法选择 */}
      <div className="space-y-2">
        <Label htmlFor="method_name" className="text-sm font-medium text-foreground">
          方法名称 <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="method_name"
          {...register('method_name')}
          placeholder="MethodName"
          className="font-mono text-sm"
        />
        {errors.method_name && (
          <p className="text-xs text-rose-500">{errors.method_name.message}</p>
        )}
      </div>

      {/* 请求 JSON */}
      <div className="space-y-2">
        <Label htmlFor="request_json" className="text-sm font-medium text-foreground">
          请求 JSON
        </Label>
        <Textarea
          id="request_json"
          {...register('request_json')}
          placeholder='{"field": "value"}'
          className="min-h-[120px] resize-y font-mono text-sm"
          rows={5}
        />
        {errors.request_json && (
          <p className="text-xs text-rose-500">{errors.request_json.message}</p>
        )}
        <p className="text-xs text-muted-foreground">
          请求消息的 JSON 格式，将自动序列化发送
        </p>
      </div>

      {/* Metadata */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">Metadata（可选）</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddMetadata}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            添加
          </Button>
        </div>
        <div className="space-y-2">
          {metadataFields.fields.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">暂无调用级 Metadata</p>
              <p className="mt-1 text-xs text-muted-foreground/60">
                会与连接的默认 Metadata 合并发送
              </p>
            </div>
          ) : (
            metadataFields.fields.map((field, index) => (
              <div key={field.id} className="flex items-center gap-2 p-2 rounded-md border bg-muted/30">
                <Input
                  {...register(`metadata.${index}.key`)}
                  placeholder="Key"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Input
                  {...register(`metadata.${index}.value`)}
                  placeholder="Value"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  onClick={() => handleRemoveMetadata(index)}
                  className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            ))
          )}
        </div>
      </div>

      {/* 调用超时 */}
      <div className="space-y-2">
        <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground">
          调用超时
        </Label>
        <Input
          id="timeout_ms"
          type="number"
          min={1000}
          max={300000}
          {...register('timeout_ms', { valueAsNumber: true })}
        />
        <p className="text-xs text-muted-foreground">毫秒 (ms)，范围 1000-300000</p>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">调用说明：</p>
          <p>选择已建立的 gRPC 连接，指定服务和方法名称</p>
          <p>请求 JSON 会根据方法的 Input Type 生成骨架结构</p>
          <p>调用级 Metadata 会覆盖连接默认的同名条目</p>
        </div>
      </div>
    </form>
  );
}
