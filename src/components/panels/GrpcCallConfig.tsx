/**
 * gRPC Call 节点配置组件
 * 支持连接选择、服务/方法选择、请求 JSON、Metadata、超时
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  grpcCallConfigSchema,
  type GrpcCallConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Plus, Trash2, Radio, Info, FileJson, CheckCircle2, XCircle, Loader2, FileText, Braces, Terminal, Copy, Check } from 'lucide-react';
import type { GrpcCallConfig as GrpcCallConfigType, GrpcServiceInfo, GrpcMethodInfo } from '@/types';
import { useFlowStore } from '@/stores';
import { isGrpcConnectConfig } from '@/types/node';
import { generateGrpcRequestSkeleton, generateGrpcRequestSkeletonFromReflection, validateGrpcRequest } from '@/services/tauri';
import type { ValidationError } from '@/services/tauri';
import { GrpcFormEditor } from './GrpcFormEditor';
import { parseProtoFiles, findMessageDescriptor } from '@/lib/protoFieldParser';
import type { ProtoParseResult } from '@/lib/protoFieldParser';
import { generateGrpcurlCommand, extractGrpcurlParamsFromConnectConfig } from '@/lib/grpcurlGenerator';

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
    request_messages: data.request_messages || [],
    compression_encoding: data.compression_encoding,
    max_retries: data.max_retries,
    initial_backoff_ms: data.initial_backoff_ms,
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
    request_messages: formData.request_messages && formData.request_messages.length > 0
      ? formData.request_messages : undefined,
    compression_encoding: formData.compression_encoding || undefined,
    max_retries: formData.max_retries,
    initial_backoff_ms: formData.initial_backoff_ms,
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

  // 服务/方法选择模式：'select' 表示下拉选择，'manual' 表示手动输入
  const [serviceInputMode, setServiceInputMode] = useState<'select' | 'manual'>('select');
  const [methodInputMode, setMethodInputMode] = useState<'select' | 'manual'>('select');

  // 当前选中的 connection_id
  const currentConnectionId = watch('connection_id');

  // 从选中连接的 grpc_connect 节点中提取 discovered_services
  const discoveredServices = useMemo((): GrpcServiceInfo[] => {
    if (!flow || !currentConnectionId) return [];
    const connectNode = flow.nodes.find(n => n.uuid === currentConnectionId);
    if (!connectNode || !isGrpcConnectConfig(connectNode.data)) return [];
    return connectNode.data.discovered_services || [];
  }, [flow, currentConnectionId]);

  // 从选中连接的 grpc_connect 节点中提取 proto_files
  const protoFiles = useMemo((): [string, string][] => {
    if (!flow || !currentConnectionId) return [];
    const connectNode = flow.nodes.find(n => n.uuid === currentConnectionId);
    if (!connectNode || !isGrpcConnectConfig(connectNode.data)) return [];
    const files = connectNode.data.proto_files;
    if (!files || files.length === 0) return [];
    return files.map(f => [f.path, f.content] as [string, string]);
  }, [flow, currentConnectionId]);

  // 判断连接是否使用 Reflection 模式（无 proto_files 且 enable_reflection=true）
  const isReflectionMode = useMemo((): boolean => {
    if (!flow || !currentConnectionId) return false;
    const connectNode = flow.nodes.find(n => n.uuid === currentConnectionId);
    if (!connectNode || !isGrpcConnectConfig(connectNode.data)) return false;
    const data = connectNode.data;
    return !!data.enable_reflection && (!data.proto_files || data.proto_files.length === 0);
  }, [flow, currentConnectionId]);

  // 从选中连接获取 Reflection 相关配置
  const reflectionConfig = useMemo((): {
    url?: string;
    use_tls?: boolean;
    tls_skip_verify?: boolean;
    connect_timeout_ms?: number;
    tls_ca_cert?: string;
    tls_client_cert?: string;
    tls_client_key?: string;
    tls_server_name_override?: string;
    reflection_metadata?: [string, string][];
  } | null => {
    if (!flow || !currentConnectionId || !isReflectionMode) return null;
    const connectNode = flow.nodes.find(n => n.uuid === currentConnectionId);
    if (!connectNode || !isGrpcConnectConfig(connectNode.data)) return null;
    const data = connectNode.data;
    return {
      url: data.url,
      use_tls: data.use_tls,
      tls_skip_verify: data.tls_skip_verify,
      connect_timeout_ms: data.connect_timeout_ms,
      tls_ca_cert: data.tls_ca_cert,
      tls_client_cert: data.tls_client_cert,
      tls_client_key: data.tls_client_key,
      tls_server_name_override: data.tls_server_name_override,
      reflection_metadata: data.reflection_metadata?.map(m => [m.key, m.value] as [string, string]),
    };
  }, [flow, currentConnectionId, isReflectionMode]);

  // 当前选中的 service_name
  const currentServiceName = watch('service_name');

  // 从已选服务中提取方法列表
  const discoveredMethods = useMemo((): GrpcMethodInfo[] => {
    if (!currentServiceName || discoveredServices.length === 0) return [];
    const service = discoveredServices.find(s => s.service_name === currentServiceName);
    return service?.methods || [];
  }, [currentServiceName, discoveredServices]);

  // 判断选中方法的流式模式
  const selectedMethod = useMemo((): GrpcMethodInfo | null => {
    const methodName = watch('method_name');
    if (!methodName || discoveredMethods.length === 0) return null;
    return discoveredMethods.find(m => m.method_name === methodName) || null;
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [watch('method_name'), discoveredMethods]);

  const isClientStreaming = selectedMethod?.is_client_streaming ?? false;

  // 编辑模式：JSON（默认）| 表单
  const [editMode, setEditMode] = useState<'json' | 'form'>('json');
  // JSON ↔ 表单 切换时的解析错误
  const [modeSwitchError, setModeSwitchError] = useState<string | null>(null);

  // 解析 proto 文件获取 schema（用于表单模式）
  const protoParseResult = useMemo((): ProtoParseResult | null => {
    if (protoFiles.length === 0) return null;
    try {
      const files = protoFiles.map(([path, content]) => ({ path, content }));
      return parseProtoFiles(files);
    } catch {
      return null;
    }
  }, [protoFiles]);

  // grpcurl 导出状态
  const [grpcurlCommand, setGrpcurlCommand] = useState<string | null>(null);
  const [grpcurlCopied, setGrpcurlCopied] = useState(false);

  // 骨架生成加载状态
  const [isGeneratingSkeleton, setIsGeneratingSkeleton] = useState(false);

  // 验证状态
  const [validationErrors, setValidationErrors] = useState<ValidationError[]>([]);
  const [isValidating, setIsValidating] = useState(false);
  const [validationValid, setValidationValid] = useState<boolean | null>(null);
  const validationTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // 判断骨架按钮是否可用（service + method 都已选择，且有 proto_files 或 reflection 模式）
  const canGenerateSkeleton = useMemo(() => {
    const methodName = getValues('method_name');
    return currentServiceName.length > 0
      && methodName.length > 0
      && (protoFiles.length > 0 || isReflectionMode);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentServiceName, protoFiles, isReflectionMode, watch('method_name')]);

  // 当 connection_id 变化时，检查已选服务是否仍有效
  useEffect(() => {
    if (!currentConnectionId || discoveredServices.length === 0) return;
    const currentService = getValues('service_name');
    if (currentService) {
      const stillExists = discoveredServices.some(s => s.service_name === currentService);
      if (!stillExists) {
        // 服务不再存在于新连接中，清空选择
        setValue('service_name', '', { shouldDirty: true });
        setValue('method_name', '', { shouldDirty: true });
      }
    }
  }, [currentConnectionId, discoveredServices, getValues, setValue]);

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
   * 生成请求 JSON 骨架
   * 
   * 双路径：
   * - proto_files 模式：解析 proto 文件生成骨架
   * - reflection 模式：通过 Server Reflection 获取类型信息生成骨架
   */
  const handleGenerateSkeleton = useCallback(async () => {
    const serviceName = getValues('service_name');
    const methodName = getValues('method_name');
    if (!serviceName || !methodName) return;

    // Reflection 模式：通过缓存的连接配置使用 Server Reflection
    if (isReflectionMode && reflectionConfig) {
      if (!reflectionConfig.url) return;
      setIsGeneratingSkeleton(true);
      try {
        const result = await generateGrpcRequestSkeletonFromReflection({
          url: reflectionConfig.url,
          use_tls: reflectionConfig.use_tls,
          tls_skip_verify: reflectionConfig.tls_skip_verify,
          connect_timeout_ms: reflectionConfig.connect_timeout_ms,
          tls_ca_cert: reflectionConfig.tls_ca_cert,
          tls_client_cert: reflectionConfig.tls_client_cert,
          tls_client_key: reflectionConfig.tls_client_key,
          tls_server_name_override: reflectionConfig.tls_server_name_override,
          reflection_metadata: reflectionConfig.reflection_metadata,
          service_name: serviceName,
          method_name: methodName,
        });
        if (result.success && result.skeleton) {
          setValue('request_json', result.skeleton, { shouldDirty: true });
          safeSetTimeout(saveData, 100);
        }
      } catch {
        // Reflection 骨架生成失败，静默处理
      } finally {
        setIsGeneratingSkeleton(false);
      }
      return;
    }

    // proto_files 模式：解析 proto 文件生成骨架
    if (protoFiles.length === 0) return;
    setIsGeneratingSkeleton(true);
    try {
      const result = await generateGrpcRequestSkeleton({
        proto_files: protoFiles,
        service_name: serviceName,
        method_name: methodName,
      });
      if (result.success && result.skeleton) {
        setValue('request_json', result.skeleton, { shouldDirty: true });
        safeSetTimeout(saveData, 100);
      }
    } catch {
      // 骨架生成失败，静默处理
    } finally {
      setIsGeneratingSkeleton(false);
    }
  }, [isReflectionMode, reflectionConfig, protoFiles, getValues, setValue, saveData, safeSetTimeout]);

  /**
   * 导出 grpcurl 命令
   */
  const handleExportGrpcurl = useCallback(() => {
    const serviceName = getValues('service_name');
    const methodName = getValues('method_name');
    const connectionId = getValues('connection_id');

    if (!serviceName || !methodName || !connectionId) return;

    // 从 flow 中找到 grpc_connect 节点
    const connectNode = flow?.nodes.find(n => n.uuid === connectionId);
    if (!connectNode || !isGrpcConnectConfig(connectNode.data)) return;

    const connectParams = extractGrpcurlParamsFromConnectConfig(connectNode.data);
    const requestJson = getValues('request_json');
    const callMetadata = getValues('metadata');

    const command = generateGrpcurlCommand({
      ...connectParams,
      serviceName,
      methodName,
      requestJson,
      callMetadata,
    });

    setGrpcurlCommand(command);
    setGrpcurlCopied(false);
  }, [flow, getValues]);

  /**
   * 复制 grpcurl 命令到剪贴板
   */
  const handleCopyGrpcurl = useCallback(async () => {
    if (!grpcurlCommand) return;
    try {
      await navigator.clipboard.writeText(grpcurlCommand);
      setGrpcurlCopied(true);
      setTimeout(() => setGrpcurlCopied(false), 2000);
    } catch {
      // 剪贴板写入失败，忽略
    }
  }, [grpcurlCommand]);

  // 获取当前选中的方法的 input_type（用于验证）
  const selectedMethodInputType = useMemo((): string | null => {
    if (!selectedMethod) return null;
    return selectedMethod.input_type;
  }, [selectedMethod]);

  // 当前方法的 input message 是否有可用的 schema（用于表单模式）
  const hasSchemaForForm = useMemo((): boolean => {
    if (!protoParseResult || !selectedMethodInputType) return false;
    return findMessageDescriptor(protoParseResult, selectedMethodInputType) !== null;
  }, [protoParseResult, selectedMethodInputType]);

  // 防抖验证请求 JSON
  useEffect(() => {
    // 清除之前的定时器
    if (validationTimerRef.current) {
      clearTimeout(validationTimerRef.current);
      validationTimerRef.current = null;
    }

    // 前置条件检查
    const requestJson = getValues('request_json');
    if (!requestJson || !selectedMethodInputType || protoFiles.length === 0) {
      setValidationValid(null);
      setValidationErrors([]);
      setIsValidating(false);
      return;
    }

    // 空对象不验证
    if (requestJson.trim() === '{}') {
      setValidationValid(null);
      setValidationErrors([]);
      return;
    }

    setIsValidating(true);

    // 300ms 防抖
    const timerId = setTimeout(async () => {
      try {
        const result = await validateGrpcRequest(requestJson, selectedMethodInputType, protoFiles);
        if (result.success && result.result) {
          setValidationValid(result.result.valid);
          setValidationErrors(result.result.errors);
        } else {
          // 验证执行失败（如 proto 解析失败），不显示验证结果
          setValidationValid(null);
          setValidationErrors([]);
        }
      } catch {
        setValidationValid(null);
        setValidationErrors([]);
      } finally {
        setIsValidating(false);
      }
    }, 300);
    validationTimerRef.current = timerId;

    return () => {
      clearTimeout(timerId);
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [watch('request_json'), selectedMethodInputType, protoFiles, safeSetTimeout]);

  // 消息列表字段数组（Client/Bidi Streaming 模式）
  const messageFields = useFieldArray({
    control,
    name: 'request_messages' as never,
  });

  const handleAddMessage = useCallback(() => {
    messageFields.append({ enabled: true, content: '{}' });
  }, [messageFields]);

  const handleRemoveMessage = useCallback((index: number) => {
    messageFields.remove(index);
    safeSetTimeout(saveData, 100);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [messageFields, saveData, safeSetTimeout]);

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
              // 切换连接时重置服务和方法选择
              setValue('service_name', '', { shouldDirty: true });
              setValue('method_name', '', { shouldDirty: true });
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
        <div className="flex items-center justify-between">
          <Label htmlFor="service_name" className="text-sm font-medium text-foreground">
            服务名称 <span className="text-rose-500">*</span>
          </Label>
          {discoveredServices.length > 0 && (
            <button
              type="button"
              onClick={() => setServiceInputMode(prev => prev === 'select' ? 'manual' : 'select')}
              className="text-xs text-violet-600 hover:text-violet-800 hover:underline transition-colors"
            >
              {serviceInputMode === 'select' ? '手动输入' : '下拉选择'}
            </button>
          )}
        </div>
        {discoveredServices.length > 0 && serviceInputMode === 'select' ? (
          <div className="space-y-1.5">
            <Select
              value={watch('service_name') || ''}
              onValueChange={(value) => {
                setValue('service_name', value, { shouldDirty: true });
                // 切换服务时清空方法选择
                setValue('method_name', '', { shouldDirty: true });
                setMethodInputMode('select');
                safeSetTimeout(saveData, 100);
              }}
            >
              <SelectTrigger id="service_name" className="font-mono">
                <SelectValue placeholder="选择服务" />
              </SelectTrigger>
              <SelectContent>
                {discoveredServices.map((svc) => (
                  <SelectItem key={svc.service_name} value={svc.service_name}>
                    <span className="font-mono text-sm">{svc.service_name}</span>
                    <span className="ml-2 text-xs text-muted-foreground">
                      ({svc.methods.length} 个方法)
                    </span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <div className="flex items-center gap-1.5">
              <span className="inline-flex items-center rounded-full bg-violet-100 px-2 py-0.5 text-xs font-medium text-violet-700">
                {discoveredServices.length} 个服务
              </span>
            </div>
          </div>
        ) : (
          <div className="space-y-1.5">
            <Input
              id="service_name"
              {...register('service_name')}
              placeholder="package.ServiceName"
              className="font-mono text-sm"
            />
            {discoveredServices.length > 0 && (
              <p className="text-xs text-muted-foreground">
                当前为手动输入模式，
                <button
                  type="button"
                  onClick={() => setServiceInputMode('select')}
                  className="text-violet-600 hover:underline"
                >
                  切换到下拉选择
                </button>
              </p>
            )}
          </div>
        )}
        {errors.service_name && (
          <p className="text-xs text-rose-500">{errors.service_name.message}</p>
        )}
        {discoveredServices.length === 0 && (
          <p className="text-xs text-muted-foreground">
            完整的服务名称（含包名），如 <code className="rounded bg-muted px-1">mypackage.MyService</code>
            {currentConnectionId && '（连接节点未发现服务，请手动输入）'}
          </p>
        )}
      </div>

      {/* 方法选择 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label htmlFor="method_name" className="text-sm font-medium text-foreground">
            方法名称 <span className="text-rose-500">*</span>
          </Label>
          {discoveredMethods.length > 0 && (
            <button
              type="button"
              onClick={() => setMethodInputMode(prev => prev === 'select' ? 'manual' : 'select')}
              className="text-xs text-violet-600 hover:text-violet-800 hover:underline transition-colors"
            >
              {methodInputMode === 'select' ? '手动输入' : '下拉选择'}
            </button>
          )}
        </div>
        {discoveredMethods.length > 0 && methodInputMode === 'select' ? (
          <div className="space-y-1.5">
            <Select
              value={watch('method_name') || ''}
              onValueChange={(value) => {
                setValue('method_name', value, { shouldDirty: true });
                safeSetTimeout(saveData, 100);
              }}
            >
              <SelectTrigger id="method_name" className="font-mono">
                <SelectValue placeholder="选择方法" />
              </SelectTrigger>
              <SelectContent>
                {discoveredMethods.map((method) => (
                  <SelectItem key={method.method_name} value={method.method_name}>
                    <span className="font-mono text-sm">{method.method_name}</span>
                    {method.is_server_streaming && method.is_client_streaming ? (
                      <span className="ml-2 inline-flex items-center rounded-full bg-purple-100 px-1.5 py-0.5 text-[10px] font-medium text-purple-700">
                        双向流式
                      </span>
                    ) : method.is_server_streaming ? (
                      <span className="ml-2 inline-flex items-center rounded-full bg-amber-100 px-1.5 py-0.5 text-[10px] font-medium text-amber-700">
                        服务端流式
                      </span>
                    ) : method.is_client_streaming ? (
                      <span className="ml-2 inline-flex items-center rounded-full bg-blue-100 px-1.5 py-0.5 text-[10px] font-medium text-blue-700">
                        客户端流式
                      </span>
                    ) : (
                      <span className="ml-2 inline-flex items-center rounded-full bg-green-100 px-1.5 py-0.5 text-[10px] font-medium text-green-700">
                        Unary
                      </span>
                    )}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {/* 方法类型信息 */}
            {watch('method_name') && (() => {
              const selected = discoveredMethods.find(m => m.method_name === watch('method_name'));
              if (!selected) return null;
              return (
                <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                  <span className="font-mono text-emerald-700 bg-emerald-50 rounded px-1.5 py-0.5">
                    {selected.input_type}
                  </span>
                  <span>→</span>
                  <span className="font-mono text-blue-700 bg-blue-50 rounded px-1.5 py-0.5">
                    {selected.output_type}
                  </span>
                </div>
              );
            })()}
          </div>
        ) : (
          <div className="space-y-1.5">
            <Input
              id="method_name"
              {...register('method_name')}
              placeholder="MethodName"
              className="font-mono text-sm"
            />
            {discoveredMethods.length > 0 && (
              <p className="text-xs text-muted-foreground">
                当前为手动输入模式，
                <button
                  type="button"
                  onClick={() => setMethodInputMode('select')}
                  className="text-violet-600 hover:underline"
                >
                  切换到下拉选择
                </button>
              </p>
            )}
          </div>
        )}
        {errors.method_name && (
          <p className="text-xs text-rose-500">{errors.method_name.message}</p>
        )}
      </div>

      {/* 导出 grpcurl 命令 */}
      <div className="space-y-2">
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={!currentServiceName || !watch('method_name') || !currentConnectionId}
            onClick={handleExportGrpcurl}
            className="h-7 gap-1.5 text-xs"
          >
            <Terminal className="h-3 w-3" />
            导出 grpcurl
          </Button>
        </div>

        {/* grpcurl 命令展示 */}
        {grpcurlCommand && (
          <div className="space-y-2 rounded-md border bg-muted/30 p-3">
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium text-muted-foreground">grpcurl 命令</span>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={handleCopyGrpcurl}
                className="h-6 gap-1 text-xs text-muted-foreground hover:text-foreground"
              >
                {grpcurlCopied ? (
                  <>
                    <Check className="h-3 w-3 text-emerald-500" />
                    已复制
                  </>
                ) : (
                  <>
                    <Copy className="h-3 w-3" />
                    复制到剪贴板
                  </>
                )}
              </Button>
            </div>
            <pre className="max-h-[200px] overflow-auto rounded bg-zinc-900 p-3 text-xs text-emerald-400 whitespace-pre-wrap break-all font-mono">
              {grpcurlCommand}
            </pre>
            <button
              type="button"
              onClick={() => setGrpcurlCommand(null)}
              className="text-xs text-muted-foreground hover:text-foreground"
            >
              关闭
            </button>
          </div>
        )}
      </div>

      {/* 消息列表（Client/Bidi Streaming 模式） */}
      {isClientStreaming && (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <Label className="text-sm font-medium text-foreground">
              消息列表
            </Label>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={handleAddMessage}
              className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
            >
              <Plus className="h-3 w-3" />
              添加消息
            </Button>
          </div>
          {messageFields.fields.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">点击"添加消息"按钮添加请求消息</p>
            </div>
          ) : (
            <div className="space-y-2">
              {messageFields.fields.map((field, index) => (
                <div key={field.id} className="flex items-start gap-2 p-2 rounded-md border bg-muted/30">
                  <input
                    type="checkbox"
                    {...register(`request_messages.${index}.enabled`)}
                    className="mt-2 h-4 w-4 shrink-0"
                    defaultChecked={(field as Record<string, unknown>).enabled as boolean}
                  />
                  <div className="flex-1 space-y-1">
                    <Textarea
                      {...register(`request_messages.${index}.content`)}
                      placeholder='{"field": "value"}'
                      className="min-h-[60px] resize-y font-mono text-xs"
                      rows={3}
                    />
                    <p className="text-[10px] text-muted-foreground">消息 {index + 1}</p>
                  </div>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    onClick={() => handleRemoveMessage(index)}
                    className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>
              ))}
            </div>
          )}
          <p className="text-xs text-muted-foreground">
            启用的消息将按顺序发送，禁用的消息会被跳过
          </p>
        </div>
      )}

      {/* 单消息编辑器（Unary/Server Streaming 模式） */}
      {!isClientStreaming && (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <Label htmlFor="request_json" className="text-sm font-medium text-foreground">
              请求编辑
            </Label>
            <div className="flex items-center gap-2">
              {/* 编辑模式切换 */}
              <div className="inline-flex items-center rounded-lg bg-muted p-0.5">
                <button
                  type="button"
                  onClick={() => { setEditMode('json'); setModeSwitchError(null); }}
                  className={`inline-flex items-center gap-1 rounded-md px-2.5 py-1 text-xs font-medium transition-all ${
                    editMode === 'json'
                      ? 'bg-background text-foreground shadow-sm'
                      : 'text-muted-foreground hover:text-foreground'
                  }`}
                >
                  <Braces className="h-3 w-3" />
                  JSON
                </button>
                <button
                  type="button"
                  disabled={!hasSchemaForForm}
                  onClick={() => {
                    if (!hasSchemaForForm) return;
                    // 切换到表单模式前验证 JSON
                    const currentJson = getValues('request_json');
                    if (currentJson && currentJson.trim() !== '{}') {
                      try {
                        JSON.parse(currentJson);
                      } catch {
                        setModeSwitchError('JSON 格式错误，无法切换到表单模式');
                        return;
                      }
                    }
                    setModeSwitchError(null);
                    setEditMode('form');
                  }}
                  className={`inline-flex items-center gap-1 rounded-md px-2.5 py-1 text-xs font-medium transition-all ${
                    editMode === 'form'
                      ? 'bg-background text-foreground shadow-sm'
                      : hasSchemaForForm
                        ? 'text-muted-foreground hover:text-foreground'
                        : 'text-muted-foreground/40 cursor-not-allowed'
                  }`}
                >
                  <FileText className="h-3 w-3" />
                  表单
                </button>
              </div>
              {/* 生成骨架按钮（仅 JSON 模式显示） */}
              {editMode === 'json' && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  disabled={!canGenerateSkeleton || isGeneratingSkeleton}
                  onClick={handleGenerateSkeleton}
                  className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground disabled:opacity-50"
                >
                  <FileJson className="h-3 w-3" />
                  {isGeneratingSkeleton ? '生成中...' : '生成骨架'}
                </Button>
              )}
            </div>
          </div>

          {/* 模式切换错误提示 */}
          {modeSwitchError && (
            <div className="flex items-center gap-1.5 rounded border border-rose-200 bg-rose-50/50 px-2.5 py-1.5 text-xs text-rose-600">
              <XCircle className="h-3 w-3 shrink-0" />
              <span>{modeSwitchError}</span>
            </div>
          )}

          {/* JSON 编辑模式 */}
          {editMode === 'json' && (
            <>
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
              {/* 验证状态指示器 */}
              {isValidating && (
                <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                  <Loader2 className="h-3 w-3 animate-spin" />
                  <span>验证中...</span>
                </div>
              )}
              {!isValidating && validationValid === true && (
                <div className="flex items-center gap-1.5 text-xs text-emerald-600">
                  <CheckCircle2 className="h-3 w-3" />
                  <span>JSON 与 proto schema 匹配</span>
                </div>
              )}
              {!isValidating && validationValid === false && validationErrors.length > 0 && (
                <div className="space-y-1">
                  <div className="flex items-center gap-1.5 text-xs text-amber-600">
                    <XCircle className="h-3 w-3" />
                    <span>发现 {validationErrors.length} 个问题（不影响发送）</span>
                  </div>
                  <div className="max-h-[120px] overflow-y-auto rounded border border-amber-200 bg-amber-50/50 p-2">
                    {validationErrors.map((err, idx) => (
                      <div key={idx} className="flex items-start gap-1 text-xs text-amber-700">
                        <span className="shrink-0 font-mono text-amber-900">{err.path || '(root)'}</span>
                        <span className="text-amber-500">—</span>
                        <span>{err.message}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}
              <p className="text-xs text-muted-foreground">
                请求消息的 JSON 格式，将自动序列化发送
              </p>
            </>
          )}

          {/* 表单编辑模式 */}
          {editMode === 'form' && (
            <>
              <div className="rounded-md border bg-background p-3">
                <GrpcFormEditor
                  value={watch('request_json') || '{}'}
                  onChange={(json: string) => {
                    setValue('request_json', json, { shouldDirty: true });
                  }}
                  parseResult={protoParseResult}
                  inputType={selectedMethodInputType}
                  onBlur={handleBlur}
                />
              </div>
              {!hasSchemaForForm && (
                <p className="text-xs text-muted-foreground">
                  {protoFiles.length === 0
                    ? '无 proto 文件，表单模式不可用。请切换到 JSON 模式或添加 proto 文件。'
                    : '未找到匹配的消息定义，请切换到 JSON 模式。'}
                </p>
              )}
            </>
          )}
        </div>
      )}

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

      {/* 压缩编码 */}
      <div className="space-y-2">
        <Label htmlFor="call_compression_encoding" className="text-sm font-medium text-foreground">
          压缩编码
        </Label>
        <Select
          value={watch('compression_encoding') || 'inherit'}
          onValueChange={(value) => {
            setValue('compression_encoding', value === 'inherit' ? undefined : value, { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger id="call_compression_encoding" className="font-mono text-sm">
            <SelectValue placeholder="继承默认" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="inherit">继承默认</SelectItem>
            <SelectItem value="none">无</SelectItem>
            <SelectItem value="gzip">Gzip</SelectItem>
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          "继承默认" 使用连接节点的压缩配置
        </p>
      </div>

      {/* 重试配置 */}
      <div className="space-y-3 rounded-md border bg-muted/30 p-3">
        <p className="text-xs font-medium text-muted-foreground">重试配置（EXPERIMENTAL）</p>
        <div className="space-y-2">
          <Label htmlFor="max_retries" className="text-xs text-muted-foreground">
            最大重试次数
          </Label>
          <Input
            id="max_retries"
            type="number"
            min={0}
            max={10}
            placeholder="继承默认"
            {...register('max_retries', { valueAsNumber: true })}
            className="h-8 font-mono text-xs"
          />
          <p className="text-[11px] text-muted-foreground/70">
            0 = 不重试，留空使用连接级默认值
          </p>
        </div>
        <div className="space-y-2">
          <Label htmlFor="call_initial_backoff_ms" className="text-xs text-muted-foreground">
            初始退避时间 (ms)
          </Label>
          <Input
            id="call_initial_backoff_ms"
            type="number"
            min={100}
            max={30000}
            placeholder="继承默认"
            {...register('initial_backoff_ms', { valueAsNumber: true })}
            className="h-8 font-mono text-xs"
          />
          <p className="text-[11px] text-muted-foreground/70">
            范围 100-30000ms，留空使用连接级默认值
          </p>
        </div>
        <p className="text-[11px] text-muted-foreground/70">
          仅对 Unary 和 Server Streaming 调用生效
        </p>
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
