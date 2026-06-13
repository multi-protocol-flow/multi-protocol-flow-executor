/**
 * WebSocket Connect 节点配置组件
 * 支持 URL、Headers、Subprotocols、等待配置
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef } from 'react';
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
import { Plus, Trash2, Wifi, Info, AlertTriangle } from 'lucide-react';
import type { WsConnectConfig as WsConnectConfigType, FormField } from '@/types';
import { useFlowVariables } from '@/hooks';
import { getLL } from '@/i18n/i18n-lazy';

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
    use_tls: data.use_tls,
    tls_skip_verify: data.tls_skip_verify,
    ping_interval_ms: data.ping_interval_ms,
    ping_timeout_ms: data.ping_timeout_ms,
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
    use_tls: formData.use_tls || undefined,
    tls_skip_verify: formData.use_tls ? (formData.tls_skip_verify || undefined) : undefined,
    ping_interval_ms: formData.ping_interval_ms || undefined,
    ping_timeout_ms: formData.ping_timeout_ms || undefined,
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

  // 获取流程变量
  const { variables, availableVariableNames, nodeLabels } = useFlowVariables();

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
  const useTls = watch('use_tls');
  const pingIntervalMs = watch('ping_interval_ms');
  const pingTimeoutMs = watch('ping_timeout_ms');

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

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-amber-200/50 bg-gradient-to-br from-amber-50/80 to-orange-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-amber-900 flex items-center gap-2">
            <Wifi className="h-4 w-4" />
            {getLL().panels.wsConnect.title()}
          </h4>
          <p className="text-sm leading-relaxed text-amber-700/90 whitespace-normal break-words">
            {getLL().panels.wsConnect.description()}
          </p>
        </div>
      </div>

      {/* URL 输入 */}
      <div className="space-y-2">
        <Label htmlFor="url" className="text-sm font-medium text-foreground">
          {getLL().panels.wsConnect.wsUrl()} <span className="text-rose-500">*</span>
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
          <Label className="text-sm font-medium text-foreground">{getLL().panels.wsConnect.headers_label()}</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddHeader}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            {getLL().panels.add()}
          </Button>
        </div>
        <div className="space-y-2">
          {fields.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">{getLL().panels.wsConnect.noHeaders()}</p>
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
          {getLL().panels.wsConnect.subprotocolsOptional()}
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
          {getLL().panels.wsConnect.subprotocolsHelp()}
        </p>
      </div>

      {/* 连接超时 */}
      <div className="space-y-2">
        <Label htmlFor="connect_timeout_ms" className="text-sm font-medium text-foreground">
          {getLL().panels.wsConnect.connectTimeout()}
        </Label>
        <Input
          id="connect_timeout_ms"
          type="number"
          min={1000}
          max={300000}
          {...register('connect_timeout_ms', { valueAsNumber: true })}
        />
        <p className="text-xs text-muted-foreground">{getLL().panels.wsConnect.timeoutRange()}</p>
      </div>

      {/* TLS 配置 */}
      <div className="space-y-3 rounded-md border bg-muted/30 p-3">
        <div className="flex items-center space-x-2">
          <Switch
            id="use_tls"
            checked={useTls || false}
            onCheckedChange={(checked) => {
              setValue('use_tls', checked, { shouldDirty: true });
              if (!checked) {
                setValue('tls_skip_verify', false, { shouldDirty: true });
              }
              saveData();
            }}
          />
          <Label htmlFor="use_tls" className="text-sm font-medium text-foreground cursor-pointer">
            {getLL().panels.wsConnect.enableTls()}
          </Label>
        </div>

        {useTls && (
          <>
            <div className="flex items-center space-x-2">
              <Switch
                id="tls_skip_verify"
                checked={watch('tls_skip_verify') || false}
                onCheckedChange={(checked) => {
                  setValue('tls_skip_verify', checked, { shouldDirty: true });
                  saveData();
                }}
              />
              <Label htmlFor="tls_skip_verify" className="text-sm font-medium text-foreground cursor-pointer">
                {getLL().panels.wsConnect.skipCertVerify()}
              </Label>
            </div>
            {watch('tls_skip_verify') && (
              <div className="flex items-start gap-2 rounded-md border border-amber-200 bg-amber-50 p-2.5 dark:border-amber-800/60 dark:bg-amber-950/30">
                <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-500" />
                <p className="text-xs leading-relaxed text-amber-700 dark:text-amber-400">
                  {getLL().panels.wsConnect.skipCertVerifyWarning()}
                </p>
              </div>
            )}
          </>
        )}
      </div>

      {/* 心跳配置 */}
      <div className="space-y-3 rounded-md border bg-muted/30 p-3">
        <div className="space-y-2">
          <div className="flex items-center space-x-2">
            <Switch
              id="ping_enabled"
              checked={!!(pingIntervalMs || pingTimeoutMs)}
              onCheckedChange={(checked) => {
                if (checked) {
                  setValue('ping_interval_ms', 30000, { shouldDirty: true });
                  setValue('ping_timeout_ms', 10000, { shouldDirty: true });
                } else {
                  setValue('ping_interval_ms', undefined as unknown as number, { shouldDirty: true });
                  setValue('ping_timeout_ms', undefined as unknown as number, { shouldDirty: true });
                }
                saveData();
              }}
            />
            <Label htmlFor="ping_enabled" className="text-sm font-medium text-foreground cursor-pointer">
              {getLL().panels.wsConnect.enablePing()}
            </Label>
          </div>

          {(pingIntervalMs || pingTimeoutMs) && (
            <>
              <div className="space-y-2">
                <Label htmlFor="ping_interval_ms" className="text-sm font-medium text-foreground">
                  {getLL().panels.wsConnect.pingInterval()}
                </Label>
                <Input
                  id="ping_interval_ms"
                  type="number"
                  min={1000}
                  max={300000}
                  {...register('ping_interval_ms', { valueAsNumber: true })}
                />
                <p className="text-xs text-muted-foreground">{getLL().panels.wsConnect.timeoutRange()}</p>
              </div>
              <div className="space-y-2">
                <Label htmlFor="ping_timeout_ms" className="text-sm font-medium text-foreground">
                  {getLL().panels.wsConnect.pingTimeout()}
                </Label>
                <Input
                  id="ping_timeout_ms"
                  type="number"
                  min={1000}
                  max={300000}
                  {...register('ping_timeout_ms', { valueAsNumber: true })}
                />
                <p className="text-xs text-muted-foreground">{getLL().panels.wsConnect.timeoutRange()}</p>
              </div>
            </>
          )}
        </div>
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
          {getLL().panels.wsConnect.collectAfterConnect()}
        </Label>
      </div>

      {/* 采集配置（仅当 collect_message 为 true 时显示） */}
      {collectMessage && (
        <>
          {/* 采集模式 */}
          <div className="space-y-2">
            <Label htmlFor="collect_mode" className="text-sm font-medium text-foreground">
              {getLL().panels.wsConnect.collectMode()}
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
                <SelectItem value="first_match">{getLL().panels.wsConnect.collectFirstStop_option()}</SelectItem>
                <SelectItem value="condition_match">{getLL().panels.wsConnect.matchConditionStop_option()}</SelectItem>
                <SelectItem value="duration_collect">{getLL().panels.wsConnect.continuousCollect_option()}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* 采集条件（仅当 collect_mode 为 'condition_match' 时显示） */}
          {collectMode === 'condition_match' && (
            <div className="space-y-2">
              <Label className="text-sm font-medium text-foreground">{getLL().panels.wsConnect.condition_sectionTitle()}</Label>
              <div className="space-y-2 p-3 rounded-md border bg-muted/30">
                {/* 匹配类型 */}
                <div className="space-y-2">
                  <Label htmlFor="collect_condition.match_type" className="text-xs text-muted-foreground">
                    {getLL().panels.wsConnect.matchType()}
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
                      <SelectItem value="regex">{getLL().panels.wsConnect.regex_option()}</SelectItem>
                      <SelectItem value="expression">{getLL().panels.wsConnect.expression_option()}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                {/* 匹配表达式 */}
                <div className="space-y-2">
                  <Label htmlFor="collect_condition.expression" className="text-xs text-muted-foreground">
                    {getLL().panels.wsConnect.matchExpression()} <span className="text-rose-500">*</span>
                  </Label>
                  <Textarea
                    id="collect_condition.expression"
                    {...register('collect_condition.expression', { required: getLL().panels.wsConnect.matchExpressionRequired() })}
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
                      <p className="font-medium text-sky-800">{getLL().panels.wsConnect.help_matchType_title()}</p>
                      <div className="space-y-1.5">
                        {/* JSONPath */}
                        <div className="p-2 rounded bg-background/50 border border-sky-100">
                          <p className="font-medium text-sky-800">{getLL().panels.wsConnect.jsonpath_title()}</p>
                          <p className="font-mono text-[11px] text-sky-600 mt-1">{getLL().panels.wsConnect.jsonpath_line1()}</p>
                          <p className="font-mono text-[11px] text-sky-600">{getLL().panels.wsConnect.jsonpath_line2()}</p>
                          <p className="text-[10px] text-sky-500 mt-1">{getLL().panels.wsConnect.jsonpath_warning()}</p>
                        </div>
                        {/* Regex */}
                        <div className="p-2 rounded bg-background/50 border border-sky-100">
                          <p className="font-medium text-sky-800">{getLL().panels.wsConnect.regex_title()}</p>
                          <p className="font-mono text-[11px] text-sky-600 mt-1">{getLL().panels.wsConnect.regex_line1()}</p>
                          <p className="font-mono text-[11px] text-sky-600">{getLL().panels.wsConnect.regex_line2()}</p>
                          <p className="text-[10px] text-sky-500 mt-1">{getLL().panels.wsConnect.regex_warning()}</p>
                        </div>
                        {/* Expression */}
                        <div className="p-2 rounded bg-background/50 border border-sky-100">
                          <p className="font-medium text-sky-800">{getLL().panels.wsConnect.expression_title()}</p>
                          <p className="font-mono text-[11px] text-sky-600 mt-1">{getLL().panels.wsConnect.expression_line1()}</p>
                          <p className="font-mono text-[11px] text-sky-600">{getLL().panels.wsConnect.expression_line2()}</p>
                          <p className="text-[10px] text-sky-500 mt-1">{getLL().panels.wsConnect.expression_note()}</p>
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
              {getLL().panels.wsConnect.collectTimeout()}
            </Label>
            <Input
              id="collect_timeout_ms"
              type="number"
              min={1000}
              max={300000}
              {...register('collect_timeout_ms', { valueAsNumber: true })}
            />
            <p className="text-xs text-muted-foreground">{getLL().panels.wsConnect.timeoutRange()}</p>
          </div>
        </>
      )}
    </form>
  );
}
