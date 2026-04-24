/**
 * MCP Read Resource 节点配置组件
 * 支持选择连接、资源 URI（从已发现资源选择或手动输入）
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { mcpReadResourceConfigSchema, type McpReadResourceConfigFormData } from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { FileText, Info } from 'lucide-react';
import type { McpReadResourceConfig as McpReadResourceConfigType, McpResourceInfo } from '@/types';
import { useFlowStore } from '@/stores';
import { isMcpConnectConfig } from '@/types/node';

interface McpReadResourceConfigProps {
  /** 当前节点数据 */
  data: McpReadResourceConfigType;
  /** 保存回调 */
  onSave: (data: McpReadResourceConfigType) => void;
}

/**
 * 将 McpReadResourceConfigType 转换为表单数据
 */
function toFormData(data: McpReadResourceConfigType): McpReadResourceConfigFormData {
  return {
    type: 'mcp_read_resource',
    connection_id: data.connection_id || '',
    resource_uri: data.resource_uri || '',
  };
}

/**
 * 将表单数据转换为 McpReadResourceConfigType
 */
function fromFormData(formData: McpReadResourceConfigFormData): McpReadResourceConfigType {
  return {
    type: 'mcp_read_resource',
    connection_id: formData.connection_id,
    resource_uri: formData.resource_uri,
  };
}

export function McpReadResourceConfig({ data, onSave }: McpReadResourceConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<McpReadResourceConfigFormData>({
    resolver: zodResolver(mcpReadResourceConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 追踪表单实际修改状态
  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  // 获取 flow 并提取 mcp_connect 节点
  const flow = useFlowStore((state) => state.flow);

  // 提取可用的 MCP 连接
  const availableConnections = useMemo(() => {
    if (!flow) return [];
    const connections: Array<{ uuid: string; name: string }> = [];
    flow.nodes.forEach(node => {
      if (isMcpConnectConfig(node.data)) {
        connections.push({
          uuid: node.uuid,
          name: node.name,
        });
      }
    });
    return connections;
  }, [flow]);

  // 当前选中的 connection_id
  const currentConnectionId = watch('connection_id');

  // 从选中连接的 mcp_connect 节点中提取 discovered_resources
  const discoveredResources: McpResourceInfo[] = useMemo(() => {
    if (!flow || !currentConnectionId) return [];
    const connectNode = flow.nodes.find(n => n.uuid === currentConnectionId);
    if (!connectNode || !isMcpConnectConfig(connectNode.data)) return [];
    return connectNode.data.discovered_resources || [];
  }, [flow, currentConnectionId]);

  // 资源 URI 输入模式：'select' 下拉选择 | 'manual' 手动输入
  const [uriInputMode, setUriInputMode] = useState<'select' | 'manual'>('select');

  // 当 connection_id 变化时，重置 resource_uri
  useEffect(() => {
    if (currentConnectionId) {
      const currentUri = getValues('resource_uri');
      if (currentUri) {
        const stillExists = discoveredResources.some(r => r.uri === currentUri);
        if (!stillExists) {
          setValue('resource_uri', '', { shouldDirty: true });
        }
      }
    }
  }, [currentConnectionId, discoveredResources, getValues, setValue]);

  // 当外部数据变化时，重置表单
  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  // 追踪待执行的 setTimeout，组件卸载时清除
  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

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

  /**
   * 保存表单数据
   */
  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 MCP Read Resource 配置失败:', error);
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

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-amber-200/50 bg-gradient-to-br from-amber-50/80 to-yellow-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-amber-900 flex items-center gap-2">
            <FileText className="h-4 w-4" />
            MCP 资源读取节点
          </h4>
          <p className="text-sm leading-relaxed text-amber-700/90 whitespace-normal break-words">
            读取 MCP 服务器上的资源内容，通过 URI 定位。
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
              // 切换连接时重置资源 URI
              setValue('resource_uri', '', { shouldDirty: true });
              saveData();
            }}
          >
            <SelectTrigger id="connection_id" className="font-mono">
              <SelectValue placeholder="选择 MCP 连接" />
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
            未检测到 MCP Connect 节点，请手动输入连接 ID
          </p>
        )}
      </div>

      {/* 资源 URI */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label htmlFor="resource_uri" className="text-sm font-medium text-foreground">
            资源 URI <span className="text-rose-500">*</span>
          </Label>
          {discoveredResources.length > 0 && (
            <button
              type="button"
              onClick={() => setUriInputMode(prev => prev === 'select' ? 'manual' : 'select')}
              className="text-xs text-amber-600 hover:text-amber-800 hover:underline transition-colors"
            >
              {uriInputMode === 'select' ? '手动输入' : '下拉选择'}
            </button>
          )}
        </div>
        {discoveredResources.length > 0 && uriInputMode === 'select' ? (
          <div className="space-y-1.5">
            <Select
              value={watch('resource_uri') || ''}
              onValueChange={(value) => {
                setValue('resource_uri', value, { shouldDirty: true });
                safeSetTimeout(saveData, 100);
              }}
            >
              <SelectTrigger id="resource_uri" className="font-mono">
                <SelectValue placeholder="选择资源" />
              </SelectTrigger>
              <SelectContent>
                {discoveredResources.map((resource) => (
                  <SelectItem key={resource.uri} value={resource.uri} title={resource.description}>
                    <span className="font-mono text-sm">{resource.name}</span>
                    <span className="ml-2 text-xs text-muted-foreground truncate max-w-[200px] inline-block align-bottom">
                      ({resource.uri.length > 40
                        ? resource.uri.substring(0, 40) + '...'
                        : resource.uri})
                    </span>
                    {resource.mime_type && (
                      <span className="ml-2 inline-flex items-center rounded-full bg-amber-100 px-1.5 py-0.5 text-[10px] font-medium text-amber-700">
                        {resource.mime_type}
                      </span>
                    )}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <div className="flex items-center gap-1.5">
              <span className="inline-flex items-center rounded-full bg-amber-100 px-2 py-0.5 text-xs font-medium text-amber-700">
                {discoveredResources.length} 个资源
              </span>
            </div>
          </div>
        ) : (
          <div className="space-y-1.5">
            <Input
              id="resource_uri"
              {...register('resource_uri')}
              placeholder="例如：file:///path/to/resource"
              className="font-mono text-sm"
            />
            {discoveredResources.length > 0 && (
              <p className="text-xs text-muted-foreground">
                当前为手动输入模式，
                <button
                  type="button"
                  onClick={() => setUriInputMode('select')}
                  className="text-amber-600 hover:underline"
                >
                  切换到下拉选择
                </button>
              </p>
            )}
          </div>
        )}
        {errors.resource_uri && (
          <p className="text-xs text-rose-500">{errors.resource_uri.message}</p>
        )}
        {discoveredResources.length === 0 && currentConnectionId && (
          <p className="text-xs text-muted-foreground">
            连接节点未发现资源，请手动输入资源 URI
          </p>
        )}
        {!currentConnectionId && (
          <p className="text-xs text-muted-foreground">
            MCP 资源的 URI 标识符
          </p>
        )}
      </div>

      {/* 选中资源的描述 */}
      {(() => {
        const currentUri = watch('resource_uri');
        if (!currentUri || discoveredResources.length === 0) return null;
        const selectedResource = discoveredResources.find(r => r.uri === currentUri);
        if (!selectedResource?.description) return null;
        return (
          <div className="rounded-md bg-amber-50/60 border border-amber-100 px-3 py-2">
            <p className="text-xs text-amber-800 whitespace-normal break-words">
              {selectedResource.description}
            </p>
          </div>
        );
      })()}

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">资源读取说明：</p>
          <p>资源 URI 需与 MCP 服务器支持的资源路径匹配</p>
          <p>读取结果将作为节点输出传递给后续节点</p>
          {discoveredResources.length === 0 && currentConnectionId && (
            <p className="text-amber-600">提示：连接节点尚未发现资源，请先在连接节点中执行资源发现</p>
          )}
        </div>
      </div>
    </form>
  );
}
