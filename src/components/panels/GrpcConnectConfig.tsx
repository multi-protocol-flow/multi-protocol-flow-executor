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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Plus, Trash2, Cable, FileText, Info, Search, Loader2, ChevronDown, AlertTriangle, FolderOpen, Upload, FileUp, Activity, RefreshCw } from 'lucide-react';
import type { GrpcConnectConfig as GrpcConnectConfigType, GrpcServiceInfo } from '@/types';
import { discoverGrpcServices, scanProtoDirectory, parseDescriptorSet, readProtoFiles } from '@/services/tauri';
import { grpcChannelzInfo, type ChannelzInfo } from '@/services/tauri/grpc';
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
    tls_ca_cert: data.tls_ca_cert,
    tls_client_cert: data.tls_client_cert,
    tls_client_key: data.tls_client_key,
    tls_server_name_override: data.tls_server_name_override,
    reflection_metadata: data.reflection_metadata || [],
    compression_encoding: data.compression_encoding,
    keepalive_time_ms: data.keepalive_time_ms,
    keepalive_timeout_ms: data.keepalive_timeout_ms,
    keepalive_permit_without_streams: data.keepalive_permit_without_streams,
    endpoints: data.endpoints || [],
  };
}

/**
 * 将表单数据转换为 GrpcConnectConfigType
 */
function fromFormData(formData: GrpcConnectConfigFormData): GrpcConnectConfigType {
  const protoFiles = formData.proto_files?.filter(f => f.path.trim() && f.content.trim());
  const metadata = formData.default_metadata?.filter(m => m.key.trim());
  const reflectionMetadata = formData.reflection_metadata?.filter(m => m.key.trim());
  const endpoints = formData.endpoints?.filter(ep => ep.trim());

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
    tls_ca_cert: formData.tls_ca_cert?.trim() || undefined,
    tls_client_cert: formData.tls_client_cert?.trim() || undefined,
    tls_client_key: formData.tls_client_key?.trim() || undefined,
    tls_server_name_override: formData.tls_server_name_override?.trim() || undefined,
    reflection_metadata: reflectionMetadata && reflectionMetadata.length > 0 ? reflectionMetadata : undefined,
    compression_encoding: formData.compression_encoding || undefined,
    keepalive_time_ms: formData.keepalive_time_ms || undefined,
    keepalive_timeout_ms: formData.keepalive_timeout_ms || undefined,
    keepalive_permit_without_streams: formData.keepalive_permit_without_streams || undefined,
    endpoints: endpoints && endpoints.length > 0 ? endpoints : undefined,
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
    shouldFocusError: false,
  });

  const protoFiles = useFieldArray({
    control,
    name: 'proto_files' as never,
  });

  const metadataFields = useFieldArray({
    control,
    name: 'default_metadata' as never,
  });

  const reflectionMetadataFields = useFieldArray({
    control,
    name: 'reflection_metadata' as never,
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
      } catch {
        // 保存失败时静默处理
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

  /**
   * 添加 Reflection Metadata 条目
   */
  const handleAddReflectionMetadata = useCallback(() => {
    reflectionMetadataFields.append({ key: '', value: '' });
  }, [reflectionMetadataFields]);

  /**
   * 删除 Reflection Metadata 条目
   */
  const handleRemoveReflectionMetadata = useCallback((index: number) => {
    reflectionMetadataFields.remove(index);
    safeSetTimeout(saveData, 100);
  }, [reflectionMetadataFields, saveData, safeSetTimeout]);

  /**
   * 添加额外端点（负载均衡）
   */
  const handleAddEndpoint = useCallback(() => {
    const current = getValues('endpoints') ?? [];
    setValue('endpoints', [...current, ''], { shouldDirty: true });
    saveData();
  }, [getValues, setValue, saveData]);

  /**
   * 删除额外端点
   */
  const handleRemoveEndpoint = useCallback((index: number) => {
    const current = getValues('endpoints') ?? [];
    setValue('endpoints', current.filter((_: string, i: number) => i !== index), { shouldDirty: true });
    safeSetTimeout(saveData, 100);
  }, [getValues, setValue, saveData, safeSetTimeout]);

  // 服务发现状态
  const [isDiscovering, setIsDiscovering] = useState(false);
  const [discoveryResult, setDiscoveryResult] = useState<{
    success: boolean;
    services: GrpcServiceInfo[];
    error: string | null;
  } | null>(null);

  // Channelz 内省状态（EXPERIMENTAL）
  const [isChannelzLoading, setIsChannelzLoading] = useState(false);
  const [channelzResult, setChannelzResult] = useState<{
    success: boolean;
    info: ChannelzInfo | null;
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
      const result = await discoverGrpcServices({
        url,
        use_tls: formData.use_tls ?? false,
        tls_skip_verify: formData.tls_skip_verify ?? false,
        enable_reflection: formData.enable_reflection ?? false,
        proto_files: formData.proto_files
          ?.filter(f => f.path.trim() && f.content.trim())
          .map(f => [f.path, f.content] as [string, string]),
        connect_timeout_ms: formData.connect_timeout_ms ?? 30000,
        tls_ca_cert: formData.tls_ca_cert?.trim() || undefined,
        tls_client_cert: formData.tls_client_cert?.trim() || undefined,
        tls_client_key: formData.tls_client_key?.trim() || undefined,
        tls_server_name_override: formData.tls_server_name_override?.trim() || undefined,
        reflection_metadata: formData.reflection_metadata
          ?.filter(m => m.key.trim())
          .map(m => [m.key, m.value] as [string, string]),
      });

      setDiscoveryResult(result);

      if (result.success && result.services.length > 0) {
        // 将发现的服务缓存到节点数据中
        const config = fromFormData(formData);
        config.discovered_services = result.services;
        onSave(config);
      }
    } catch (error) {
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
          <>
            <div className="flex items-center space-x-2">
              <Switch
                id="tls_skip_verify"
                checked={watch('tls_skip_verify') || false}
                onCheckedChange={(checked) => {
                  setValue('tls_skip_verify', checked, { shouldDirty: true });
                  saveData();
                }}
              />
              <Label htmlFor="tls_skip_verify" className="text-sm font-medium text-foreground cursor-pointer">
                跳过证书验证
              </Label>
            </div>
            {watch('tls_skip_verify') && (
              <div className="flex items-start gap-2 rounded-md border border-amber-200 bg-amber-50 p-2.5 dark:border-amber-800/60 dark:bg-amber-950/30">
                <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-500" />
                <p className="text-xs leading-relaxed text-amber-700 dark:text-amber-400">
                  跳过证书验证会使连接面临中间人攻击风险，数据仍会加密传输，但无法确认服务端身份。仅建议在开发/测试环境使用。
                </p>
              </div>
            )}

            {/* TLS 高级配置 */}
            {!watch('tls_skip_verify') && (
              <div className="space-y-2 pl-2 border-l-2 border-violet-200 dark:border-violet-800/60">
                <p className="text-xs font-medium text-muted-foreground">TLS 高级配置</p>

                {/* CA 证书 */}
                <div className="space-y-1">
                  <Label className="text-xs text-muted-foreground">CA 证书</Label>
                  <div className="flex items-center gap-2">
                    <Input
                      {...register('tls_ca_cert')}
                      placeholder="CA 证书文件路径（PEM）"
                      className="h-8 flex-1 font-mono text-xs"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8 shrink-0 text-muted-foreground"
                      onClick={async () => {
                        try {
                          const { open } = await import('@tauri-apps/plugin-dialog');
                          const selected = await open({
                            multiple: false,
                            filters: [{ name: 'PEM 证书', extensions: ['pem', 'crt', 'cer', 'ca'] }],
                          });
                          if (selected) {
                            setValue('tls_ca_cert', selected, { shouldDirty: true });
                            saveData();
                          }
                        } catch {
                          // 非 Tauri 环境或用户取消
                        }
                      }}
                    >
                      <FolderOpen className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                </div>

                {/* 客户端证书 */}
                <div className="space-y-1">
                  <Label className="text-xs text-muted-foreground">客户端证书（mTLS）</Label>
                  <div className="flex items-center gap-2">
                    <Input
                      {...register('tls_client_cert')}
                      placeholder="客户端证书文件路径（PEM）"
                      className="h-8 flex-1 font-mono text-xs"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8 shrink-0 text-muted-foreground"
                      onClick={async () => {
                        try {
                          const { open } = await import('@tauri-apps/plugin-dialog');
                          const selected = await open({
                            multiple: false,
                            filters: [{ name: 'PEM 证书', extensions: ['pem', 'crt', 'cer'] }],
                          });
                          if (selected) {
                            setValue('tls_client_cert', selected, { shouldDirty: true });
                            saveData();
                          }
                        } catch {
                          // 非 Tauri 环境或用户取消
                        }
                      }}
                    >
                      <FolderOpen className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                </div>

                {/* 客户端私钥 */}
                <div className="space-y-1">
                  <Label className="text-xs text-muted-foreground">客户端私钥（mTLS）</Label>
                  <div className="flex items-center gap-2">
                    <Input
                      {...register('tls_client_key')}
                      placeholder="客户端私钥文件路径（PEM）"
                      className="h-8 flex-1 font-mono text-xs"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8 shrink-0 text-muted-foreground"
                      onClick={async () => {
                        try {
                          const { open } = await import('@tauri-apps/plugin-dialog');
                          const selected = await open({
                            multiple: false,
                            filters: [{ name: 'PEM 私钥', extensions: ['pem', 'key'] }],
                          });
                          if (selected) {
                            setValue('tls_client_key', selected, { shouldDirty: true });
                            saveData();
                          }
                        } catch {
                          // 非 Tauri 环境或用户取消
                        }
                      }}
                    >
                      <FolderOpen className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                </div>

                {/* SNI 覆盖 */}
                <div className="space-y-1">
                  <Label className="text-xs text-muted-foreground">SNI 覆盖</Label>
                  <Input
                    {...register('tls_server_name_override')}
                    placeholder="自定义 TLS 服务器名称（留空使用 URL host）"
                    className="h-8 font-mono text-xs"
                  />
                  <p className="text-[11px] text-muted-foreground/70">
                    当证书域名与实际连接地址不一致时使用
                  </p>
                </div>
              </div>
            )}
          </>
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

          {/* Reflection 认证 Metadata（仅 Reflection 模式显示） */}
          {enableReflection && (
            <div className="space-y-2 pl-2 border-l-2 border-violet-200 dark:border-violet-800/60">
              <div className="flex items-center justify-between">
                <Label className="text-xs text-muted-foreground">认证 Metadata（可选）</Label>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={handleAddReflectionMetadata}
                  className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
                >
                  <Plus className="h-3 w-3" />
                  添加
                </Button>
              </div>
              <p className="text-[11px] text-muted-foreground/70">
                用于需要认证的 gRPC 反射服务（与连接级 Metadata 独立）
              </p>
              <div className="space-y-2">
                {reflectionMetadataFields.fields.length === 0 ? (
                  <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-3 text-center">
                    <p className="text-xs text-muted-foreground">暂无认证 Metadata</p>
                  </div>
                ) : (
                  reflectionMetadataFields.fields.map((field, index) => (
                    <div key={field.id} className="flex items-center gap-2 p-2 rounded-md border bg-muted/30">
                      <Input
                        {...register(`reflection_metadata.${index}.key`)}
                        placeholder="Key"
                        className="h-8 flex-1 font-mono text-xs"
                      />
                      <Input
                        {...register(`reflection_metadata.${index}.value`)}
                        placeholder="Value"
                        className="h-8 flex-1 font-mono text-xs"
                      />
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon"
                        onClick={() => handleRemoveReflectionMetadata(index)}
                        className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </Button>
                    </div>
                  ))
                )}
              </div>
            </div>
          )}
        </div>

        {/* 手动 Proto 文件管理（仅非 Reflection 模式显示） */}
        {!enableReflection && (
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-xs text-muted-foreground">Proto 文件列表</Label>
              <div className="flex items-center gap-1">
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
            </div>

            {/* 文件选择器按钮组 */}
            <div className="flex flex-wrap items-center gap-2">
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="h-8 gap-1.5 text-xs"
                onClick={async () => {
                  try {
                    const { open } = await import('@tauri-apps/plugin-dialog');
                    const selected = await open({
                      multiple: true,
                      filters: [{ name: 'Proto 文件', extensions: ['proto'] }],
                    });
                    if (selected) {
                      const paths = Array.isArray(selected) ? selected : [selected];
                      const result = await readProtoFiles(paths);
                      if (result.success) {
                        for (const file of result.files) {
                          const existingFiles = getValues('proto_files') ?? [];
                          const existingIndex = existingFiles.findIndex(
                            (f: { path: string; content: string }) => f.path === file.path
                          );
                          if (existingIndex >= 0) {
                            protoFiles.update(existingIndex, { path: file.path, content: file.content });
                          } else {
                            protoFiles.append({ path: file.path, content: file.content });
                          }
                        }
                        safeSetTimeout(saveData, 100);
                      }
                    }
                  } catch {
                    // 非 Tauri 环境或用户取消
                  }
                }}
              >
                <FileUp className="h-3.5 w-3.5" />
                选择 .proto 文件
              </Button>
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="h-8 gap-1.5 text-xs"
                onClick={async () => {
                  try {
                    const { open } = await import('@tauri-apps/plugin-dialog');
                    const selected = await open({
                      directory: true,
                    });
                    if (selected && typeof selected === 'string') {
                      const result = await scanProtoDirectory(selected);
                      if (result.success && result.files.length > 0) {
                        for (const file of result.files) {
                          const existingFiles = getValues('proto_files') ?? [];
                          const existingIndex = existingFiles.findIndex(
                            (f: { path: string; content: string }) => f.path === file.path
                          );
                          if (existingIndex >= 0) {
                            protoFiles.update(existingIndex, { path: file.path, content: file.content });
                          } else {
                            protoFiles.append({ path: file.path, content: file.content });
                          }
                        }
                        safeSetTimeout(saveData, 100);
                      } else if (result.error) {
                          // 目录扫描失败，静默处理
                        }
                    }
                  } catch {
                    // 非 Tauri 环境或用户取消
                  }
                }}
              >
                <FolderOpen className="h-3.5 w-3.5" />
                选择目录
              </Button>
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="h-8 gap-1.5 text-xs"
                onClick={async () => {
                  try {
                    const { open } = await import('@tauri-apps/plugin-dialog');
                    const selected = await open({
                      multiple: false,
                      filters: [{ name: 'Descriptor Set', extensions: ['pb', 'fdset', 'desc'] }],
                    });
                    if (selected && typeof selected === 'string') {
                      const result = await parseDescriptorSet(selected);
                      if (result.success && result.services.length > 0) {
                        // 将发现的服务缓存到节点数据中
                        const formData = getValues();
                        const config = fromFormData(formData);
                        const existingServices = config.discovered_services ?? [];
                        // 合并服务（去重）
                        const mergedServices = [...existingServices];
                        for (const svc of result.services) {
                          if (!mergedServices.some(s => s.service_name === svc.service_name)) {
                            mergedServices.push(svc);
                          }
                        }
                        config.discovered_services = mergedServices;
                        onSave(config);
                        setDiscoveryResult({
                          success: true,
                          services: mergedServices,
                          error: null,
                        });
                       } else if (result.error) {
                        // Descriptor Set 解析失败，静默处理
                      }
                    }
                  } catch {
                    // 非 Tauri 环境或用户取消
                  }
                }}
              >
                <Upload className="h-3.5 w-3.5" />
                导入 Descriptor Set
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
          {...register('connect_timeout_ms', { setValueAs: (v: string) => v === '' ? undefined : Number(v) })}
        />
        <p className="text-xs text-muted-foreground">毫秒 (ms)，范围 1000-300000</p>
      </div>

      {/* 压缩编码 */}
      <div className="space-y-2">
        <Label htmlFor="compression_encoding" className="text-sm font-medium text-foreground">
          压缩编码
        </Label>
        <Select
          value={watch('compression_encoding') || 'none'}
          onValueChange={(value) => {
            setValue('compression_encoding', value === 'none' ? undefined : value, { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger id="compression_encoding" className="font-mono text-sm">
            <SelectValue placeholder="无" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="none">无</SelectItem>
            <SelectItem value="gzip">Gzip</SelectItem>
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          连接级默认压缩，gRPC Call 节点可单独覆盖
        </p>
      </div>

      {/* Keepalive 配置 */}
      <div className="space-y-3 rounded-md border bg-muted/30 p-3">
        <div className="flex items-center gap-2">
          <Cable className="h-3.5 w-3.5 text-muted-foreground" />
          <Label className="text-sm font-medium text-foreground">连接设置</Label>
        </div>

        <div className="space-y-2 pl-2">
          <div className="space-y-1">
            <Label className="text-xs text-muted-foreground">PING 间隔 (ms)</Label>
            <Input
              type="number"
              min={1000}
              max={300000}
              placeholder="默认不发送 PING"
              {...register('keepalive_time_ms', { setValueAs: (v: string) => v === '' ? undefined : Number(v) })}
              className="h-8 font-mono text-xs"
            />
            <p className="text-[11px] text-muted-foreground/70">
              HTTP/2 PING 帧发送间隔，范围 1000-300000
            </p>
          </div>

          <div className="space-y-1">
            <Label className="text-xs text-muted-foreground">PING 超时 (ms)</Label>
            <Input
              type="number"
              min={1000}
              max={60000}
              placeholder="默认 20000"
              {...register('keepalive_timeout_ms', { setValueAs: (v: string) => v === '' ? undefined : Number(v) })}
              className="h-8 font-mono text-xs"
            />
            <p className="text-[11px] text-muted-foreground/70">
              等待 PING ACK 的超时时间，范围 1000-60000
            </p>
          </div>

          <div className="flex items-center space-x-2">
            <Switch
              id="keepalive_permit_without_streams"
              checked={watch('keepalive_permit_without_streams') || false}
              onCheckedChange={(checked) => {
                setValue('keepalive_permit_without_streams', checked || undefined, { shouldDirty: true });
                saveData();
              }}
            />
            <Label htmlFor="keepalive_permit_without_streams" className="text-xs text-foreground cursor-pointer">
              无流时允许 PING
            </Label>
          </div>
          <p className="text-[11px] text-muted-foreground/70">
            没有活跃 RPC 调用时也保持连接存活
          </p>
        </div>
      </div>

      {/* Channelz 内省（EXPERIMENTAL） */}
      <div className="space-y-3 rounded-md border bg-muted/30 p-3">
        <div className="flex items-center gap-2">
          <Activity className="h-3.5 w-3.5 text-muted-foreground" />
          <Label className="text-sm font-medium text-foreground">Channelz 内省</Label>
          <span className="text-[10px] text-muted-foreground/60 uppercase tracking-wider">Experimental</span>
        </div>

        <div className="space-y-2 pl-2">
          <p className="text-xs text-muted-foreground">
            查询活跃 gRPC 连接的实时状态信息（连接状态、运行时长、活跃调用数）。
            需要在流程执行期间查询，输入连接 ID（通常为 grpc_connect 节点的 UUID）。
          </p>

          <div className="space-y-1">
            <Label className="text-xs text-muted-foreground">连接 ID</Label>
            <Input
              type="text"
              placeholder="输入连接 ID（如节点 UUID）"
              {...register('channelz_connection_id')}
              className="h-8 font-mono text-xs"
            />
          </div>

          <Button
            type="button"
            variant="outline"
            size="sm"
            className="w-full gap-2"
            disabled={isChannelzLoading}
            onClick={async () => {
              const formData = getValues();
              const connectionId = formData.channelz_connection_id?.trim();
              if (!connectionId) return;

              setIsChannelzLoading(true);
              setChannelzResult(null);
              try {
                const result = await grpcChannelzInfo(connectionId);
                setChannelzResult(result);
              } catch (error) {
                setChannelzResult({
                  success: false,
                  info: null,
                  error: typeof error === 'string' ? error : (error instanceof Error ? error.message : String(error)),
                });
              } finally {
                setIsChannelzLoading(false);
              }
            }}
          >
            {isChannelzLoading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="h-4 w-4" />
            )}
            {isChannelzLoading ? '正在查询...' : '查询连接状态'}
          </Button>

          {/* Channelz 结果 */}
          {channelzResult && (
            <div className={`rounded-md border px-3 py-2 text-xs ${
              channelzResult.success
                ? 'border-violet-200 dark:border-violet-800/60 bg-violet-50 dark:bg-violet-950/30'
                : 'border-rose-200 dark:border-rose-800/60 bg-rose-50 dark:bg-rose-950/30'
            }`}>
              {channelzResult.success && channelzResult.info ? (
                <div className="space-y-1.5">
                  {/* 状态指示 */}
                  <div className="flex items-center gap-2">
                    <span className={cn(
                      'inline-block h-2 w-2 rounded-full',
                      channelzResult.info.state === 'READY' && 'bg-emerald-500',
                      channelzResult.info.state === 'IDLE' && 'bg-amber-500',
                      channelzResult.info.state === 'CONNECTING' && 'bg-blue-500 animate-pulse',
                      channelzResult.info.state === 'TRANSIENT_FAILURE' && 'bg-rose-500',
                    )} />
                    <span className={cn(
                      'font-mono font-medium',
                      channelzResult.info.state === 'READY' && 'text-emerald-600 dark:text-emerald-400',
                      channelzResult.info.state === 'IDLE' && 'text-amber-600 dark:text-amber-400',
                      channelzResult.info.state === 'CONNECTING' && 'text-blue-600 dark:text-blue-400',
                      channelzResult.info.state === 'TRANSIENT_FAILURE' && 'text-rose-600 dark:text-rose-400',
                    )}>
                      {channelzResult.info.state}
                    </span>
                  </div>
                  {/* 详细信息 */}
                  <div className="grid grid-cols-2 gap-1 text-muted-foreground">
                    <span>运行时长:</span>
                    <span className="font-mono text-foreground">
                      {channelzResult.info.uptime_secs < 60
                        ? `${channelzResult.info.uptime_secs}s`
                        : channelzResult.info.uptime_secs < 3600
                          ? `${Math.floor(channelzResult.info.uptime_secs / 60)}m ${channelzResult.info.uptime_secs % 60}s`
                          : `${Math.floor(channelzResult.info.uptime_secs / 3600)}h ${Math.floor((channelzResult.info.uptime_secs % 3600) / 60)}m`
                      }
                    </span>
                    <span>活跃调用:</span>
                    <span className="font-mono text-foreground">{channelzResult.info.active_calls}</span>
                    <span>连接时间:</span>
                    <span className="font-mono text-foreground text-[11px]">
                      {new Date(channelzResult.info.connected_at).toLocaleTimeString()}
                    </span>
                  </div>
                  {channelzResult.info.last_error && (
                    <div className="mt-1 text-rose-500 dark:text-rose-400 text-[11px]">
                      最后错误: {channelzResult.info.last_error}
                    </div>
                  )}
                </div>
              ) : (
                <p className="text-rose-500 dark:text-rose-400">{channelzResult.error}</p>
              )}
            </div>
          )}
        </div>
      </div>

      {/* 默认重试配置 */}
      <div className="space-y-3 rounded-md border bg-muted/30 p-3">
        <p className="text-xs font-medium text-muted-foreground">默认重试配置（EXPERIMENTAL）</p>
        <div className="space-y-2">
          <Label htmlFor="default_max_retries" className="text-xs text-muted-foreground">
            默认最大重试次数
          </Label>
          <Input
            id="default_max_retries"
            type="number"
            min={0}
            max={10}
            placeholder="0（不重试）"
            {...register('max_retries', { setValueAs: (v: string) => v === '' ? undefined : Number(v) })}
            className="h-8 font-mono text-xs"
          />
          <p className="text-[11px] text-muted-foreground/70">
            0 = 不重试，Call 节点可单独覆盖
          </p>
        </div>
        <div className="space-y-2">
          <Label htmlFor="default_initial_backoff_ms" className="text-xs text-muted-foreground">
            默认初始退避时间 (ms)
          </Label>
          <Input
            id="default_initial_backoff_ms"
            type="number"
            min={100}
            max={30000}
            placeholder="1000"
            {...register('initial_backoff_ms', { setValueAs: (v: string) => v === '' ? undefined : Number(v) })}
            className="h-8 font-mono text-xs"
          />
          <p className="text-[11px] text-muted-foreground/70">
            范围 100-30000ms，指数退避（上限 30s）+ 随机抖动
          </p>
        </div>
      </div>

      {/* 负载均衡配置 */}
      <div className="space-y-3 rounded-md border bg-muted/30 p-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Cable className="h-3.5 w-3.5 text-muted-foreground" />
            <Label className="text-sm font-medium text-foreground">负载均衡（EXPERIMENTAL）</Label>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddEndpoint}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            添加端点
          </Button>
        </div>
        <p className="text-xs text-muted-foreground">
          主端点 + 额外端点将使用轮询负载均衡
        </p>
        <div className="space-y-2">
          {(() => {
            const eps = watch('endpoints') ?? [];
            if (eps.length === 0) {
              return (
                <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-3 text-center">
                  <p className="text-xs text-muted-foreground">暂无额外端点</p>
                </div>
              );
            }
            return eps.map((_: string, index: number) => (
              <div key={index} className="flex items-center gap-2 p-2 rounded-md border bg-muted/30">
                <Input
                  {...register(`endpoints.${index}`)}
                  placeholder="http://host:port"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  onClick={() => handleRemoveEndpoint(index)}
                  className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            ));
          })()}
        </div>
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
