/**
 * WebSocket Connect 节点配置组件
 * 支持 URL、Headers、Subprotocols、等待配置
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo } from 'react';
import {
  wsConnectConfigSchema,
  type WsConnectConfigFormData,
  type WsWaitMode,
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
    wait_for_message: data.wait_for_message || false,
    wait_mode: data.wait_mode || 'none',
    wait_condition: data.wait_condition,
    message_timeout_ms: data.message_timeout_ms ?? 30000,
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
    wait_for_message: formData.wait_for_message,
    wait_mode: formData.wait_for_message ? formData.wait_mode : undefined,
    wait_condition: formData.wait_for_message && formData.wait_mode === 'condition' ? formData.wait_condition : undefined,
    message_timeout_ms: formData.wait_for_message ? formData.message_timeout_ms : undefined,
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
    formState: { errors },
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

  // 监听表单值变化
  const waitForMessage = watch('wait_for_message');
  const waitMode = watch('wait_mode');

  // 当外部数据变化时，重置表单
  useEffect(() => {
    const formData = toFormData(data);
    setValue('url', formData.url);
    setValue('subprotocols', formData.subprotocols || []);
    setValue('wait_for_message', formData.wait_for_message);
    setValue('wait_mode', formData.wait_mode);
  }, [data, setValue]);

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
    setTimeout(saveData, 100);
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
    setTimeout(saveData, 100);
  }, [remove, saveData]);

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
    setTimeout(saveData, 100);
  }, [outputMappings, saveData]);

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
        <Label htmlFor="url" className="text-sm font-medium text-black">
          WebSocket URL <span className="text-rose-500">*</span>
        </Label>
        <div className="relative">
          <Wifi className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <VariableInput
            id="url"
            variables={variables}
            nodeLabels={nodeLabels}
            value={watch('url') || ''}
            onValueChange={(value) => setValue('url', value)}
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
          <Label className="text-sm font-medium text-black">请求头（可选）</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddHeader}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-black"
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
                  onValueChange={(value) => setValue(`headers.${index}.value`, value)}
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
        <Label htmlFor="subprotocols" className="text-sm font-medium text-black">
          子协议（可选）
        </Label>
        <Input
          id="subprotocols"
          value={watch('subprotocols')?.join(', ') || ''}
          onChange={(e) => setValue('subprotocols', e.target.value.split(',').map(s => s.trim()).filter(Boolean))}
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
        <Label htmlFor="connect_timeout_ms" className="text-sm font-medium text-black">
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

      {/* 等待消息开关 */}
      <div className="flex items-center space-x-2">
        <Switch
          id="wait_for_message"
          checked={waitForMessage || false}
          onCheckedChange={(checked) => {
            setValue('wait_for_message', checked);
            if (!checked) {
              setValue('wait_mode', 'none');
            }
            saveData();
          }}
        />
        <Label htmlFor="wait_for_message" className="text-sm font-medium text-black cursor-pointer">
          连接后等待消息
        </Label>
      </div>

      {/* 等待配置（仅当 wait_for_message 为 true 时显示） */}
      {waitForMessage && (
        <>
          {/* 等待模式 */}
          <div className="space-y-2">
            <Label htmlFor="wait_mode" className="text-sm font-medium text-black">
              等待模式
            </Label>
            <Select
              value={waitMode || 'none'}
              onValueChange={(value) => {
                setValue('wait_mode', value as WsWaitMode);
                saveData();
              }}
            >
              <SelectTrigger id="wait_mode" className="font-mono">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="none">不等待</SelectItem>
                <SelectItem value="first_message">等待第一条消息</SelectItem>
                <SelectItem value="condition">等待条件匹配</SelectItem>
                <SelectItem value="duration">等待指定时长</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* 等待条件（仅当 wait_mode 为 'condition' 时显示） */}
          {waitMode === 'condition' && (
            <div className="space-y-2">
              <Label className="text-sm font-medium text-black">匹配条件</Label>
              <div className="space-y-2 p-3 rounded-md border bg-muted/30">
                {/* 匹配类型 */}
                <div className="space-y-2">
                  <Label htmlFor="wait_condition.match_type" className="text-xs text-muted-foreground">
                    匹配类型
                  </Label>
                  <Select
                    value={watch('wait_condition.match_type') || 'json_path'}
                    onValueChange={(value) => {
                      setValue('wait_condition.match_type', value as WsMatchType);
                      saveData();
                    }}
                  >
                    <SelectTrigger id="wait_condition.match_type" className="font-mono text-xs h-8">
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
                  <Label htmlFor="wait_condition.expression" className="text-xs text-muted-foreground">
                    匹配表达式
                  </Label>
                  <Textarea
                    id="wait_condition.expression"
                    {...register('wait_condition.expression')}
                    placeholder="$.data.status == 'ready'"
                    className="min-h-[80px] resize-y font-mono text-xs"
                    rows={3}
                  />
                  <VariableValidationWarning
                    value={watch('wait_condition.expression') || ''}
                    availableVariables={availableVariableNames}
                    compact
                  />
                </div>
              </div>
            </div>
          )}

          {/* 消息超时 */}
          <div className="space-y-2">
            <Label htmlFor="message_timeout_ms" className="text-sm font-medium text-black">
              消息超时
            </Label>
            <Input
              id="message_timeout_ms"
              type="number"
              min={1000}
              max={300000}
              {...register('message_timeout_ms', { valueAsNumber: true })}
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
            <Label className="text-sm font-medium text-black">输出变量映射</Label>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddOutputMapping}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-black"
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
                点击「添加映射」配置变量提取规则
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

              // JSONPath 常用预设
              const jsonPathPresets = [
                { value: '$.message', label: '消息内容' },
                { value: '$.connection_id', label: '连接 ID' },
                { value: '$.data', label: 'data 字段' },
                { value: '$.type', label: '消息类型' },
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
              <p className="font-medium text-black">JSONPath 语法：</p>
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
