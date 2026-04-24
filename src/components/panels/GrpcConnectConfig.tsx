/**
 * gRPC Connect 节点配置组件
 * 支持服务器地址、TLS、Proto 文件管理、默认 Metadata、连接超时
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef, useMemo } from 'react';
import {
  grpcConnectConfigSchema,
  type GrpcConnectConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { Plus, Trash2, Cable, FileText, Info, Search, Loader2, ChevronDown } from 'lucide-react';
import type { GrpcConnectConfig as GrpcConnectConfigType, GrpcServiceInfo } from '@/types';
import { invoke } from '@tauri-apps/api/core';
import { cn } from '@/lib/utils';

interface GrpcConnectConfigProps {
  /** 当前节点数据 */
  data: GrpcConnectConfigType;
  /** 保存回调 */
  onSave: (data: GrpcConnectConfigType) => void;
}

/**
 * 将 GrpcConnectConfigType 转换为表单数据
 */
function toFormData(data: GrpcConnectConfigType): GrpcConnectConfigFormData {
  return {
    type: 'grpc_connect',
    url: data.url || '',
    use_tls: data.use_tls ?? false,
    tls_skip_verify: data.tls_skip_verify ?? false,
    enable_reflection: data.enable_reflection ?? false,
    proto_files: data.proto_files || [],
    default_metadata: data.default_metadata || [],
    connect_timeout_ms: data.connect_timeout_ms ?? 30000,
    discovered_services: data.discovered_services,
  };
}

/**
 * 将表单数据转换为 GrpcConnectConfigType
 */
function fromFormData(formData: GrpcConnectConfigFormData): GrpcConnectConfigType {
  const protoFiles = formData.proto_files?.filter(f => f.path.trim() && f.content.trim());
  const metadata = formData.default_metadata?.filter(m => m.key.trim());

  return {
    type: 'grpc_connect',
    url: formData.url,
    use_tls: formData.use_tls,
    tls_skip_verify: formData.tls_skip_verify,
    enable_reflection: formData.enable_reflection,
    proto_files: protoFiles && protoFiles.length > 0 ? protoFiles : undefined,
    default_metadata: metadata && metadata.length > 0 ? metadata : undefined,
    connect_timeout_ms: formData.connect_timeout_ms,
    discovered_services: formData.discovered_services,
  };
}

export function GrpcConnectConfig({ data, onSave }: GrpcConnectConfigProps) {
  const {
    register,
    handleSubmit,
    control,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<GrpcConnectConfigFormData>({
    resolver: zodResolver(grpcConnectConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const protoFiles = useFieldArray({
    control,
    name: 'proto_files' as never,
  });

  const metadataFields = useFieldArray({
    control,
    name: 'default_metadata' as never,
  });

  // 监听表单值变化
  const useTls = watch('use_tls');
  const enableReflection = watch('enable_reflection');

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
        console.error('保存 gRPC Connect 配置失败:', error);
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
   * 添加 Proto 文件
   */
  const handleAddProtoFile = useCallback(() => {
    protoFiles.append({ path: '', content: '' });
  }, [protoFiles]);

  /**
   * 删除 Proto 文件
   */
  const handleRemoveProtoFile = useCallback((index: number) => {
    protoFiles.remove(index);
    safeSetTimeout(saveData, 100);
  }, [protoFiles, saveData, safeSetTimeout]);

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

  // 服务发现状态
  const [isDiscovering, setIsDiscovering] = useState(false);
  const [discoveryResult, setDiscoveryResult] = useState<{
    success: boolean;
    services: GrpcServiceInfo[];
    error: string | null;
  } | null>(null);

  // 可折叠服务树的展开状态
  const [expandedServices, setExpandedServices] = useState<Set<string>>(new Set());

  // 合并发现结果：优先使用最新的发现结果，否则使用已缓存的服务
  const displayServices = useMemo((): GrpcServiceInfo[] => {
    if (discoveryResult?.success && discoveryResult.services.length > 0) {
      return discoveryResult.services;
    }
    return data.discovered_services || [];
  }, [discoveryResult, data.discovered_services]);

  // 监听 URL 变化，清除已发现的服务
  const currentUrl = watch('url');
  const prevUrlRef = useRef(data.url);
  useEffect(() => {
    const trimmedUrl = currentUrl?.trim();
    const prevTrimmed = prevUrlRef.current?.trim();
    if (trimmedUrl !== prevTrimmed && prevTrimmed !== undefined) {
      // URL 发生变化，清除已发现的服务
      const formData = getValues();
      const config = fromFormData(formData);
      config.discovered_services = undefined;
      onSave(config);
      setDiscoveryResult(null);
      prevUrlRef.current = trimmedUrl;
    } else {
      prevUrlRef.current = trimmedUrl;
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentUrl]);

  /**
   * 测试连接 & 发现服务
   */
  const handleDiscoverServices = useCallback(async () => {
    const formData = getValues();
    const url = formData.url?.trim();
    if (!url) return;

    setIsDiscovering(true);
    setDiscoveryResult(null);

    try {
      console.log('[gRPC] 开始发现服务:', {
        url,
        enable_reflection: formData.enable_reflection,
        use_tls: formData.use_tls,
      });

      const result = await invoke<{
        success: boolean;
        services: GrpcServiceInfo[];
        error: string | null;
      }>('grpc_discover_services', {
        input: {
          url,
          use_tls: formData.use_tls ?? false,
          tls_skip_verify: formData.tls_skip_verify ?? false,
          enable_reflection: formData.enable_reflection ?? false,
          proto_files: formData.proto_files
            ?.filter(f => f.path.trim() && f.content.trim())
            .map(f => [f.path, f.content] as [string, string]),
          connect_timeout_ms: formData.connect_timeout_ms ?? 30000,
        },
      });

      console.log('[gRPC] 发现服务结果:', result);
      setDiscoveryResult(result);

      if (result.success && result.services.length > 0) {
        // 将发现的服务缓存到节点数据中
        const config = fromFormData(formData);
        config.discovered_services = result.services;
        onSave(config);
      }
    } catch (error) {
      console.error('[gRPC] 发现服务失败:', error);
      setDiscoveryResult({
        success: false,
        services: [],
        error: typeof error === 'string' ? error : (error instanceof Error ? error.message : String(error)),
      });
    } finally {
      setIsDiscovering(false);
    }
  }, [getValues, onSave]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-violet-200/50 bg-gradient-to-br from-violet-50/80 to-purple-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-violet-900 flex items-center gap-2">
            <Cable className="h-4 w-4" />
            gRPC 连接节点
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90 whitespace-normal break-words">
            建立 gRPC 连接，支持 TLS、Proto 文件管理或 Server Reflection。
          </p>
        </div>
      </div>

      {/* 服务器地址 */}
      <div className="space-y-2">
        <Label htmlFor="url" className="text-sm font-medium text-foreground">
          gRPC 服务器地址 <span className="text-rose-500">*</span>
        </Label>
        <div className="relative">
          <Cable className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <Input
            id="url"
            {...register('url')}
            placeholder="http://localhost:50051"
            className="pl-9 font-mono text-sm"
          />
        </div>
        {errors.url && (
          <p className="text-xs text-rose-500">{errors.url.message}</p>
        )}
        <p className="text-xs text-muted-foreground">
          支持 http://（明文）和 https://（TLS）格式
        </p>
      </div>

      {/* TLS 配置 */}
      <div className="space-y-3 rounded-md border bg-muted/30 p-3">
        <div className="flex items-center space-x-2">
          <Switch
            id="use_tls"
            checked={useTls || false}
            onCheckedChange={(checked) => {
              setValue('use_tls', checked, { shouldDirty: true });
              if (!checked) {
                setValue('tls_skip_verify', false, { shouldDirty: true });
              }
              saveData();
            }}
          />
          <Label htmlFor="use_tls" className="text-sm font-medium text-foreground cursor-pointer">
            启用 TLS
          </Label>
        </div>

        {useTls && (
          <div className="flex items-center space-x-2 pl-2">
            <Switch
              id="tls_skip_verify"
              checked={watch('tls_skip_verify') || false}
              onCheckedChange={(checked) => {
                setValue('tls_skip_verify', checked, { shouldDirty: true });
                saveData();
              }}
            />
            <Label htmlFor="tls_skip_verify" className="text-sm text-muted-foreground cursor-pointer">
              跳过证书验证
            </Label>
          </div>
        )}
      </div>

      {/* Proto 来源 */}
      <div className="space-y-3">
        <Label className="text-sm font-medium text-foreground">Proto 来源</Label>
        <div className="space-y-2">
          <div className="flex items-center space-x-2">
            <Switch
              id="enable_reflection"
              checked={enableReflection || false}
              onCheckedChange={(checked) => {
                setValue('enable_reflection', checked, { shouldDirty: true });
                saveData();
              }}
            />
            <Label htmlFor="enable_reflection" className="text-sm text-foreground cursor-pointer">
              使用 Server Reflection
            </Label>
          </div>
          <p className="text-xs text-muted-foreground pl-2">
            {enableReflection
              ? '从 gRPC 服务器自动获取服务和方法定义'
              : '手动上传 .proto 文件定义服务和方法'}
          </p>
        </div>

        {/* 手动 Proto 文件管理（仅非 Reflection 模式显示） */}
        {!enableReflection && (
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-xs text-muted-foreground">Proto 文件列表</Label>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={handleAddProtoFile}
                className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
              >
                <Plus className="h-3 w-3" />
                添加文件
              </Button>
            </div>
            <div className="space-y-2">
              {protoFiles.fields.length === 0 ? (
                <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
                  <p className="text-xs text-muted-foreground">暂无 Proto 文件</p>
                  <p className="mt-1 text-xs text-muted-foreground/60">
                    点击「添加文件」上传 .proto 文件
                  </p>
                </div>
              ) : (
                protoFiles.fields.map((field, index) => (
                  <div key={field.id} className="space-y-2 p-3 rounded-md border bg-muted/30">
                    <div className="flex items-center gap-2">
                      <FileText className="h-4 w-4 text-muted-foreground shrink-0" />
                      <Input
                        {...register(`proto_files.${index}.path`)}
                        placeholder="文件名（如 example.proto）"
                        className="h-8 flex-1 font-mono text-xs"
                      />
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon"
                        onClick={() => handleRemoveProtoFile(index)}
                        className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </Button>
                    </div>
                    <Textarea
                      {...register(`proto_files.${index}.content`)}
                      placeholder="syntax = &quot;proto3&quot;;&#10;&#10;service MyService {&#10;  rpc MyMethod (Request) returns (Response);&#10;}"
                      className="min-h-[100px] resize-y font-mono text-xs"
                      rows={5}
                    />
                  </div>
                ))
              )}
            </div>
          </div>
        )}
      </div>

      {/* 测试连接 & 发现服务 */}
      <div className="space-y-2">
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="w-full gap-2"
          disabled={!watch('url')?.trim() || isDiscovering}
          onClick={handleDiscoverServices}
        >
          {isDiscovering ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Search className="h-4 w-4" />
          )}
          {isDiscovering ? '正在发现服务...' : '测试连接 & 发现服务'}
        </Button>

        {/* 发现结果摘要 */}
        {discoveryResult && (
          <div className={`rounded-md border px-3 py-2 text-xs ${
            discoveryResult.success
              ? 'border-emerald-200 dark:border-emerald-800/60 bg-emerald-50 dark:bg-emerald-950/30 text-emerald-700 dark:text-emerald-400'
              : 'border-rose-200 dark:border-rose-800/60 bg-rose-50 dark:bg-rose-950/30 text-rose-600 dark:text-rose-400'
          }`}>
            {discoveryResult.success ? (
              <p>发现 {discoveryResult.services.length} 个服务</p>
            ) : (
              <p className="text-rose-500 dark:text-rose-400">{discoveryResult.error}</p>
            )}
          </div>
        )}

        {/* 已发现的服务树 */}
        {displayServices.length > 0 && (
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-xs text-muted-foreground">已发现的服务</Label>
              <span className="inline-flex items-center rounded-full bg-violet-100 dark:bg-violet-900/30 px-2 py-0.5 text-xs font-medium text-violet-700 dark:text-violet-300">
                {displayServices.length} 个服务
              </span>
            </div>
            <div className="space-y-2 max-h-[400px] overflow-y-auto">
              {displayServices.map((service) => {
                const isExpanded = expandedServices.has(service.service_name);
                return (
                  <div key={service.service_name} className="border rounded-lg overflow-hidden">
                    <button
                      type="button"
                      onClick={() => {
                        setExpandedServices(prev => {
                          const next = new Set(prev);
                          if (next.has(service.service_name)) next.delete(service.service_name);
                          else next.add(service.service_name);
                          return next;
                        });
                      }}
                      className="w-full flex items-center justify-between px-3 py-2 text-left hover:bg-muted/50 transition-colors"
                    >
                      <span className="font-mono text-sm font-medium text-foreground truncate">
                        {service.service_name}
                      </span>
                      <div className="flex items-center gap-2 shrink-0 ml-2">
                        <span className="text-xs text-muted-foreground">
                          {service.methods.length} 个方法
                        </span>
                        <ChevronDown className={cn(
                          "h-3.5 w-3.5 text-muted-foreground transition-transform",
                          isExpanded && "rotate-180"
                        )} />
                      </div>
                    </button>
                    {isExpanded && service.methods.length > 0 && (
                      <div className="border-t bg-muted/20 px-3 py-2 space-y-1">
                        {service.methods.map((method) => (
                          <div key={method.method_name} className="flex items-center gap-2 text-xs rounded px-2 py-1 bg-background/50">
                            <code className="text-foreground font-mono">{method.method_name}</code>
                            <span className="text-muted-foreground">→</span>
                            <code className="text-muted-foreground font-mono text-[11px]">{method.output_type}</code>
                            {method.is_server_streaming && method.is_client_streaming ? (
                              <span className="inline-flex items-center rounded-full bg-purple-100 dark:bg-purple-900/30 px-1.5 py-0.5 text-[10px] font-medium text-purple-700 dark:text-purple-300">
                                双向流式
                              </span>
                            ) : method.is_server_streaming ? (
                              <span className="inline-flex items-center rounded-full bg-amber-100 dark:bg-amber-900/30 px-1.5 py-0.5 text-[10px] font-medium text-amber-700 dark:text-amber-300">
                                服务端流式
                              </span>
                            ) : method.is_client_streaming ? (
                              <span className="inline-flex items-center rounded-full bg-blue-100 dark:bg-blue-900/30 px-1.5 py-0.5 text-[10px] font-medium text-blue-700 dark:text-blue-300">
                                客户端流式
                              </span>
                            ) : (
                              <span className="inline-flex items-center rounded-full bg-green-100 dark:bg-green-900/30 px-1.5 py-0.5 text-[10px] font-medium text-green-700 dark:text-green-300">
                                Unary
                              </span>
                            )}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>

      {/* 默认 Metadata */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">默认 Metadata（可选）</Label>
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
              <p className="text-xs text-muted-foreground">暂无默认 Metadata</p>
            </div>
          ) : (
            metadataFields.fields.map((field, index) => (
              <div key={field.id} className="flex items-center gap-2 p-2 rounded-md border bg-muted/30">
                <Input
                  {...register(`default_metadata.${index}.key`)}
                  placeholder="Key"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Input
                  {...register(`default_metadata.${index}.value`)}
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

      {/* 连接超时 */}
      <div className="space-y-2">
        <Label htmlFor="connect_timeout_ms" className="text-sm font-medium text-foreground">
          连接超时
        </Label>
        <Input
          id="connect_timeout_ms"
          type="number"
          min={1000}
          max={300000}
          {...register('connect_timeout_ms', { valueAsNumber: true })}
        />
        <p className="text-xs text-muted-foreground">毫秒 (ms)，范围 1000-300000</p>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">Proto 来源说明：</p>
          <p><code className="rounded bg-muted px-1">Server Reflection</code> 从服务器自动获取服务定义，无需手动上传</p>
          <p><code className="rounded bg-muted px-1">手动上传</code> 适合不支持 Reflection 的服务器或离线开发</p>
          <p><code className="rounded bg-muted px-1">测试连接</code> 验证连接并发现可用服务，结果自动缓存供 gRPC Call 节点使用</p>
        </div>
      </div>
    </form>
  );
}
