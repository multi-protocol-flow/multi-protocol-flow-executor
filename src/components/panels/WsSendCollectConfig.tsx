/**
 * WebSocket Send+Collect 节点配置组件
 * 支持消息发送、采集响应配置
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo } from 'react';
import {
  wsSendCollectConfigSchema,
  type WsSendCollectConfigFormData,
  type WsCollectMode,
  type WsMatchType,
  type WsMessageType,
} from './schemas';
import { Input } from '@/components/ui/input';
import { VariableInput } from '@/components/ui/VariableInput';
import { VariableValidationWarning } from '@/components/ui/VariableValidationWarning';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { Plus, Trash2, Send, Info, ArrowRightFromLine } from 'lucide-react';
import type { WsSendCollectConfig as WsSendCollectConfigType } from '@/types';
import type { Variable } from '@/types/variable';
import { useFlowStore } from '@/stores';
import { isEntryConfig, isHttpConfig, isWsConnectConfig, isWsSendCollectConfig } from '@/types/node';

interface WsSendCollectConfigProps {
  /** 当前节点数据 */
  data: WsSendCollectConfigType;
  /** 保存回调 */
  onSave: (data: WsSendCollectConfigType) => void;
}

/**
 * 将 WsSendCollectConfigType 转换为表单数据
 */
function toFormData(data: WsSendCollectConfigType): WsSendCollectConfigFormData {
  return {
    type: 'ws_send_collect',
    connection_id: data.connection_id || '',
    message: data.message || '',
    message_type: data.message_type || 'text',
    send_timeout_ms: data.send_timeout_ms ?? 30000,
    collect_response: data.collect_response ?? true,
    collect_mode: data.collect_mode || 'first_match',
    collect_condition: data.collect_condition,
    collect_timeout_ms: data.collect_timeout_ms ?? 30000,
    output_mappings: data.output_mappings || [],
  };
}

/**
 * 将表单数据转换为 WsSendCollectConfigType
 */
function fromFormData(formData: WsSendCollectConfigFormData): WsSendCollectConfigType {
  return {
    type: 'ws_send_collect',
    connection_id: formData.connection_id,
    message: formData.message,
    message_type: formData.message_type,
    send_timeout_ms: formData.send_timeout_ms,
    collect_response: formData.collect_response,
    collect_mode: formData.collect_response ? formData.collect_mode : undefined,
    collect_condition: formData.collect_response && formData.collect_mode === 'condition_match' ? formData.collect_condition : undefined,
    collect_timeout_ms: formData.collect_response ? formData.collect_timeout_ms : undefined,
    output_mappings: formData.output_mappings && formData.output_mappings.length > 0 ? formData.output_mappings : undefined,
  };
}

export function WsSendCollectConfig({ data, onSave }: WsSendCollectConfigProps) {
  const {
    register,
    handleSubmit,
    control,
    watch,
    setValue,
    reset,
    formState: { errors },
  } = useForm<WsSendCollectConfigFormData>({
    resolver: zodResolver(wsSendCollectConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
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
      // WS Send+Collect 节点：提取 output_mappings
      if (isWsSendCollectConfig(node.data) && node.data.output_mappings) {
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

  // 提取可用的 WebSocket 连接（从 ws_connect 节点）
  const availableConnectionIds = useMemo(() => {
    if (!flow) return [];
    const connectionIds: Array<{ uuid: string; name: string }> = [];
    flow.nodes.forEach(node => {
      if (isWsConnectConfig(node.data)) {
        connectionIds.push({
          uuid: node.uuid,
          name: node.name,
        });
      }
    });
    return connectionIds;
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
  const collectResponse = watch('collect_response');
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
        console.error('保存 WebSocket Send+Collect 配置失败:', error);
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
   * 添加输出映射
   */
  const handleAddOutputMapping = useCallback(() => {
    outputMappings.append({ source: '$.response', target: '', description: '' });
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
      <div className="rounded-lg border border-teal-200/50 bg-gradient-to-br from-teal-50/80 to-cyan-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-teal-900 flex items-center gap-2">
            <Send className="h-4 w-4" />
            WebSocket 发送+采集节点
          </h4>
          <p className="text-sm leading-relaxed text-teal-700/90 dark:text-teal-300/90 whitespace-normal break-words">
            通过已建立的 WebSocket 连接发送消息，可选择采集响应或条件满足后继续流程。
          </p>
        </div>
      </div>

      {/* 连接 ID */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          连接 ID <span className="text-rose-500">*</span>
        </Label>
        {availableConnectionIds.length > 0 ? (
          <Select
            value={watch('connection_id') || ''}
            onValueChange={(value) => {
              setValue('connection_id', value);
              saveData();
            }}
          >
            <SelectTrigger id="connection_id" className="font-mono">
              <SelectValue placeholder="选择连接" />
            </SelectTrigger>
            <SelectContent>
              {availableConnectionIds.map((conn) => (
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
          <VariableInput
            id="connection_id"
            variables={variables}
            nodeLabels={nodeLabels}
            value={watch('connection_id') || ''}
            onValueChange={(value) => setValue('connection_id', value)}
            placeholder="ws_connection_id"
            className="font-mono text-sm"
          />
        )}
        {errors.connection_id && (
          <p className="text-xs text-rose-500">{errors.connection_id.message}</p>
        )}
        <VariableValidationWarning
          value={watch('connection_id') || ''}
          availableVariables={availableVariableNames}
          compact
        />
        {availableConnectionIds.length === 0 && (
          <p className="text-xs text-muted-foreground">
            未检测到 WS Connect 节点，请手动输入连接 ID 或使用变量引用
          </p>
        )}
      </div>

      {/* 消息内容 */}
      <div className="space-y-2">
        <Label htmlFor="message" className="text-sm font-medium text-foreground">
          消息内容 <span className="text-rose-500">*</span>
        </Label>
        <Textarea
          id="message"
          {...register('message')}
          placeholder='{"type": "ping", "data": "hello"}'
          className="min-h-[100px] resize-y font-mono text-sm"
          rows={4}
        />
        {errors.message && (
          <p className="text-xs text-rose-500">{errors.message.message}</p>
        )}
        <VariableValidationWarning
          value={watch('message') || ''}
          availableVariables={availableVariableNames}
        />
        <p className="text-xs text-muted-foreground">
          支持使用 <code className="rounded bg-muted px-1">{'{{变量名}}'}</code> 引用变量
        </p>
      </div>

      {/* 消息类型 */}
      <div className="space-y-2">
        <Label htmlFor="message_type" className="text-sm font-medium text-foreground">
          消息类型
        </Label>
        <Select
          value={watch('message_type') || 'text'}
          onValueChange={(value) => {
            setValue('message_type', value as WsMessageType);
            saveData();
          }}
        >
          <SelectTrigger id="message_type" className="font-mono">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="text">文本 (Text)</SelectItem>
            <SelectItem value="binary">二进制 (Binary)</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* 发送超时 */}
      <div className="space-y-2">
        <Label htmlFor="send_timeout_ms" className="text-sm font-medium text-foreground">
          发送超时
        </Label>
        <Input
          id="send_timeout_ms"
          type="number"
          min={1000}
          max={300000}
          {...register('send_timeout_ms', { valueAsNumber: true })}
        />
        <p className="text-xs text-muted-foreground">毫秒 (ms)，范围 1000-300000</p>
      </div>

      {/* 采集响应开关 */}
      <div className="flex items-center space-x-2">
        <Switch
          id="collect_response"
          checked={collectResponse || false}
          onCheckedChange={(checked) => {
            setValue('collect_response', checked);
            if (!checked) {
              // 关闭采集响应时，清除所有关联字段，避免 schema 验证失败
              setValue('collect_mode', undefined as unknown as WsCollectMode);
              setValue('collect_condition', undefined as unknown as { match_type: WsMatchType; expression: string });
              setValue('collect_timeout_ms', undefined as unknown as number);
            }
            saveData();
          }}
        />
        <Label htmlFor="collect_response" className="text-sm font-medium text-foreground cursor-pointer">
          发送后采集响应
        </Label>
      </div>

      {/* 采集配置（仅当 collect_response 为 true 时显示） */}
      {collectResponse && (
        <>
          {/* 采集模式 */}
          <div className="space-y-2">
            <Label htmlFor="collect_mode" className="text-sm font-medium text-foreground">
              采集模式
            </Label>
            <Select
              value={collectMode || 'first_match'}
                onValueChange={(value) => {
                  setValue('collect_mode', value as WsCollectMode);
                  if (value === 'condition_match') {
                    // 切换到 condition_match 模式时，初始化 collect_condition 为默认值
                    setValue('collect_condition', { match_type: 'json_path', expression: '' });
                  } else {
                    // 切换到非 condition_match 模式时，清除 collect_condition，避免 schema 验证失败
                    setValue('collect_condition', undefined as unknown as { match_type: WsMatchType; expression: string });
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
                      setValue('collect_condition.match_type', value as WsMatchType);
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
                  <div className="flex items-start gap-2 rounded-md bg-sky-50/50 dark:bg-sky-950/30 border border-sky-200/50 dark:border-sky-800/50 p-3">
                    <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500 dark:text-sky-400" />
                    <div className="text-xs text-sky-700/90 dark:text-sky-300/90 space-y-2">
                      <p className="font-medium text-sky-800 dark:text-sky-200">匹配类型说明：</p>
                      <div className="space-y-1.5">
                        {/* JSONPath */}
                        <div className="p-2 rounded bg-background/50 border border-sky-100 dark:border-sky-900/50">
                          <p className="font-medium text-sky-800 dark:text-sky-200">📌 JSONPath — 检查字段是否存在</p>
                          <p className="font-mono text-[11px] text-sky-600 dark:text-sky-400 mt-1">
                            <code className="bg-sky-100/50 dark:bg-sky-900/50 px-1 rounded">$.msg</code> 检查 msg 字段是否存在
                          </p>
                          <p className="font-mono text-[11px] text-sky-600 dark:text-sky-400">
                            <code className="bg-sky-100/50 dark:bg-sky-900/50 px-1 rounded">$.data.status</code> 检查嵌套字段
                          </p>
                          <p className="text-[10px] text-sky-500 dark:text-sky-500 mt-1">⚠️ 仅检查字段存在，不支持值比较</p>
                        </div>
                        {/* Regex */}
                        <div className="p-2 rounded bg-background/50 border border-sky-100 dark:border-sky-900/50">
                          <p className="font-medium text-sky-800 dark:text-sky-200">🔍 正则表达式 — 匹配消息文本</p>
                          <p className="font-mono text-[11px] text-sky-600 dark:text-sky-400 mt-1">
                            <code className="bg-sky-100/50 dark:bg-sky-900/50 px-1 rounded">authenticated</code> 包含该文本
                          </p>
                          <p className="font-mono text-[11px] text-sky-600 dark:text-sky-400">
                            <code className="bg-sky-100/50 dark:bg-sky-900/50 px-1 rounded">success|ok</code> 多选一
                          </p>
                          <p className="text-[10px] text-sky-500 dark:text-sky-500 mt-1">⚠️ 对整个消息文本进行正则匹配</p>
                        </div>
                        {/* Expression */}
                        <div className="p-2 rounded bg-background/50 border border-sky-100 dark:border-sky-900/50">
                          <p className="font-medium text-sky-800 dark:text-sky-200">🧮 表达式 — 值比较（推荐）</p>
                          <p className="font-mono text-[11px] text-sky-600 dark:text-sky-400 mt-1">
                            <code className="bg-sky-100/50 dark:bg-sky-900/50 px-1 rounded">msg == "authenticated"</code> 字符串比较
                          </p>
                          <p className="font-mono text-[11px] text-sky-600 dark:text-sky-400">
                            <code className="bg-sky-100/50 dark:bg-sky-900/50 px-1 rounded">code == 1</code> 数字比较
                          </p>
                          <p className="text-[10px] text-sky-500 dark:text-sky-500 mt-1">✅ JSON 顶层字段自动注入，无需 $. 前缀</p>
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

              // JSONPath 常用预设（根据 collect_response 状态动态变化）
              const jsonPathPresets = collectResponse
                ? [
                    { value: '$.response', label: '响应消息' },
                    { value: '$.data', label: 'data 字段' },
                    { value: '$.type', label: '消息类型' },
                    { value: '$.status', label: '状态字段' },
                  ]
                : [
                    { value: '$.sent', label: '发送状态' },
                    { value: '$.timestamp', label: '发送时间' },
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
                      placeholder="源字段（如 $.response、$.data.status）"
                      className={`h-8 w-full font-mono text-xs ${sourceError ? 'border-rose-500 focus-visible:ring-rose-500' : ''}`}
                      list={`jsonpath-presets-wssend-${index}`}
                    />
                    <datalist id={`jsonpath-presets-wssend-${index}`}>
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
              <p className="font-medium text-foreground">JSONPath 语法：</p>
              <p><code className="rounded bg-muted px-1">$.response</code> 响应消息</p>
              <p><code className="rounded bg-muted px-1">$.data.status</code> 嵌套字段</p>
              <p><code className="rounded bg-muted px-1">$.items[0]</code> 数组索引</p>
              <p className="text-muted-foreground/70 mt-1">变量名：字母、数字、下划线，1-64 字符</p>
            </div>
          </div>
        )}
      </div>
    </form>
  );
}
