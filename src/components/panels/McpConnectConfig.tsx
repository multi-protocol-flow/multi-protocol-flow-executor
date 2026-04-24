/**
 * MCP Connect 节点配置组件
 * 支持服务器地址、自定义请求头、协议版本、工具/资源/提示词发现
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef, useMemo } from 'react';
import {
  mcpConnectConfigSchema,
  type McpConnectConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Plus, Trash2, Plug, Info, Search, Loader2, ChevronDown, Wrench, FileSearch, MessageSquare } from 'lucide-react';
import type { McpConnectConfig as McpConnectConfigType, McpToolInfo, McpResourceInfo, McpPromptInfo } from '@/types';
import { discoverMcpTools } from '@/services/tauri/mcp';
import { cn } from '@/lib/utils';

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
function fromFormData(formData: McpConnectConfigFormData, cachedData?: McpConnectConfigType): McpConnectConfigType {
  const customHeaders = formData.custom_headers?.filter(h => h.key.trim());

  return {
    type: 'mcp_connect',
    url: formData.url,
    protocol_version: formData.protocol_version || undefined,
    custom_headers: customHeaders && customHeaders.length > 0
      ? Object.fromEntries(customHeaders.map(h => [h.key, h.value]))
      : undefined,
    discovered_tools: cachedData?.discovered_tools,
    discovered_resources: cachedData?.discovered_resources,
    discovered_prompts: cachedData?.discovered_prompts,
  };
}

export function McpConnectConfig({ data, onSave }: McpConnectConfigProps) {
  const {
    register,
    handleSubmit,
    control,
    reset,
    getValues,
    watch,
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
        const config = fromFormData(formData, data);
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
        const config = fromFormData(formData, data);
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

  // 服务发现状态
  const [isDiscovering, setIsDiscovering] = useState(false);
  const [discoveryError, setDiscoveryError] = useState<string | null>(null);

  // 可折叠区域的展开状态
  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set());
  const [expandedTool, setExpandedTool] = useState<string | null>(null);

  // 合并发现结果：使用已缓存的数据
  const displayTools = useMemo((): McpToolInfo[] => {
    return data.discovered_tools || [];
  }, [data.discovered_tools]);

  const displayResources = useMemo((): McpResourceInfo[] => {
    return data.discovered_resources || [];
  }, [data.discovered_resources]);

  const displayPrompts = useMemo((): McpPromptInfo[] => {
    return data.discovered_prompts || [];
  }, [data.discovered_prompts]);

  // 监听 URL 变化，清除已发现的数据
  const currentUrl = watch('url');
  const prevUrlRef = useRef(data.url);
  useEffect(() => {
    const trimmedUrl = currentUrl?.trim();
    const prevTrimmed = prevUrlRef.current?.trim();
    if (trimmedUrl !== prevTrimmed && prevTrimmed !== undefined) {
      // URL 发生变化，清除已发现的数据
      const formData = getValues();
      const config = fromFormData(formData);
      config.discovered_tools = undefined;
      config.discovered_resources = undefined;
      config.discovered_prompts = undefined;
      onSave(config);
      setDiscoveryError(null);
      prevUrlRef.current = trimmedUrl;
    } else {
      prevUrlRef.current = trimmedUrl;
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentUrl]);

  /**
   * 测试连接 & 发现服务
   */
  const handleDiscover = useCallback(async () => {
    const formData = getValues();
    const url = formData.url?.trim();
    if (!url) return;

    setIsDiscovering(true);
    setDiscoveryError(null);

    try {
      // 构造自定义请求头
      const customHeaders = formData.custom_headers
        ?.filter(h => h.key.trim())
        ? Object.fromEntries(formData.custom_headers.filter(h => h.key.trim()).map(h => [h.key, h.value]))
        : undefined;

      const result = await discoverMcpTools({
        url,
        headers: customHeaders,
        protocol_version: formData.protocol_version || undefined,
      });

      if (result.success) {
        // 将发现的结果缓存到节点数据中
        const config = fromFormData(formData);
        config.discovered_tools = result.tools;
        config.discovered_resources = result.resources;
        config.discovered_prompts = result.prompts;
        onSave(config);
      } else {
        setDiscoveryError(result.error || '发现服务失败，未知错误');
      }
    } catch (error) {
      console.error('[MCP] 发现服务失败:', error);
      setDiscoveryError(typeof error === 'string' ? error : (error instanceof Error ? error.message : String(error)));
    } finally {
      setIsDiscovering(false);
    }
  }, [getValues, onSave]);

  /**
   * 切换折叠区域
   */
  const toggleSection = useCallback((section: string) => {
    setExpandedSections(prev => {
      const next = new Set(prev);
      if (next.has(section)) next.delete(section);
      else next.add(section);
      return next;
    });
  }, []);

  // 是否有发现的数据
  const hasDiscoveredData = displayTools.length > 0 || displayResources.length > 0 || displayPrompts.length > 0;

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

      {/* 测试连接 & 发现服务 */}
      <div className="space-y-2">
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="w-full gap-2"
          disabled={!watch('url')?.trim() || isDiscovering}
          onClick={handleDiscover}
        >
          {isDiscovering ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Search className="h-4 w-4" />
          )}
          {isDiscovering ? '正在发现服务...' : '测试连接 & 发现服务'}
        </Button>

        {/* 发现错误 */}
        {discoveryError && (
          <div className="rounded-md border border-rose-200 dark:border-rose-800/60 bg-rose-50 dark:bg-rose-950/30 px-3 py-2 text-xs text-rose-600 dark:text-rose-400">
            {discoveryError}
          </div>
        )}

        {/* 发现成功摘要 */}
        {hasDiscoveredData && !discoveryError && (
          <div className="rounded-md border border-emerald-200 dark:border-emerald-800/60 bg-emerald-50 dark:bg-emerald-950/30 px-3 py-2 text-xs text-emerald-700 dark:text-emerald-400">
            <p>发现 {displayTools.length} 个工具, {displayResources.length} 个资源, {displayPrompts.length} 个提示词</p>
          </div>
        )}
      </div>

      {/* 已发现的工具 */}
      {displayTools.length > 0 && (
        <div className="border rounded-lg overflow-hidden">
          <button
            type="button"
            onClick={() => toggleSection('tools')}
            className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors"
          >
            <div className="flex items-center gap-2">
              <Wrench className="h-4 w-4 text-amber-600 dark:text-amber-400" />
              <span className="text-sm font-medium text-foreground">工具</span>
              <Badge variant="secondary" className="h-5 px-1.5 text-[11px]">
                {displayTools.length}
              </Badge>
            </div>
            <ChevronDown className={cn(
              "h-3.5 w-3.5 text-muted-foreground transition-transform",
              expandedSections.has('tools') && "rotate-180"
            )} />
          </button>
          {expandedSections.has('tools') && (
            <div className="border-t bg-muted/20 px-3 py-2 space-y-1.5 max-h-[300px] overflow-y-auto">
              {displayTools.map((tool) => {
                const isExpanded = expandedTool === tool.name;
                const schemaFields = (() => {
                  const schema = tool.input_schema;
                  if (!schema || typeof schema !== 'object' || schema.type !== 'object') return [];
                  const props = schema.properties as Record<string, Record<string, unknown>> | undefined;
                  if (!props) return [];
                  const required = (schema.required as string[]) || [];
                  return Object.entries(props).map(([name, prop]) => ({
                    name,
                    type: String(prop.type || '?'),
                    description: prop.description as string | undefined,
                    required: required.includes(name),
                  }));
                })();
                return (
                  <div key={tool.name} className="rounded-md bg-background/50 overflow-hidden">
                    <button
                      type="button"
                      onClick={() => setExpandedTool(isExpanded ? null : tool.name)}
                      className="w-full flex items-start gap-2 px-2.5 py-2 text-left hover:bg-muted/30 transition-colors"
                    >
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-mono font-medium text-foreground truncate">{tool.name}</p>
                        {tool.description && !isExpanded && (
                          <p className="text-xs text-muted-foreground mt-0.5 truncate">{tool.description}</p>
                        )}
                      </div>
                      <ChevronDown className={cn(
                        "h-3.5 w-3.5 text-muted-foreground shrink-0 mt-1 transition-transform",
                        isExpanded && "rotate-180"
                      )} />
                    </button>
                    {isExpanded && (
                      <div className="px-2.5 pb-2.5 pt-0 border-t border-border/40 space-y-2">
                        {tool.description && (
                          <p className="text-xs leading-relaxed text-foreground/80 whitespace-normal break-words">
                            {tool.description}
                          </p>
                        )}
                        {schemaFields.length > 0 && (
                          <div className="space-y-0.5">
                            <p className="text-[10px] font-medium text-muted-foreground">参数：</p>
                            {schemaFields.map((f) => (
                              <div key={f.name} className="flex items-start gap-1 text-[10px]">
                                <code className="rounded bg-amber-100/80 px-1 py-0 text-amber-900 font-mono shrink-0">{f.name}</code>
                                <span className="text-muted-foreground">{f.type}</span>
                                {f.required && <span className="text-rose-500">*</span>}
                                {f.description && <span className="text-muted-foreground">— {f.description}</span>}
                              </div>
                            ))}
                          </div>
                        )}
                        {!tool.description && schemaFields.length === 0 && (
                          <p className="text-[10px] text-muted-foreground">此工具无描述和参数说明</p>
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}

      {/* 已发现的资源 */}
      {displayResources.length > 0 && (
        <div className="border rounded-lg overflow-hidden">
          <button
            type="button"
            onClick={() => toggleSection('resources')}
            className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors"
          >
            <div className="flex items-center gap-2">
              <FileSearch className="h-4 w-4 text-amber-600 dark:text-amber-400" />
              <span className="text-sm font-medium text-foreground">资源</span>
              <Badge variant="secondary" className="h-5 px-1.5 text-[11px]">
                {displayResources.length}
              </Badge>
            </div>
            <ChevronDown className={cn(
              "h-3.5 w-3.5 text-muted-foreground transition-transform",
              expandedSections.has('resources') && "rotate-180"
            )} />
          </button>
          {expandedSections.has('resources') && (
            <div className="border-t bg-muted/20 px-3 py-2 space-y-1.5 max-h-[300px] overflow-y-auto">
              {displayResources.map((resource) => (
                <div key={resource.uri} className="flex items-start gap-2 rounded-md px-2.5 py-2 bg-background/50">
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-foreground truncate">{resource.name}</p>
                    <p className="text-xs text-muted-foreground font-mono mt-0.5 truncate">{resource.uri}</p>
                    {resource.description && (
                      <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">{resource.description}</p>
                    )}
                  </div>
                  {resource.mime_type && (
                    <Badge variant="outline" className="shrink-0 h-5 px-1.5 text-[10px]">
                      {resource.mime_type}
                    </Badge>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* 已发现的提示词 */}
      {displayPrompts.length > 0 && (
        <div className="border rounded-lg overflow-hidden">
          <button
            type="button"
            onClick={() => toggleSection('prompts')}
            className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors"
          >
            <div className="flex items-center gap-2">
              <MessageSquare className="h-4 w-4 text-amber-600 dark:text-amber-400" />
              <span className="text-sm font-medium text-foreground">提示词</span>
              <Badge variant="secondary" className="h-5 px-1.5 text-[11px]">
                {displayPrompts.length}
              </Badge>
            </div>
            <ChevronDown className={cn(
              "h-3.5 w-3.5 text-muted-foreground transition-transform",
              expandedSections.has('prompts') && "rotate-180"
            )} />
          </button>
          {expandedSections.has('prompts') && (
            <div className="border-t bg-muted/20 px-3 py-2 space-y-1.5 max-h-[300px] overflow-y-auto">
              {displayPrompts.map((prompt) => (
                <div key={prompt.name} className="flex items-start gap-2 rounded-md px-2.5 py-2 bg-background/50">
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-foreground truncate">{prompt.name}</p>
                    {prompt.description && (
                      <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">{prompt.description}</p>
                    )}
                  </div>
                  {prompt.arguments && prompt.arguments.length > 0 && (
                    <Badge variant="outline" className="shrink-0 h-5 px-1.5 text-[10px]">
                      {prompt.arguments.length} 个参数
                    </Badge>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

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
          <p><code className="rounded bg-muted px-1">测试连接</code> 验证连接并发现可用工具、资源和提示词，结果自动缓存供下游节点使用</p>
        </div>
      </div>
    </form>
  );
}
