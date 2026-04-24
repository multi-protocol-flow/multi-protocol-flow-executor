/**
 * MCP Call Tool 节点配置组件
 * 支持选择连接、工具名称、参数配置
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import { mcpCallToolConfigSchema, type McpCallToolConfigFormData } from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Wrench, Info } from 'lucide-react';
import type { McpCallToolConfig as McpCallToolConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { isMcpConnectConfig } from '@/types/node';

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

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-amber-200/50 bg-gradient-to-br from-amber-50/80 to-yellow-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-amber-900 flex items-center gap-2">
            <Wrench className="h-4 w-4" />
            MCP 工具调用节点
          </h4>
          <p className="text-sm leading-relaxed text-amber-700/90 whitespace-normal break-words">
            调用 MCP 服务器上注册的工具，传入参数并获取结果。
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

      {/* 工具名称 */}
      <div className="space-y-2">
        <Label htmlFor="tool_name" className="text-sm font-medium text-foreground">
          工具名称 <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="tool_name"
          {...register('tool_name')}
          placeholder="例如：search、query"
          className="font-mono text-sm"
        />
        {errors.tool_name && (
          <p className="text-xs text-rose-500">{errors.tool_name.message}</p>
        )}
      </div>

      {/* 工具参数 */}
      <div className="space-y-2">
        <Label htmlFor="arguments" className="text-sm font-medium text-foreground">
          工具参数（JSON）
        </Label>
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
          JSON 格式的工具参数，留空则不传参数
        </p>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">工具调用说明：</p>
          <p>工具名称需与 MCP 服务器上注册的工具匹配</p>
          <p>参数以 JSON 格式传入，结构取决于工具定义</p>
        </div>
      </div>
    </form>
  );
}
