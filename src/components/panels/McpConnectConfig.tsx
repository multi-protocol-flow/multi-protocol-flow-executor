/**
 * MCP Connect 节点配置组件
 * 支持服务器地址、自定义请求头、协议版本
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef } from 'react';
import {
  mcpConnectConfigSchema,
  type McpConnectConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Plus, Trash2, Plug, Info } from 'lucide-react';
import type { McpConnectConfig as McpConnectConfigType } from '@/types';

interface McpConnectConfigProps {
  /** 当前节点数据 */
  data: McpConnectConfigType;
  /** 保存回调 */
  onSave: (data: McpConnectConfigType) => void;
}

/**
 * 将 McpConnectConfigType 转换为表单数据
 */
function toFormData(data: McpConnectConfigType): McpConnectConfigFormData {
  const customHeaders = data.custom_headers
    ? Object.entries(data.custom_headers).map(([key, value]) => ({ key, value }))
    : [];

  return {
    type: 'mcp_connect',
    url: data.url || '',
    protocol_version: data.protocol_version || '2025-11-25',
    custom_headers: customHeaders.length > 0 ? customHeaders : [],
  };
}

/**
 * 将表单数据转换为 McpConnectConfigType
 */
function fromFormData(formData: McpConnectConfigFormData): McpConnectConfigType {
  const customHeaders = formData.custom_headers?.filter(h => h.key.trim());

  return {
    type: 'mcp_connect',
    url: formData.url,
    protocol_version: formData.protocol_version || undefined,
    custom_headers: customHeaders && customHeaders.length > 0
      ? Object.fromEntries(customHeaders.map(h => [h.key, h.value]))
      : undefined,
  };
}

export function McpConnectConfig({ data, onSave }: McpConnectConfigProps) {
  const {
    register,
    handleSubmit,
    control,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<McpConnectConfigFormData>({
    resolver: zodResolver(mcpConnectConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const headerFields = useFieldArray({
    control,
    name: 'custom_headers' as never,
  });

  // 追踪表单实际修改状态
  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

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
        console.error('保存 MCP Connect 配置失败:', error);
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
   * 添加自定义请求头
   */
  const handleAddHeader = useCallback(() => {
    headerFields.append({ key: '', value: '' });
  }, [headerFields]);

  /**
   * 删除自定义请求头
   */
  const handleRemoveHeader = useCallback((index: number) => {
    headerFields.remove(index);
    safeSetTimeout(saveData, 100);
  }, [headerFields, saveData, safeSetTimeout]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-amber-200/50 bg-gradient-to-br from-amber-50/80 to-yellow-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-amber-900 flex items-center gap-2">
            <Plug className="h-4 w-4" />
            MCP 连接节点
          </h4>
          <p className="text-sm leading-relaxed text-amber-700/90 whitespace-normal break-words">
            建立 MCP（Model Context Protocol）连接，支持 Streamable HTTP 传输。
          </p>
        </div>
      </div>

      {/* 服务器地址 */}
      <div className="space-y-2">
        <Label htmlFor="url" className="text-sm font-medium text-foreground">
          MCP 服务器地址 <span className="text-rose-500">*</span>
        </Label>
        <div className="relative">
          <Plug className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <Input
            id="url"
            {...register('url')}
            placeholder="http://localhost:3000/mcp"
            className="pl-9 font-mono text-sm"
          />
        </div>
        {errors.url && (
          <p className="text-xs text-rose-500">{errors.url.message}</p>
        )}
        <p className="text-xs text-muted-foreground">
          MCP 服务器的 Streamable HTTP 端点地址
        </p>
      </div>

      {/* 协议版本 */}
      <div className="space-y-2">
        <Label htmlFor="protocol_version" className="text-sm font-medium text-foreground">
          协议版本
        </Label>
        <Input
          id="protocol_version"
          {...register('protocol_version')}
          placeholder="2025-11-25"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          MCP 协议版本号，默认 2025-11-25
        </p>
      </div>

      {/* 自定义请求头 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">自定义请求头（可选）</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddHeader}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            添加
          </Button>
        </div>
        <div className="space-y-2">
          {headerFields.fields.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">暂无自定义请求头</p>
            </div>
          ) : (
            headerFields.fields.map((field, index) => (
              <div key={field.id} className="flex items-center gap-2 p-2 rounded-md border bg-muted/30">
                <Input
                  {...register(`custom_headers.${index}.key`)}
                  placeholder="Key"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Input
                  {...register(`custom_headers.${index}.value`)}
                  placeholder="Value"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  onClick={() => handleRemoveHeader(index)}
                  className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            ))
          )}
        </div>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">MCP 协议说明：</p>
          <p>MCP（Model Context Protocol）用于与大语言模型工具和服务交互</p>
          <p>支持 Streamable HTTP 传输方式，兼容标准 MCP 服务器</p>
        </div>
      </div>
    </form>
  );
}
