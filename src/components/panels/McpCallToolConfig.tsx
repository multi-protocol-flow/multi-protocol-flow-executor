/**
 * MCP Call Tool 节点配置组件
 * 支持连接选择（级联下拉）、工具名称（从已发现工具选择）、JSON Schema 驱动参数表单
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { mcpCallToolConfigSchema, type McpCallToolConfigFormData } from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Wrench, Info } from 'lucide-react';
import type { McpCallToolConfig as McpCallToolConfigType, McpToolInfo } from '@/types';
import { useFlowStore } from '@/stores';
import { isMcpConnectConfig } from '@/types/node';
import { getLL } from '@/i18n/i18n-lazy';

// ============================================================================
// Schema 分析辅助
// ============================================================================

/**
 * Schema 字段描述
 */
interface SchemaField {
  name: string;
  type: 'string' | 'number' | 'boolean' | 'array' | 'object';
  description?: string;
  required: boolean;
  default?: unknown;
  enum?: string[];
  items?: { type: string };
}

/**
 * 分析 JSON Schema，判断是否为简单 schema 并提取字段列表
 *
 * 简单 schema 定义：type=object，所有 properties 均为基本类型
 * (string/number/boolean) 或带 enum 的 string，或基本类型的 array
 */
function analyzeSchema(schema?: Record<string, unknown>): {
  isSimple: boolean;
  fields: SchemaField[];
} {
  if (!schema || typeof schema !== 'object') {
    return { isSimple: false, fields: [] };
  }

  // 仅处理 type=object 的 schema
  if (schema.type !== 'object') {
    return { isSimple: false, fields: [] };
  }

  // 检查是否有 oneOf/anyOf/allOf 等复杂结构
  if (schema.oneOf || schema.anyOf || schema.allOf) {
    return { isSimple: false, fields: [] };
  }

  const properties = schema.properties as Record<string, Record<string, unknown>> | undefined;
  if (!properties || typeof properties !== 'object') {
    return { isSimple: false, fields: [] };
  }

  const requiredArr = (schema.required as string[]) || [];
  const fields: SchemaField[] = [];
  let allSimple = true;

  for (const [name, prop] of Object.entries(properties)) {
    if (!prop || typeof prop !== 'object') continue;

    const propType = prop.type as string | undefined;

    // 如果属性本身有 oneOf/anyOf，视为复杂
    if (prop.oneOf || prop.anyOf || prop.allOf) {
      allSimple = false;
      break;
    }

    if (propType === 'string') {
      const enumValues = prop.enum as string[] | undefined;
      fields.push({
        name,
        type: 'string',
        description: prop.description as string | undefined,
        required: requiredArr.includes(name),
        default: prop.default,
        enum: enumValues,
      });
    } else if (propType === 'number' || propType === 'integer') {
      fields.push({
        name,
        type: 'number',
        description: prop.description as string | undefined,
        required: requiredArr.includes(name),
        default: prop.default,
      });
    } else if (propType === 'boolean') {
      fields.push({
        name,
        type: 'boolean',
        description: prop.description as string | undefined,
        required: requiredArr.includes(name),
        default: prop.default,
      });
    } else if (propType === 'array') {
      const items = prop.items as { type: string } | undefined;
      // 仅允许基本类型数组
      if (items && ['string', 'number', 'integer', 'boolean'].includes(items.type)) {
        fields.push({
          name,
          type: 'array',
          description: prop.description as string | undefined,
          required: requiredArr.includes(name),
          default: prop.default,
          items,
        });
      } else {
        allSimple = false;
        break;
      }
    } else if (propType === 'object') {
      // 嵌套对象视为复杂
      allSimple = false;
      break;
    } else {
      // 未知类型视为复杂
      allSimple = false;
      break;
    }
  }

  return { isSimple: allSimple, fields };
}

// ============================================================================
// 组件
// ============================================================================

interface McpCallToolConfigProps {
  /** 当前节点数据 */
  data: McpCallToolConfigType;
  /** 保存回调 */
  onSave: (data: McpCallToolConfigType) => void;
}

/**
 * 将 McpCallToolConfigType 转换为表单数据
 */
function toFormData(data: McpCallToolConfigType): McpCallToolConfigFormData {
  return {
    type: 'mcp_call_tool',
    connection_id: data.connection_id || '',
    tool_name: data.tool_name || '',
    arguments: data.arguments,
  };
}

/**
 * 将表单数据转换为 McpCallToolConfigType
 */
function fromFormData(formData: McpCallToolConfigFormData): McpCallToolConfigType {
  let parsedArgs: Record<string, unknown> | undefined;
  if (formData.arguments) {
    if (typeof formData.arguments === 'string') {
      try {
        parsedArgs = JSON.parse(formData.arguments as string);
      } catch {
        parsedArgs = undefined;
      }
    } else {
      parsedArgs = formData.arguments as Record<string, unknown>;
    }
  }

  return {
    type: 'mcp_call_tool',
    connection_id: formData.connection_id,
    tool_name: formData.tool_name,
    arguments: parsedArgs,
  };
}

export function McpCallToolConfig({ data, onSave }: McpCallToolConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<McpCallToolConfigFormData>({
    resolver: zodResolver(mcpCallToolConfigSchema),
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

  // 从选中连接的 mcp_connect 节点中提取 discovered_tools
  const discoveredTools: McpToolInfo[] = useMemo(() => {
    if (!flow || !currentConnectionId) return [];
    const connectNode = flow.nodes.find(n => n.uuid === currentConnectionId);
    if (!connectNode || !isMcpConnectConfig(connectNode.data)) return [];
    return connectNode.data.discovered_tools || [];
  }, [flow, currentConnectionId]);

  // 工具输入模式：'select' 下拉选择 | 'manual' 手动输入
  const [toolInputMode, setToolInputMode] = useState<'select' | 'manual'>('select');

  // 当前选中的 tool_name
  const currentToolName = watch('tool_name');

  // 当前选中的工具信息
  const selectedTool = useMemo((): McpToolInfo | null => {
    if (!currentToolName || discoveredTools.length === 0) return null;
    return discoveredTools.find(t => t.name === currentToolName) || null;
  }, [currentToolName, discoveredTools]);

  // Schema 分析结果
  const schemaAnalysis = useMemo(() => {
    return analyzeSchema(selectedTool?.input_schema);
  }, [selectedTool]);

  // 参数输入模式：'form' 表单模式 | 'json' JSON 模式
  const [argsMode, setArgsMode] = useState<'form' | 'json'>('form');

  // 表单模式下的字段值缓存
  const [formValues, setFormValues] = useState<Record<string, unknown>>({});

  // 当 schema 变化时，初始化表单值
  useEffect(() => {
    if (schemaAnalysis.isSimple && schemaAnalysis.fields.length > 0) {
      const currentArgs = getValues('arguments');
      const existingArgs = currentArgs && typeof currentArgs === 'object'
        ? currentArgs as Record<string, unknown>
        : {};

      const initialValues: Record<string, unknown> = {};
      for (const field of schemaAnalysis.fields) {
        if (field.name in existingArgs) {
          initialValues[field.name] = existingArgs[field.name];
        } else if (field.default !== undefined) {
          initialValues[field.name] = field.default;
        } else {
          // 根据类型设置默认值
          switch (field.type) {
            case 'boolean': initialValues[field.name] = false; break;
            case 'number': initialValues[field.name] = ''; break;
            case 'array': initialValues[field.name] = ''; break;
            default: initialValues[field.name] = '';
          }
        }
      }
      setFormValues(initialValues);
      // 如果有 schema，默认使用表单模式
      setArgsMode('form');
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [schemaAnalysis.isSimple, schemaAnalysis.fields.length]);

  // 当 connection_id 变化时，重置工具名和参数
  useEffect(() => {
    if (currentConnectionId) {
      const currentTool = getValues('tool_name');
      if (currentTool) {
        const stillExists = discoveredTools.some(t => t.name === currentTool);
        if (!stillExists) {
          setValue('tool_name', '', { shouldDirty: true });
          setValue('arguments' as 'tool_name', undefined as unknown as string, { shouldDirty: true });
          setFormValues({});
        }
      }
    }
  }, [currentConnectionId, discoveredTools, getValues, setValue]);

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
        console.error('保存 MCP Call Tool 配置失败:', error);
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

  // 将 arguments 对象转为字符串用于显示
  const argumentsStr = (() => {
    const args = watch('arguments');
    if (!args) return '';
    if (typeof args === 'string') return args;
    try {
      return JSON.stringify(args, null, 2);
    } catch {
      return '';
    }
  })();

  /**
   * 更新表单模式下的字段值并同步到 arguments
   */
  const handleFormValueChange = useCallback((fieldName: string, value: unknown) => {
    setFormValues(prev => {
      const next = { ...prev, [fieldName]: value };
      // 将表单值收集为 arguments 对象
      const argsObj: Record<string, unknown> = {};
      for (const [k, v] of Object.entries(next)) {
        // 跳过空字符串（未填写的字段）
        if (v === '' || v === undefined) continue;
        argsObj[k] = v;
      }
      setValue('arguments', argsObj, { shouldDirty: true });
      return next;
    });
  }, [setValue]);

  /**
   * 处理数组字段值（逗号分隔字符串 → 数组）
   */
  const handleArrayValueChange = useCallback((fieldName: string, rawValue: string) => {
    if (!rawValue.trim()) {
      handleFormValueChange(fieldName, undefined);
      return;
    }
    const items = rawValue.split(',').map(s => s.trim()).filter(Boolean);
    handleFormValueChange(fieldName, items);
  }, [handleFormValueChange]);

  /**
   * 工具选择变更处理
   */
  const handleToolChange = useCallback((toolName: string) => {
    setValue('tool_name', toolName, { shouldDirty: true });
    setValue('arguments' as 'tool_name', undefined as unknown as string, { shouldDirty: true });
    setFormValues({});
    safeSetTimeout(saveData, 100);
  }, [setValue, safeSetTimeout, saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-amber-200/50 bg-gradient-to-br from-amber-50/80 to-yellow-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-amber-900 flex items-center gap-2">
            <Wrench className="h-4 w-4" />
            {getLL().panels.mcpCallTool.title()}
          </h4>
          <p className="text-sm leading-relaxed text-amber-700/90 whitespace-normal break-words">
            {getLL().panels.mcpCallTool.description()}
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
              // 切换连接时重置工具和参数
              setValue('tool_name', '', { shouldDirty: true });
              setValue('arguments' as 'tool_name', undefined as unknown as string, { shouldDirty: true });
              setFormValues({});
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
            {getLL().panels.mcpCallTool.noConnectNodeDetected()}
          </p>
        )}
      </div>

      {/* 工具名称 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label htmlFor="tool_name" className="text-sm font-medium text-foreground">
            {getLL().panels.mcpCallTool.toolName()} <span className="text-rose-500">*</span>
          </Label>
          {discoveredTools.length > 0 && (
            <button
              type="button"
              onClick={() => setToolInputMode(prev => prev === 'select' ? 'manual' : 'select')}
              className="text-xs text-amber-600 hover:text-amber-800 hover:underline transition-colors"
            >
              {toolInputMode === 'select' ? getLL().panels.manualInput() : getLL().panels.dropdownSelect()}
            </button>
          )}
        </div>
        {discoveredTools.length > 0 && toolInputMode === 'select' ? (
          <div className="space-y-1.5">
            <Select
              value={watch('tool_name') || ''}
              onValueChange={handleToolChange}
            >
              <SelectTrigger id="tool_name" className="font-mono">
                <SelectValue placeholder={getLL().panels.mcpCallTool.selectTool()} />
              </SelectTrigger>
              <SelectContent className="max-h-[280px]">
                {discoveredTools.map((tool) => (
                  <SelectItem key={tool.name} value={tool.name} title={tool.description}>
                    <span className="font-mono text-sm">{tool.name}</span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <div className="flex items-center gap-1.5">
              <span className="inline-flex items-center rounded-full bg-amber-100 px-2 py-0.5 text-xs font-medium text-amber-700">
                {getLL().panels.mcpCallTool.toolCount({ count: discoveredTools.length })}
              </span>
            </div>
          </div>
        ) : (
          <div className="space-y-1.5">
            <Input
              id="tool_name"
              {...register('tool_name')}
              placeholder={getLL().panels.mcpCallTool.toolNamePlaceholder()}
              className="font-mono text-sm"
            />
            {discoveredTools.length > 0 && (
              <p className="text-xs text-muted-foreground">
                {getLL().panels.manualInput()}，
                <button
                  type="button"
                  onClick={() => setToolInputMode('select')}
                  className="text-amber-600 hover:underline"
                >
                  {getLL().panels.switchToDropdown()}
                </button>
              </p>
            )}
          </div>
        )}
        {errors.tool_name && (
          <p className="text-xs text-rose-500">{errors.tool_name.message}</p>
        )}
        {discoveredTools.length === 0 && currentConnectionId && (
          <p className="text-xs text-muted-foreground">
            {getLL().panels.mcpCallTool.noDiscoveredTools()}
          </p>
        )}
      </div>

      {/* 选中工具的详情说明 */}
      {selectedTool && (
        <div className="rounded-lg border border-amber-200/50 bg-gradient-to-br from-amber-50/80 to-yellow-50/50 p-3 overflow-hidden">
          <div className="space-y-2">
            <h5 className="text-xs font-semibold text-amber-900 flex items-center gap-1.5">
              <Wrench className="h-3.5 w-3.5" />
              {selectedTool.name}
            </h5>
            {selectedTool.description && (
              <p className="text-xs leading-relaxed text-amber-800 whitespace-normal break-words">
                {selectedTool.description}
              </p>
            )}
            {schemaAnalysis.fields.length > 0 && (
              <div className="space-y-1 pt-1 border-t border-amber-200/40">
                <p className="text-[10px] font-medium text-amber-700">{getLL().panels.mcpCallTool.paramList()}</p>
                <div className="space-y-0.5">
                  {schemaAnalysis.fields.map((field) => (
                    <div key={field.name} className="flex items-start gap-1 text-[10px]">
                      <code className="rounded bg-amber-100/80 px-1 py-0 text-amber-900 font-mono shrink-0">{field.name}</code>
                      <span className="text-amber-600/80">{field.type}</span>
                      {field.required && <span className="text-rose-500">*</span>}
                      {field.description && <span className="text-muted-foreground">— {field.description}</span>}
                    </div>
                  ))}
                </div>
              </div>
            )}
            {!selectedTool.input_schema && !selectedTool.description && (
              <p className="text-[10px] text-muted-foreground">{getLL().panels.mcpCallTool.noDescription()}</p>
            )}
          </div>
        </div>
      )}

      {/* 工具参数 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">
            {getLL().panels.mcpCallTool.argsLabel()}
          </Label>
          {selectedTool?.input_schema && schemaAnalysis.isSimple && schemaAnalysis.fields.length > 0 && (
            <button
              type="button"
              onClick={() => setArgsMode(prev => prev === 'form' ? 'json' : 'form')}
              className="text-xs text-amber-600 hover:text-amber-800 hover:underline transition-colors"
            >
              {argsMode === 'form' ? getLL().panels.mcpCallTool.jsonMode() : getLL().panels.mcpCallTool.formMode()}
            </button>
          )}
        </div>

        {/* 表单模式：从 schema 生成字段 */}
        {argsMode === 'form' && schemaAnalysis.isSimple && schemaAnalysis.fields.length > 0 ? (
          <div className="space-y-3">
            {schemaAnalysis.fields.map((field) => (
              <div key={field.name} className="space-y-1">
                <Label className="text-xs font-medium text-foreground">
                  {field.name}
                  {field.required && <span className="text-rose-500 ml-0.5">*</span>}
                </Label>

                {/* enum → Select */}
                {field.type === 'string' && field.enum && field.enum.length > 0 ? (
                  <Select
                    value={String(formValues[field.name] ?? '')}
                    onValueChange={(val) => handleFormValueChange(field.name, val)}
                  >
                    <SelectTrigger className="h-8 text-xs font-mono">
                      <SelectValue placeholder={getLL().panels.selectFieldName({ name: field.name })} />
                    </SelectTrigger>
            <SelectContent className="max-h-[280px]">
                      {field.enum.map((opt) => (
                        <SelectItem key={opt} value={opt}>
                          <span className="font-mono text-xs">{opt}</span>
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                ) : field.type === 'boolean' ? (
                  /* boolean → Switch */
                  <div className="flex items-center gap-2">
                    <Switch
                      checked={Boolean(formValues[field.name] ?? false)}
                      onCheckedChange={(checked) => handleFormValueChange(field.name, checked)}
                    />
                    <span className="text-xs text-muted-foreground">
                      {formValues[field.name] ? getLL().panels.mcpCallTool.booleanYes() : getLL().panels.mcpCallTool.booleanNo()}
                    </span>
                  </div>
                ) : field.type === 'number' ? (
                  /* number → Input[number] */
                  <Input
                    type="number"
                    value={String(formValues[field.name] ?? '')}
                    onChange={(e) => {
                      const val = e.target.value;
                      if (val === '') {
                        handleFormValueChange(field.name, undefined);
                      } else {
                        handleFormValueChange(field.name, Number(val));
                      }
                    }}
                    placeholder={field.description || getLL().panels.enterArg({ name: field.name })}
                    className="h-8 text-xs font-mono"
                  />
                ) : field.type === 'array' ? (
                  /* array → comma-separated input */
                  <Input
                    value={Array.isArray(formValues[field.name])
                      ? (formValues[field.name] as unknown[]).join(', ')
                      : String(formValues[field.name] ?? '')}
                    onChange={(e) => handleArrayValueChange(field.name, e.target.value)}
                    placeholder={field.description || getLL().panels.commaSeparatedInput({ name: field.name })}
                    className="h-8 text-xs font-mono"
                  />
                ) : (
                  /* string → Input */
                  <Input
                    value={String(formValues[field.name] ?? '')}
                    onChange={(e) => handleFormValueChange(field.name, e.target.value)}
                    placeholder={field.description || getLL().panels.enterArg({ name: field.name })}
                    className="h-8 text-xs font-mono"
                  />
                )}

                {/* 字段描述 */}
                {field.description && (
                  <p className="text-[10px] text-muted-foreground">{field.description}</p>
                )}
              </div>
            ))}
          </div>
        ) : (
          /* JSON 模式 / 无 Schema / 复杂 Schema */
          <>
            {selectedTool?.input_schema && !schemaAnalysis.isSimple && (
              <div className="rounded-md bg-amber-50/60 border border-amber-100 px-3 py-2 mb-2">
                <p className="text-xs text-amber-700">
                  {getLL().panels.mcpCallTool.complexSchemaHint()}
                </p>
              </div>
            )}
            <Textarea
              id="arguments"
              defaultValue={argumentsStr}
              onChange={(e) => {
                const val = e.target.value.trim();
                if (val) {
                  try {
                    setValue('arguments', JSON.parse(val) as Record<string, unknown>, { shouldDirty: true });
                  } catch {
                    // JSON 解析失败时保留原始字符串
                  }
                } else {
                  setValue('arguments' as 'tool_name', undefined as unknown as string, { shouldDirty: true });
                }
              }}
              placeholder='{"key": "value"}'
              className="min-h-[100px] resize-y font-mono text-xs"
              rows={5}
            />
            <p className="text-xs text-muted-foreground">
              {getLL().panels.mcpCallTool.jsonArgsHint()}
            </p>
          </>
        )}
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.mcpCallTool.callExplanation()}</p>
          <p>{getLL().panels.mcpCallTool.callExplanation1()}</p>
          <p>{getLL().panels.mcpCallTool.callExplanation2()}</p>
          {discoveredTools.length === 0 && currentConnectionId && (
            <p className="text-amber-600">{getLL().panels.mcpCallTool.noDiscoveryHint()}</p>
          )}
        </div>
      </div>
    </form>
  );
}
