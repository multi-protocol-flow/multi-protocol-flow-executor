/**
 * MCP Get Prompt 节点配置组件
 * 支持选择连接、提示词名称（从已发现提示词选择）、参数配置
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { mcpGetPromptConfigSchema, type McpGetPromptConfigFormData } from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { MessageSquare, Info } from 'lucide-react';
import type { McpGetPromptConfig as McpGetPromptConfigType, McpPromptInfo } from '@/types';
import { useFlowStore } from '@/stores';
import { isMcpConnectConfig } from '@/types/node';
import { getLL } from '@/i18n/i18n-lazy';

interface McpGetPromptConfigProps {
  /** 当前节点数据 */
  data: McpGetPromptConfigType;
  /** 保存回调 */
  onSave: (data: McpGetPromptConfigType) => void;
}

/**
 * 将 McpGetPromptConfigType 转换为表单数据
 */
function toFormData(data: McpGetPromptConfigType): McpGetPromptConfigFormData {
  return {
    type: 'mcp_get_prompt',
    connection_id: data.connection_id || '',
    prompt_name: data.prompt_name || '',
    arguments: data.arguments,
  };
}

/**
 * 将表单数据转换为 McpGetPromptConfigType
 */
function fromFormData(formData: McpGetPromptConfigFormData): McpGetPromptConfigType {
  return {
    type: 'mcp_get_prompt',
    connection_id: formData.connection_id,
    prompt_name: formData.prompt_name,
    arguments: formData.arguments,
  };
}

export function McpGetPromptConfig({ data, onSave }: McpGetPromptConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<McpGetPromptConfigFormData>({
    resolver: zodResolver(mcpGetPromptConfigSchema),
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

  // 从选中连接的 mcp_connect 节点中提取 discovered_prompts
  const discoveredPrompts: McpPromptInfo[] = useMemo(() => {
    if (!flow || !currentConnectionId) return [];
    const connectNode = flow.nodes.find(n => n.uuid === currentConnectionId);
    if (!connectNode || !isMcpConnectConfig(connectNode.data)) return [];
    return connectNode.data.discovered_prompts || [];
  }, [flow, currentConnectionId]);

  // 提示词输入模式：'select' 下拉选择 | 'manual' 手动输入
  const [promptInputMode, setPromptInputMode] = useState<'select' | 'manual'>('select');

  // 当前选中的 prompt_name
  const currentPromptName = watch('prompt_name');

  // 当前选中的提示词信息
  const selectedPrompt = useMemo((): McpPromptInfo | null => {
    if (!currentPromptName || discoveredPrompts.length === 0) return null;
    return discoveredPrompts.find(p => p.name === currentPromptName) || null;
  }, [currentPromptName, discoveredPrompts]);

  // 参数输入值缓存
  const [argValues, setArgValues] = useState<Record<string, string>>({});

  // 当选中的提示词变化时，初始化参数值
  useEffect(() => {
    if (selectedPrompt?.arguments && selectedPrompt.arguments.length > 0) {
      const existingArgs = getValues('arguments') || {};
      const initialValues: Record<string, string> = {};
      for (const arg of selectedPrompt.arguments) {
        initialValues[arg.name] = existingArgs[arg.name] || '';
      }
      setArgValues(initialValues);
    } else {
      setArgValues({});
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedPrompt?.name]);

  // 当 connection_id 变化时，重置提示词名称和参数
  useEffect(() => {
    if (currentConnectionId) {
      const currentPrompt = getValues('prompt_name');
      if (currentPrompt) {
        const stillExists = discoveredPrompts.some(p => p.name === currentPrompt);
        if (!stillExists) {
          setValue('prompt_name', '', { shouldDirty: true });
          setValue('arguments' as 'prompt_name', undefined as unknown as string, { shouldDirty: true });
          setArgValues({});
        }
      }
    }
  }, [currentConnectionId, discoveredPrompts, getValues, setValue]);

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
        console.error('保存 MCP Get Prompt 配置失败:', error);
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
   * 处理参数值变化
   */
  const handleArgValueChange = useCallback((argName: string, value: string) => {
    setArgValues(prev => {
      const next = { ...prev, [argName]: value };
      // 收集非空参数
      const argsObj: Record<string, string> = {};
      for (const [k, v] of Object.entries(next)) {
        if (v.trim()) {
          argsObj[k] = v;
        }
      }
      const argsToSet = Object.keys(argsObj).length > 0 ? argsObj : undefined;
      setValue('arguments' as 'prompt_name', argsToSet as unknown as string, { shouldDirty: true });
      return next;
    });
  }, [setValue]);

  /**
   * 处理提示词选择变更
   */
  const handlePromptChange = useCallback((promptName: string) => {
    setValue('prompt_name', promptName, { shouldDirty: true });
    setValue('arguments' as 'prompt_name', undefined as unknown as string, { shouldDirty: true });
    setArgValues({});
    safeSetTimeout(saveData, 100);
  }, [setValue, safeSetTimeout, saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-amber-200/50 bg-gradient-to-br from-amber-50/80 to-yellow-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-amber-900 flex items-center gap-2">
            <MessageSquare className="h-4 w-4" />
            {getLL().panels.mcpGetPrompt.title()}
          </h4>
          <p className="text-sm leading-relaxed text-amber-700/90 whitespace-normal break-words">
            {getLL().panels.mcpGetPrompt.description()}
          </p>
        </div>
      </div>

      {/* 连接选择 */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          {getLL().panels.connection()} <span className="text-rose-500">*</span>
        </Label>
        {availableConnections.length > 0 ? (
          <Select
            value={watch('connection_id') || ''}
            onValueChange={(value) => {
              setValue('connection_id', value, { shouldDirty: true });
              // 切换连接时重置提示词和参数
              setValue('prompt_name', '', { shouldDirty: true });
              setValue('arguments' as 'prompt_name', undefined as unknown as string, { shouldDirty: true });
              setArgValues({});
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
          <p className="text-xs text-rose-500">{errors.connection_id.message}</p>
        )}
        {availableConnections.length === 0 && (
          <p className="text-xs text-muted-foreground">
            {getLL().panels.mcpGetPrompt.noConnectNodeDetected()}
          </p>
        )}
      </div>

      {/* 提示词名称 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label htmlFor="prompt_name" className="text-sm font-medium text-foreground">
            {getLL().panels.mcpGetPrompt.promptName()} <span className="text-rose-500">*</span>
          </Label>
          {discoveredPrompts.length > 0 && (
            <button
              type="button"
              onClick={() => setPromptInputMode(prev => prev === 'select' ? 'manual' : 'select')}
              className="text-xs text-amber-600 hover:text-amber-800 hover:underline transition-colors"
            >
              {promptInputMode === 'select' ? getLL().panels.manualInput() : getLL().panels.dropdownSelect()}
            </button>
          )}
        </div>
        {discoveredPrompts.length > 0 && promptInputMode === 'select' ? (
          <div className="space-y-1.5">
            <Select
              value={watch('prompt_name') || ''}
              onValueChange={handlePromptChange}
            >
              <SelectTrigger id="prompt_name" className="font-mono">
                <SelectValue placeholder={getLL().panels.mcpGetPrompt.selectPrompt()} />
              </SelectTrigger>
              <SelectContent>
                {discoveredPrompts.map((prompt) => (
                  <SelectItem key={prompt.name} value={prompt.name} title={prompt.description}>
                    <span className="font-mono text-sm">{prompt.name}</span>
                    {prompt.description && (
                      <span className="ml-2 text-xs text-muted-foreground truncate max-w-[200px] inline-block align-bottom">
                        {prompt.description.length > 30
                          ? prompt.description.substring(0, 30) + '...'
                          : prompt.description}
                      </span>
                    )}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <div className="flex items-center gap-1.5">
              <span className="inline-flex items-center rounded-full bg-amber-100 px-2 py-0.5 text-xs font-medium text-amber-700">
                {discoveredPrompts.length} 个提示词
              </span>
            </div>
          </div>
        ) : (
          <div className="space-y-1.5">
            <Input
              id="prompt_name"
              {...register('prompt_name')}
              placeholder="例如：code_review、summarize"
              className="font-mono text-sm"
            />
            {discoveredPrompts.length > 0 && (
              <p className="text-xs text-muted-foreground">
                {getLL().panels.manualInput()}，
                <button
                  type="button"
                  onClick={() => setPromptInputMode('select')}
                  className="text-amber-600 hover:underline"
                >
                  {getLL().panels.switchToDropdown()}
                </button>
              </p>
            )}
          </div>
        )}
        {errors.prompt_name && (
          <p className="text-xs text-rose-500">{errors.prompt_name.message}</p>
        )}
        {discoveredPrompts.length === 0 && currentConnectionId && (
          <p className="text-xs text-muted-foreground">
            {getLL().panels.mcpGetPrompt.noDiscoveredPrompts()}
          </p>
        )}
      </div>

      {/* 选中提示词的描述 */}
      {selectedPrompt?.description && (
        <div className="rounded-md bg-amber-50/60 border border-amber-100 px-3 py-2">
          <p className="text-xs text-amber-800 whitespace-normal break-words">
            {selectedPrompt.description}
          </p>
        </div>
      )}

      {/* 提示词参数 */}
      {selectedPrompt?.arguments && selectedPrompt.arguments.length > 0 && (
        <div className="space-y-2">
          <Label className="text-sm font-medium text-foreground">
            提示词参数
          </Label>
          <div className="space-y-3">
            {selectedPrompt.arguments.map((arg) => (
              <div key={arg.name} className="space-y-1">
                <Label className="text-xs font-medium text-foreground">
                  {arg.name}
                  {arg.required && <span className="text-rose-500 ml-0.5">*</span>}
                </Label>
                <Input
                  value={argValues[arg.name] ?? ''}
                  onChange={(e) => handleArgValueChange(arg.name, e.target.value)}
                  placeholder={`输入 ${arg.name}`}
                  className="h-8 text-xs font-mono"
                />
                {arg.description && (
                  <p className="text-[10px] text-muted-foreground">{arg.description}</p>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.mcpGetPrompt.promptExplanation()}</p>
          <p>{getLL().panels.mcpGetPrompt.promptExplanation1()}</p>
          <p>{getLL().panels.mcpGetPrompt.promptExplanation2()}</p>
          {discoveredPrompts.length === 0 && currentConnectionId && (
            <p className="text-amber-600">{getLL().panels.mcpGetPrompt.noDiscoveryHint()}</p>
          )}
        </div>
      </div>
    </form>
  );
}
