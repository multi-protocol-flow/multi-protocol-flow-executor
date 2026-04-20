/**
 * WebSocket Connect 节点配置组件
 * 支持 URL、Headers、Subprotocols、等待配置
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import {
  wsConnectConfigSchema,
  type WsConnectConfigFormData,
  type WsCollectMode,
  type WsMatchType,
} from './schemas';
import { Input } from '@/components/ui/input';
import { VariableInput } from '@/components/ui/VariableInput';
import { VariableValidationWarning } from '@/components/ui/VariableValidationWarning';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { Plus, Trash2, Wifi, Info, ArrowRightFromLine } from 'lucide-react';
import type { WsConnectConfig as WsConnectConfigType, FormField } from '@/types';
import type { Variable } from '@/types/variable';
import { useFlowStore } from '@/stores';
import { isEntryConfig, isHttpConfig, isWsConnectConfig } from '@/types/node';

interface WsConnectConfigProps {
  /** 当前节点数据 */
  data: WsConnectConfigType;
  /** 保存回调 */
  onSave: (data: WsConnectConfigType) => void;
}

/**
 * 将 WsConnectConfigType 转换为表单数据
 */
function toFormData(data: WsConnectConfigType): WsConnectConfigFormData {
  const headers: FormField[] = data.headers || [];

  return {
    type: 'ws_connect',
    url: data.url || '',
    headers: headers.length > 0 ? headers : undefined,
    subprotocols: data.subprotocols || [],
    connect_timeout_ms: data.connect_timeout_ms ?? 30000,
    collect_message: data.collect_message || false,
    collect_mode: data.collect_mode || 'first_match',
    collect_condition: data.collect_condition,
    collect_timeout_ms: data.collect_timeout_ms ?? 30000,
    output_mappings: data.output_mappings || [],
  };
}

/**
 * 将表单数据转换为 WsConnectConfigType
 */
function fromFormData(formData: WsConnectConfigFormData): WsConnectConfigType {
  // 过滤掉空的 headers
  const headers = formData.headers?.filter(field => field.key.trim());

  return {
    type: 'ws_connect',
    url: formData.url,
    headers: headers && headers.length > 0 ? headers : undefined,
    subprotocols: formData.subprotocols && formData.subprotocols.length > 0 ? formData.subprotocols : undefined,
    connect_timeout_ms: formData.connect_timeout_ms,
    collect_message: formData.collect_message,
    collect_mode: formData.collect_message ? formData.collect_mode : undefined,
    collect_condition: formData.collect_message && formData.collect_mode === 'condition_match' ? formData.collect_condition : undefined,
    collect_timeout_ms: formData.collect_message ? formData.collect_timeout_ms : undefined,
    output_mappings: formData.output_mappings && formData.output_mappings.length > 0 ? formData.output_mappings : undefined,
  };
}

export function WsConnectConfig({ data, onSave }: WsConnectConfigProps) {
  const {
    register,
    handleSubmit,
    control,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<WsConnectConfigFormData>({
    resolver: zodResolver(wsConnectConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const { fields, append, remove } = useFieldArray({
    control,
    name: 'headers' as never,
  });

  const outputMappings = useFieldArray({
    control,
    name: 'output_mappings' as never,
  });

  // 获取 flow 并提取变量
  const flow = useFlowStore((state) => state.flow);

  // 提取变量列表
  const variables: Variable[] = useMemo(() => {
    if (!flow) return [];
    const vars: Variable[] = [];
    flow.nodes.forEach(node => {
      // Entry 节点：提取 initial_variables
      if (isEntryConfig(node.data) && node.data.initial_variables) {
        node.data.initial_variables.forEach(iv => {
          vars.push({
            name: iv.name,
            value: iv.value,
            sourceNodeUuid: node.uuid,
            description: iv.description,
          });
        });
      }
      // HTTP 节点：提取 output_mappings
      if (isHttpConfig(node.data) && node.data.output_mappings) {
        node.data.output_mappings.forEach(om => {
          vars.push({
            name: om.target,
            value: `{{${om.source}}}`,
            sourceNodeUuid: node.uuid,
            description: om.description,
          });
        });
      }
      // WS Connect 节点：提取 output_mappings
      if (isWsConnectConfig(node.data) && node.data.output_mappings) {
        node.data.output_mappings.forEach(om => {
          vars.push({
            name: om.target,
            value: `{{${om.source}}}`,
            sourceNodeUuid: node.uuid,
            description: om.description,
          });
        });
      }
    });
    return vars;
  }, [flow]);

  // 提取可用变量名列表（用于验证）
  const availableVariableNames = useMemo(() => {
    return variables.map(v => v.name);
  }, [variables]);

  // 节点标签映射
  const nodeLabels = useMemo(() => {
    if (!flow) return {};
    const labels: Record<string, string> = {};
    flow.nodes.forEach(node => {
      labels[node.uuid] = node.name;
    });
    return labels;
  }, [flow]);

  // 追踪待执行的 setTimeout，组件卸载时清除，防止保存到错误的节点
  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

  // 追踪表单实际修改状态（useRef 用于卸载时读取）
  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  // 组件卸载时同步保存（仅当用户实际修改过表单时）
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

  // 监听表单值变化
  const collectMessage = watch('collect_message');
  const collectMode = watch('collect_mode');

  // 当外部数据变化时，重置表单（确保所有字段同步）
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
        console.error('保存 WebSocket Connect 配置失败:', error);
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

  // 监听 Ctrl+S 刷新事件，将表单数据同步到 store
  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  /**
   * 添加 Header
   */
  const handleAddHeader = useCallback(() => {
    append({ key: '', value: '', type: 'text' });
  }, [append]);

  /**
   * 删除 Header
   */
  const handleRemoveHeader = useCallback((index: number) => {
    remove(index);
    safeSetTimeout(saveData, 100);
  }, [remove, saveData, safeSetTimeout]);

  /**
   * 添加输出映射
   */
  const handleAddOutputMapping = useCallback(() => {
    outputMappings.append({ source: '$.message', target: '', description: '' });
  }, [outputMappings]);

  /**
   * 删除输出映射
   */
  const handleRemoveOutputMapping = useCallback((index: number) => {
    outputMappings.remove(index);
    safeSetTimeout(saveData, 100);
  }, [outputMappings, saveData, safeSetTimeout]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-amber-200/50 bg-gradient-to-br from-amber-50/80 to-orange-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-amber-900 flex items-center gap-2">
            <Wifi className="h-4 w-4" />
            WebSocket 连接节点
          </h4>
          <p className="text-sm leading-relaxed text-amber-700/90 whitespace-normal break-words">
            建立 WebSocket 连接，可选择等待服务器消息或条件满足后继续流程。
          </p>
        </div>
      </div>

      {/* URL 输入 */}
      <div className="space-y-2">
        <Label htmlFor="url" className="text-sm font-medium text-foreground">
          WebSocket URL <span className="text-rose-500">*</span>
        </Label>
        <div className="relative">
          <Wifi className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <VariableInput
            id="url"
            variables={variables}
            nodeLabels={nodeLabels}
            value={watch('url') || ''}
            onValueChange={(value) => setValue('url', value, { shouldDirty: true })}
            placeholder="wss://example.com/socket"
            className="pl-9 font-mono text-sm text-left"
          />
        </div>
        {errors.url && (
          <p className="text-xs text-rose-500">{errors.url.message}</p>
        )}
        <VariableValidationWarning
          value={watch('url') || ''}
          availableVariables={availableVariableNames}
          compact
        />
      </div>

      {/* Headers 编辑器 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">请求头（可选）</Label>
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
          {fields.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">暂无自定义 Headers</p>
            </div>
          ) : (
            fields.map((field, index) => (
              <div key={field.id} className="space-y-2 p-3 rounded-md border bg-muted/30">
                <div className="flex items-center gap-2">
                  <Input
                    {...register(`headers.${index}.key`)}
                    placeholder="Header Name"
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
                <VariableInput
                  variables={variables}
                  nodeLabels={nodeLabels}
                  value={watch(`headers.${index}.value`) || ''}
                  onValueChange={(value) => setValue(`headers.${index}.value`, value, { shouldDirty: true })}
                  placeholder="Value"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <VariableValidationWarning
                  value={watch(`headers.${index}.value`) || ''}
                  availableVariables={availableVariableNames}
                  compact
                />
              </div>
            ))
          )}
        </div>
      </div>

      {/* Subprotocols */}
      <div className="space-y-2">
        <Label htmlFor="subprotocols" className="text-sm font-medium text-foreground">
          子协议（可选）
        </Label>
        <Input
          id="subprotocols"
          value={watch('subprotocols')?.join(', ') || ''}
          onChange={(e) => setValue('subprotocols', e.target.value.split(',').map(s => s.trim()).filter(Boolean), { shouldDirty: true })}
          onBlur={handleBlur}
          placeholder="protocol1, protocol2"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          多个子协议用逗号分隔
        </p>
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

      {/* 采集消息开关 */}
      <div className="flex items-center space-x-2">
        <Switch
          id="collect_message"
          checked={collectMessage || false}
          onCheckedChange={(checked) => {
            setValue('collect_message', checked, { shouldDirty: true });
            if (!checked) {
              // 关闭采集消息时，清除所有关联字段，避免 schema 验证失败
              setValue('collect_mode', undefined as unknown as WsCollectMode, { shouldDirty: true });
              setValue('collect_condition', undefined as unknown as { match_type: WsMatchType; expression: string }, { shouldDirty: true });
              setValue('collect_timeout_ms', undefined as unknown as number, { shouldDirty: true });
            }
            saveData();
          }}
        />
        <Label htmlFor="collect_message" className="text-sm font-medium text-foreground cursor-pointer">
          连接后采集消息
        </Label>
      </div>

      {/* 采集配置（仅当 collect_message 为 true 时显示） */}
      {collectMessage && (
        <>
          {/* 采集模式 */}
          <div className="space-y-2">
            <Label htmlFor="collect_mode" className="text-sm font-medium text-foreground">
              采集模式
            </Label>
            <Select
              value={collectMode || 'first_match'}
              onValueChange={(value) => {
                setValue('collect_mode', value as WsCollectMode, { shouldDirty: true });
                if (value === 'condition_match') {
                  // 切换到 condition_match 模式时，初始化 collect_condition 为默认值
                  setValue('collect_condition', { match_type: 'json_path', expression: '' }, { shouldDirty: true });
                } else {
                  // 切换到非 condition_match 模式时，清除 collect_condition，避免 schema 验证失败
                  setValue('collect_condition', undefined as unknown as { match_type: WsMatchType; expression: string }, { shouldDirty: true });
                }
                saveData();
              }}
            >
              <SelectTrigger id="collect_mode" className="font-mono">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="first_match">采集首条即停</SelectItem>
                <SelectItem value="condition_match">匹配条件即停</SelectItem>
                <SelectItem value="duration_collect">持续采集</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* 采集条件（仅当 collect_mode 为 'condition_match' 时显示） */}
          {collectMode === 'condition_match' && (
            <div className="space-y-2">
              <Label className="text-sm font-medium text-foreground">采集条件</Label>
              <div className="space-y-2 p-3 rounded-md border bg-muted/30">
                {/* 匹配类型 */}
                <div className="space-y-2">
                  <Label htmlFor="collect_condition.match_type" className="text-xs text-muted-foreground">
                    匹配类型
                  </Label>
                  <Select
                    value={watch('collect_condition.match_type') || 'json_path'}
                    onValueChange={(value) => {
                      setValue('collect_condition.match_type', value as WsMatchType, { shouldDirty: true });
                      saveData();
                    }}
                  >
                    <SelectTrigger id="collect_condition.match_type" className="font-mono text-xs h-8">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="json_path">JSONPath</SelectItem>
                      <SelectItem value="regex">正则表达式</SelectItem>
                      <SelectItem value="expression">表达式</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                {/* 匹配表达式 */}
                <div className="space-y-2">
                  <Label htmlFor="collect_condition.expression" className="text-xs text-muted-foreground">
                    匹配表达式 <span className="text-rose-500">*</span>
                  </Label>
                  <Textarea
                    id="collect_condition.expression"
                    {...register('collect_condition.expression', { required: '匹配表达式不能为空' })}
                    placeholder={watch('collect_condition.match_type') === 'expression' ? 'msg == "authenticated"' : watch('collect_condition.match_type') === 'regex' ? 'authenticated|success' : '$.msg'}
                    className={`min-h-[80px] resize-y font-mono text-xs ${
                      errors.collect_condition?.expression ? 'border-rose-500 focus-visible:ring-rose-500' : ''
                    }`}
                    rows={3}
                  />
                  {errors.collect_condition?.expression && (
                    <p className="text-xs text-rose-500">{errors.collect_condition.expression.message}</p>
                  )}
                  <VariableValidationWarning
                    value={watch('collect_condition.expression') || ''}
                    availableVariables={availableVariableNames}
                    compact
                  />
                  
                  {/* 匹配类型说明 */}
                  <div className="flex items-start gap-2 rounded-md bg-sky-50/50 border border-sky-200/50 p-3">
                    <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
                    <div className="text-xs text-sky-700/90 space-y-2">
                      <p className="font-medium text-sky-800">匹配类型说明：</p>
                      <div className="space-y-1.5">
                        {/* JSONPath */}
                        <div className="p-2 rounded bg-background/50 border border-sky-100">
                          <p className="font-medium text-sky-800">📌 JSONPath — 检查字段是否存在</p>
                          <p className="font-mono text-[11px] text-sky-600 mt-1">
                            <code className="bg-sky-100/50 px-1 rounded">$.msg</code> 检查 msg 字段是否存在
                          </p>
                          <p className="font-mono text-[11px] text-sky-600">
                            <code className="bg-sky-100/50 px-1 rounded">$.data.status</code> 检查嵌套字段
                          </p>
                          <p className="text-[10px] text-sky-500 mt-1">⚠️ 仅检查字段存在，不支持值比较</p>
                        </div>
                        {/* Regex */}
                        <div className="p-2 rounded bg-background/50 border border-sky-100">
                          <p className="font-medium text-sky-800">🔍 正则表达式 — 匹配消息文本</p>
                          <p className="font-mono text-[11px] text-sky-600 mt-1">
                            <code className="bg-sky-100/50 px-1 rounded">authenticated</code> 包含该文本
                          </p>
                          <p className="font-mono text-[11px] text-sky-600">
                            <code className="bg-sky-100/50 px-1 rounded">success|ok</code> 多选一
                          </p>
                          <p className="text-[10px] text-sky-500 mt-1">⚠️ 对整个消息文本进行正则匹配</p>
                        </div>
                        {/* Expression */}
                        <div className="p-2 rounded bg-background/50 border border-sky-100">
                          <p className="font-medium text-sky-800">🧮 表达式 — 值比较（推荐）</p>
                          <p className="font-mono text-[11px] text-sky-600 mt-1">
                            <code className="bg-sky-100/50 px-1 rounded">msg == "authenticated"</code> 字符串比较
                          </p>
                          <p className="font-mono text-[11px] text-sky-600">
                            <code className="bg-sky-100/50 px-1 rounded">code == 1</code> 数字比较
                          </p>
                          <p className="text-[10px] text-sky-500 mt-1">✅ JSON 顶层字段自动注入，无需 $. 前缀</p>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* 采集超时 */}
          <div className="space-y-2">
            <Label htmlFor="collect_timeout_ms" className="text-sm font-medium text-foreground">
              采集超时
            </Label>
            <Input
              id="collect_timeout_ms"
              type="number"
              min={1000}
              max={300000}
              {...register('collect_timeout_ms', { valueAsNumber: true })}
            />
            <p className="text-xs text-muted-foreground">毫秒 (ms)，范围 1000-300000</p>
          </div>
        </>
      )}

      {/* 输出变量映射 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <ArrowRightFromLine className="h-4 w-4 text-muted-foreground" />
            <Label className="text-sm font-medium text-foreground">输出变量映射</Label>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddOutputMapping}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            添加映射
          </Button>
        </div>

        {/* 映射列表 */}
        <div className="space-y-2">
          {outputMappings.fields.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">暂无输出映射</p>
              <p className="mt-1 text-xs text-muted-foreground/60">
                点击「添加映射」从节点执行结果的 output_data 中提取变量
              </p>
            </div>
          ) : (
            outputMappings.fields.map((field, index) => {
              const targetError = errors.output_mappings?.[index]?.target;
              const sourceError = errors.output_mappings?.[index]?.source;

              // 检查变量名是否重复
              const outputMappingsList = watch('output_mappings') || [];
              const currentTarget = outputMappingsList[index]?.target || '';
              const duplicateTargets = outputMappingsList
                .map((m, i) => (m.target && i !== index && m.target === currentTarget ? i : -1))
                .filter(i => i !== -1);
              const hasDuplicate = duplicateTargets.length > 0 && currentTarget.trim() !== '';

              // JSONPath 常用预设（根据 collect_message 状态动态变化）
              const jsonPathPresets = collectMessage
                ? [
                    { value: '$.message', label: '消息内容' },
                    { value: '$.connection_id', label: '连接 ID' },
                    { value: '$.data', label: 'data 字段' },
                    { value: '$.type', label: '消息类型' },
                  ]
                : [
                    { value: '$.connection_id', label: '连接 ID' },
                  ];

              return (
                <div key={field.id} className="space-y-2 p-3 rounded-md border bg-muted/30">
                  {/* 第一行：变量名 + 删除按钮 */}
                  <div className="flex items-center gap-2">
                    <Input
                      {...register(`output_mappings.${index}.target`)}
                      placeholder="变量名（字母、数字、下划线）"
                      className={`h-8 flex-1 font-mono text-xs ${targetError || hasDuplicate ? 'border-rose-500 focus-visible:ring-rose-500' : ''}`}
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => handleRemoveOutputMapping(index)}
                      className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>

                  {/* 第二行：源字段 */}
                  <div className="relative">
                    <Input
                      {...register(`output_mappings.${index}.source`)}
                      placeholder="源字段（如 $.message、$.connection_id）"
                      className={`h-8 w-full font-mono text-xs ${sourceError ? 'border-rose-500 focus-visible:ring-rose-500' : ''}`}
                      list={`jsonpath-presets-wsconn-${index}`}
                    />
                    <datalist id={`jsonpath-presets-wsconn-${index}`}>
                      {jsonPathPresets.map((preset) => (
                        <option key={preset.value} value={preset.value} />
                      ))}
                    </datalist>
                  </div>

                  {/* 错误提示 */}
                  {sourceError && (
                    <p className="text-xs text-rose-500">{sourceError.message}</p>
                  )}
                  {targetError && (
                    <p className="text-xs text-rose-500">{targetError.message}</p>
                  )}
                  {hasDuplicate && !targetError && (
                    <p className="text-xs text-rose-500">变量名已存在</p>
                  )}

                  {/* 第三行：说明 */}
                  <Input
                    {...register(`output_mappings.${index}.description`)}
                    placeholder="说明（可选）"
                    className="h-8 w-full text-xs"
                  />
                </div>
              );
            })
          )}
        </div>

        {/* 提示信息 */}
        {outputMappings.fields.length > 0 && (
          <div className="flex items-start gap-2 rounded-md bg-muted/30 p-2">
            <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
            <div className="text-xs text-muted-foreground space-y-1">
              <p className="font-medium text-foreground">JSONPath 语法（从节点执行结果的 <code className="rounded bg-muted px-1">output_data</code> 中提取）：</p>
              <p><code className="rounded bg-muted px-1">$.message</code> 消息内容</p>
              <p><code className="rounded bg-muted px-1">$.connection_id</code> 连接 ID</p>
              <p><code className="rounded bg-muted px-1">$.data.status</code> 嵌套字段</p>
              <p className="text-muted-foreground/70 mt-1">变量名：字母、数字、下划线，1-64 字符</p>
            </div>
          </div>
        )}
      </div>
    </form>
  );
}
