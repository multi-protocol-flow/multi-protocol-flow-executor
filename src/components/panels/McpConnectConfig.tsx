/**
 * MCP Connect 节点配置组件
 * 支持 Streamable HTTP 和 Stdio 两种传输模式
 * 支持服务器地址、自定义请求头、{getLL().panels.mcpConnect.protocolVersion()}、工具/资源/提示词发现
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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Plus, Trash2, Plug, Info, Search, Loader2, ChevronDown, Wrench, FileSearch, MessageSquare, Terminal } from 'lucide-react';
import type { McpConnectConfig as McpConnectConfigType, McpToolInfo, McpResourceInfo, McpPromptInfo } from '@/types';
import { discoverMcpTools } from '@/services/tauri/mcp';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';

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

  const envPairs = data.env
    ? Object.entries(data.env).map(([key, value]) => ({ key, value }))
    : [];

  return {
    type: 'mcp_connect',
    transport: data.transport || 'streamable_http',
    url: data.url || '',
    protocol_version: data.protocol_version || '2025-11-25',
    custom_headers: customHeaders.length > 0 ? customHeaders : [],
    command: data.command || '',
    args: data.args || [],
    env: envPairs.length > 0 ? envPairs : [],
  };
}

/**
 * 将表单数据转换为 McpConnectConfigType
 */
function fromFormData(formData: McpConnectConfigFormData, cachedData?: McpConnectConfigType): McpConnectConfigType {
  const customHeaders = formData.custom_headers?.filter(h => h.key.trim());
  const envPairs = formData.env?.filter(h => h.key.trim());
  const transport = formData.transport || 'streamable_http';

  return {
    type: 'mcp_connect',
    transport,
    url: transport === 'streamable_http' ? (formData.url || '') : '',
    protocol_version: formData.protocol_version || undefined,
    custom_headers: customHeaders && customHeaders.length > 0
      ? Object.fromEntries(customHeaders.map(h => [h.key, h.value]))
      : undefined,
    command: transport === 'stdio' ? (formData.command || undefined) : undefined,
    args: transport === 'stdio' && formData.args && formData.args.length > 0
      ? formData.args
      : undefined,
    env: transport === 'stdio' && envPairs && envPairs.length > 0
      ? Object.fromEntries(envPairs.map(h => [h.key, h.value]))
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
    setValue,
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

  const envFields = useFieldArray({
    control,
    name: 'env' as never,
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
  }, [handleSubmit, onSave, data]);

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

  /**
   * 添加环境变量
   */
  const handleAddEnv = useCallback(() => {
    envFields.append({ key: '', value: '' });
  }, [envFields]);

  /**
   * 删除环境变量
   */
  const handleRemoveEnv = useCallback((index: number) => {
    envFields.remove(index);
    safeSetTimeout(saveData, 100);
  }, [envFields, saveData, safeSetTimeout]);

  // 服务发现状态
  const [isDiscovering, setIsDiscovering] = useState(false);
  const [discoveryError, setDiscoveryError] = useState<string | null>(null);

  // 可折叠区域的展开状态
  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set());
  const [expandedTool, setExpandedTool] = useState<string | null>(null);

  // 当前传输模式
  const currentTransport = watch('transport') || 'streamable_http';

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
  const prevUrlRef = useRef<string | undefined>(data.url);
  useEffect(() => {
    const trimmedUrl = currentUrl?.trim();
    const prevTrimmed = prevUrlRef.current?.trim();
    if (trimmedUrl !== prevTrimmed && prevTrimmed !== undefined) {
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

  // 监听 command 变化，清除已发现的数据
  const currentCommand = watch('command');
  const prevCommandRef = useRef(data.command);
  useEffect(() => {
    const trimmedCommand = currentCommand?.trim();
    const prevTrimmed = prevCommandRef.current?.trim();
    if (trimmedCommand !== prevTrimmed && prevTrimmed !== undefined) {
      const formData = getValues();
      const config = fromFormData(formData);
      config.discovered_tools = undefined;
      config.discovered_resources = undefined;
      config.discovered_prompts = undefined;
      onSave(config);
      setDiscoveryError(null);
      prevCommandRef.current = trimmedCommand;
    } else {
      prevCommandRef.current = trimmedCommand;
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentCommand]);

  /**
   * 传输模式切换处理
   */
  const handleTransportChange = useCallback((value: string) => {
    const newTransport = value as 'streamable_http' | 'stdio';
    setValue('transport', newTransport, { shouldDirty: true });

    // 清除已发现的数据
    const formData = getValues();
    const config = fromFormData({ ...formData, transport: newTransport });
    config.discovered_tools = undefined;
    config.discovered_resources = undefined;
    config.discovered_prompts = undefined;
    onSave(config);
    setDiscoveryError(null);

    safeSetTimeout(saveData, 100);
  }, [setValue, getValues, onSave, saveData, safeSetTimeout]);

  /**
   * {getLL().panels.mcpConnect.testConnectionDiscover()}
   */
  const handleDiscover = useCallback(async () => {
    const formData = getValues();
    const transport = formData.transport || 'streamable_http';

    // 按模式验证必填字段
    if (transport === 'streamable_http') {
      const url = formData.url?.trim();
      if (!url) return;
    } else {
      const command = formData.command?.trim();
      if (!command) return;
    }

    setIsDiscovering(true);
    setDiscoveryError(null);

    try {
      // 构造自定义请求头
      const customHeaders = formData.custom_headers
        ?.filter(h => h.key.trim())
        ? Object.fromEntries(formData.custom_headers.filter(h => h.key.trim()).map(h => [h.key, h.value]))
        : undefined;

      // 构造环境变量
      const envMap = formData.env
        ?.filter(h => h.key.trim())
        ? Object.fromEntries(formData.env.filter(h => h.key.trim()).map(h => [h.key, h.value]))
        : undefined;

      const result = await discoverMcpTools({
        url: transport === 'streamable_http' ? (formData.url?.trim() || '') : '',
        headers: transport === 'streamable_http' ? customHeaders : undefined,
        protocol_version: formData.protocol_version || undefined,
        transport,
        command: transport === 'stdio' ? formData.command?.trim() : undefined,
        args: transport === 'stdio' && formData.args && formData.args.length > 0
          ? formData.args
          : undefined,
        env: transport === 'stdio' ? envMap : undefined,
      });

      if (result.success) {
        const config = fromFormData(formData);
        config.discovered_tools = result.tools;
        config.discovered_resources = result.resources;
        config.discovered_prompts = result.prompts;
        onSave(config);
      } else {
        setDiscoveryError(result.error || getLL().panels.mcpConnect.discoveryFailed());
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

  // 发现按钮是否禁用
  const isDiscoverDisabled = isDiscovering || (
    currentTransport === 'streamable_http'
      ? !watch('url')?.trim()
      : !watch('command')?.trim()
  );

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-amber-200/50 bg-gradient-to-br from-amber-50/80 to-yellow-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-amber-900 flex items-center gap-2">
            <Plug className="h-4 w-4" />
            {getLL().panels.mcpConnect.title()}
          </h4>
          <p className="text-sm leading-relaxed text-amber-700/90 whitespace-normal break-words">
            {getLL().panels.mcpConnect.description()}
          </p>
        </div>
      </div>

      {/* 传输模式选择 */}
      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">
          传输模式
        </Label>
        <Select
          value={currentTransport}
          onValueChange={handleTransportChange}
        >
          <SelectTrigger className="font-mono text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="streamable_http">Streamable HTTP</SelectItem>
            <SelectItem value="stdio">Stdio</SelectItem>
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          {currentTransport === 'streamable_http'
            ? '{getLL().panels.mcpConnect.transportHttpDesc()}'
            : '{getLL().panels.mcpConnect.transportStdioDesc()}'}
        </p>
      </div>

      {/* 按传输模式条件渲染表单 */}
      {currentTransport === 'streamable_http' ? (
        <>
          {/* Streamable HTTP 模式字段 */}

          {/* 服务器地址 */}
          <div className="space-y-2">
            <Label htmlFor="url" className="text-sm font-medium text-foreground">
              {getLL().panels.mcpConnect.serverAddress()} <span className="text-rose-500">*</span>
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
              {getLL().panels.mcpConnect.serverAddressHint()}
            </p>
          </div>

          {/* 自定义请求头 */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-sm font-medium text-foreground">{getLL().panels.mcpConnect.customHeaders()}</Label>
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
                  <p className="text-xs text-muted-foreground">{getLL().panels.mcpConnect.noCustomHeaders()}</p>
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
        </>
      ) : (
        <>
          {/* Stdio 模式字段 */}

          {/* Command */}
          <div className="space-y-2">
            <Label htmlFor="command" className="text-sm font-medium text-foreground">
              {getLL().panels.mcpConnect.command()} <span className="text-rose-500">*</span>
            </Label>
            <div className="relative">
              <Terminal className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
              <Input
                id="command"
                {...register('command')}
                placeholder="npx"
                className="pl-9 font-mono text-sm"
              />
            </div>
            {errors.command && (
              <p className="text-xs text-rose-500">{errors.command.message}</p>
            )}
            <p className="text-xs text-muted-foreground">
              {getLL().panels.mcpConnect.commandHint()}
            </p>
          </div>

          {/* Args */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-sm font-medium text-foreground">{getLL().panels.mcpConnect.command()}参数（可选）</Label>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => {
                  const currentArgs = getValues('args') || [];
                  setValue('args', [...currentArgs, ''], { shouldDirty: true });
                }}
                className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
              >
                <Plus className="h-3 w-3" />
                添加
              </Button>
            </div>
            <div className="space-y-2">
              {(() => {
                const args = watch('args') || [];
                if (args.length === 0) {
                  return (
                    <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
                      <p className="text-xs text-muted-foreground">{getLL().panels.mcpConnect.noCommandArgs()}</p>
                    </div>
                  );
                }
                return args.map((_, index) => (
                  <div key={index} className="flex items-center gap-2 p-2 rounded-md border bg-muted/30">
                    <Input
                      {...register(`args.${index}`)}
                      placeholder="-y"
                      className="h-8 flex-1 font-mono text-xs"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => {
                        const currentArgs = getValues('args') || [];
                        const newArgs = currentArgs.filter((_, i) => i !== index);
                        setValue('args', newArgs, { shouldDirty: true });
                        safeSetTimeout(saveData, 100);
                      }}
                      className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                ));
              })()}
            </div>
          </div>

          {/* Env 环境变量 */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-sm font-medium text-foreground">{getLL().panels.mcpConnect.envVars()}</Label>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={handleAddEnv}
                className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
              >
                <Plus className="h-3 w-3" />
                添加
              </Button>
            </div>
            <div className="space-y-2">
              {envFields.fields.length === 0 ? (
                <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
                  <p className="text-xs text-muted-foreground">{getLL().panels.mcpConnect.noEnvVars()}</p>
                </div>
              ) : (
                envFields.fields.map((field, index) => (
                  <div key={field.id} className="flex items-center gap-2 p-2 rounded-md border bg-muted/30">
                    <Input
                      {...register(`env.${index}.key`)}
                      placeholder="Key"
                      className="h-8 flex-1 font-mono text-xs"
                    />
                    <Input
                      {...register(`env.${index}.value`)}
                      placeholder="Value"
                      className="h-8 flex-1 font-mono text-xs"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => handleRemoveEnv(index)}
                      className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                ))
              )}
            </div>
          </div>

          {/* Stdio 模式提示 */}
          <div className="flex items-start gap-2 rounded-md border border-blue-200/50 bg-gradient-to-br from-blue-50/80 to-indigo-50/50 p-3">
            <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-blue-500" />
            <div className="text-xs text-blue-700/90 space-y-1">
              <p>{getLL().panels.mcpConnect.stdioInfo1()}</p>
              <p>{getLL().panels.mcpConnect.stdioInfo2()}</p>
            </div>
          </div>
        </>
      )}

      {/* {getLL().panels.mcpConnect.testConnectionDiscover()} */}
      <div className="space-y-2">
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="w-full gap-2"
          disabled={isDiscoverDisabled}
          onClick={handleDiscover}
        >
          {isDiscovering ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Search className="h-4 w-4" />
          )}
          {isDiscovering ? getLL().panels.mcpConnect.discoveringServices() : getLL().panels.mcpConnect.testConnectionDiscover()}
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
            <p>getLL().panels.mcpConnect.discoverySummary(displayTools.length, displayResources.length, displayPrompts.length)</p>
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
              <span className="text-sm font-medium text-foreground">{getLL().panels.mcpConnect.tools()}</span>
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
                            <p className="text-[10px] font-medium text-muted-foreground">{getLL().panels.mcpConnect.parametersLabel()}</p>
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
                          <p className="text-[10px] text-muted-foreground">{getLL().panels.mcpConnect.noDescription()}</p>
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
              <span className="text-sm font-medium text-foreground">{getLL().panels.mcpConnect.resources()}</span>
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
              <span className="text-sm font-medium text-foreground">{getLL().panels.mcpConnect.prompts()}</span>
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
                      getLL().panels.mcpConnect.argCount()
                    </Badge>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* {getLL().panels.mcpConnect.protocolVersion()} */}
      <div className="space-y-2">
        <Label htmlFor="protocol_version" className="text-sm font-medium text-foreground">
          {getLL().panels.mcpConnect.protocolVersion()}
        </Label>
        <Input
          id="protocol_version"
          {...register('protocol_version')}
          placeholder="2025-11-25"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          MCP {getLL().panels.mcpConnect.protocolVersion()}号，默认 2025-11-25
        </p>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.mcpConnect.protocolExplanation()}</p>
          <p>{getLL().panels.mcpConnect.protocolExplanation1()}</p>
          <p>{getLL().panels.mcpConnect.protocolExplanation2()}</p>
          <p><code className="rounded bg-muted px-1">测试连接</code> 验证连接并发现可用工具、资源和提示词，结果自动缓存供下游节点使用</p>
        </div>
      </div>
    </form>
  );
}
