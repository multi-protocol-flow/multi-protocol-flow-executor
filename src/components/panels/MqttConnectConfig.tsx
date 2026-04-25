/**
 * MQTT Connect 节点配置组件
 * 支持连接地址、认证、遗嘱消息、MQTT 5.0 选项、TLS、用户属性
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef } from 'react';
import {
  mqttConnectConfigSchema,
  type MqttConnectConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Radio, Info, Plus, Trash2, ChevronDown, Loader2, Shield, MessageSquare } from 'lucide-react';
import type { MqttConnectConfig as MqttConnectConfigType } from '@/types';
import { testMqttConnection } from '@/services/tauri/mqtt';
import { cn } from '@/lib/utils';

interface MqttConnectConfigProps {
  /** 当前节点数据 */
  data: MqttConnectConfigType;
  /** 保存回调 */
  onSave: (data: MqttConnectConfigType) => void;
}

/**
 * 将 MqttConnectConfigType 转换为表单数据
 */
function toFormData(data: MqttConnectConfigType): MqttConnectConfigFormData {
  return {
    type: 'mqtt_connect',
    url: data.url || '',
    client_id: data.client_id || '',
    username: data.username || '',
    password: data.password || '',
    mqtt_version: data.mqtt_version || '5.0',
    clean_session: data.clean_session ?? true,
    keep_alive_secs: data.keep_alive_secs ?? 60,
    connect_timeout_ms: data.connect_timeout_ms ?? 30000,
    will_topic: data.will_topic || '',
    will_payload: data.will_payload || '',
    will_qos: data.will_qos ?? 0,
    will_retain: data.will_retain ?? false,
    use_tls: data.use_tls ?? false,
    tls_skip_verify: data.tls_skip_verify ?? false,
    session_expiry_interval: data.session_expiry_interval,
    will_delay_interval: data.will_delay_interval,
    receive_maximum: data.receive_maximum,
    topic_alias_maximum: data.topic_alias_maximum,
    max_packet_size: data.max_packet_size,
    user_properties: data.user_properties || [],
  };
}

/**
 * 将表单数据转换为 MqttConnectConfigType
 */
function fromFormData(formData: MqttConnectConfigFormData): MqttConnectConfigType {
  const userProps = formData.user_properties?.filter(pair => pair[0].trim());
  const hasWill = formData.will_topic?.trim();

  return {
    type: 'mqtt_connect',
    url: formData.url || undefined,
    client_id: formData.client_id || undefined,
    username: formData.username || undefined,
    password: formData.password || undefined,
    mqtt_version: formData.mqtt_version || undefined,
    clean_session: formData.clean_session,
    keep_alive_secs: formData.keep_alive_secs,
    connect_timeout_ms: formData.connect_timeout_ms,
    will_topic: hasWill ? formData.will_topic : undefined,
    will_payload: hasWill ? (formData.will_payload || undefined) : undefined,
    will_qos: hasWill ? formData.will_qos : undefined,
    will_retain: hasWill ? formData.will_retain : undefined,
    use_tls: formData.use_tls || undefined,
    tls_skip_verify: formData.tls_skip_verify || undefined,
    session_expiry_interval: formData.session_expiry_interval || undefined,
    will_delay_interval: formData.will_delay_interval || undefined,
    receive_maximum: formData.receive_maximum || undefined,
    topic_alias_maximum: formData.topic_alias_maximum || undefined,
    max_packet_size: formData.max_packet_size || undefined,
    user_properties: userProps && userProps.length > 0
      ? userProps
      : undefined,
  };
}

export function MqttConnectConfig({ data, onSave }: MqttConnectConfigProps) {
  const {
    register,
    handleSubmit,
    reset,
    getValues,
    setValue,
    watch,
    formState: { errors, isDirty },
  } = useForm<MqttConnectConfigFormData>({
    resolver: zodResolver(mqttConnectConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
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

  // 用户属性本地状态（从 form user_properties 同步）
  const [userProps, setUserProps] = useState<Array<[string, string]>>(
    data.user_properties || []
  );

  // 当 data.user_properties 变化时同步本地状态
  useEffect(() => {
    setUserProps(data.user_properties || []);
  }, [data.user_properties]);

  /**
   * 保存表单数据
   */
  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 MQTT Connect 配置失败:', error);
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
   * 同步用户属性到表单并保存
   */
  const syncUserProps = useCallback((newProps: Array<[string, string]>) => {
    setUserProps(newProps);
    setValue('user_properties', newProps, { shouldDirty: true });
  }, [setValue]);

  /**
   * 添加用户属性
   */
  const handleAddUserProp = useCallback(() => {
    const newProps = [...userProps, ['', ''] as [string, string]];
    syncUserProps(newProps);
    safeSetTimeout(saveData, 100);
  }, [userProps, syncUserProps, safeSetTimeout, saveData]);

  /**
   * 删除用户属性
   */
  const handleRemoveUserProp = useCallback((index: number) => {
    const newProps = userProps.filter((_, i) => i !== index);
    syncUserProps(newProps);
    safeSetTimeout(saveData, 100);
  }, [userProps, syncUserProps, safeSetTimeout, saveData]);

  /**
   * 更新用户属性条目
   */
  const handleUserPropChange = useCallback((index: number, position: 0 | 1, value: string) => {
    const newProps = userProps.map((pair, i) => {
      if (i !== index) return pair;
      const updated: [string, string] = [...pair];
      updated[position] = value;
      return updated;
    });
    syncUserProps(newProps);
  }, [userProps, syncUserProps]);

  // 可折叠区域的展开状态
  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set());

  const toggleSection = useCallback((section: string) => {
    setExpandedSections(prev => {
      const next = new Set(prev);
      if (next.has(section)) next.delete(section);
      else next.add(section);
      return next;
    });
  }, []);

  // 测试连接状态
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ success: boolean; message: string } | null>(null);

  /**
   * 测试连接
   */
  const handleTestConnection = useCallback(async () => {
    const formData = getValues();
    const url = formData.url?.trim();
    if (!url) return;

    setIsTesting(true);
    setTestResult(null);

    try {
      const result = await testMqttConnection({
        url,
        client_id: formData.client_id || undefined,
        username: formData.username || undefined,
        password: formData.password || undefined,
        use_tls: formData.use_tls,
        tls_skip_verify: formData.tls_skip_verify,
        connect_timeout_ms: formData.connect_timeout_ms,
      });

      if (result.success) {
        const info = result.broker_info;
        const version = info?.version ? ` (v${info.version})` : '';
        const clientId = info?.assigned_client_id ? ` — Client ID: ${info.assigned_client_id}` : '';
        setTestResult({ success: true, message: `连接成功${version}${clientId}` });
      } else {
        setTestResult({ success: false, message: result.error || '连接失败，未知错误' });
      }
    } catch (error) {
      setTestResult({
        success: false,
        message: typeof error === 'string' ? error : (error instanceof Error ? error.message : String(error)),
      });
    } finally {
      setIsTesting(false);
    }
  }, [getValues]);

  // 当前选中的 mqtt_version
  const currentVersion = watch('mqtt_version') || '5.0';

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-cyan-200/50 bg-gradient-to-br from-cyan-50/80 to-sky-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-cyan-900 flex items-center gap-2">
            <Radio className="h-4 w-4" />
            MQTT 连接节点
          </h4>
          <p className="text-sm leading-relaxed text-cyan-700/90 whitespace-normal break-words">
            建立 MQTT Broker 连接，支持 MQTT 5.0 和 3.1.1 协议。
          </p>
        </div>
      </div>

      {/* Broker 地址 */}
      <div className="space-y-2">
        <Label htmlFor="url" className="text-sm font-medium text-foreground">
          Broker 地址 <span className="text-rose-500">*</span>
        </Label>
        <div className="relative">
          <Radio className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <Input
            id="url"
            {...register('url')}
            placeholder="mqtt://broker.emqx.io:1883"
            className="pl-9 font-mono text-sm"
          />
        </div>
        {errors.url && (
          <p className="text-xs text-rose-500">{errors.url.message}</p>
        )}
      </div>

      {/* Client ID */}
      <div className="space-y-2">
        <Label htmlFor="client_id" className="text-sm font-medium text-foreground">
          客户端 ID
        </Label>
        <Input
          id="client_id"
          {...register('client_id')}
          placeholder="留空自动生成 UUID"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          留空时执行器将自动生成唯一客户端 ID
        </p>
      </div>

      {/* 认证信息 */}
      <div className="border rounded-lg overflow-hidden">
        <button
          type="button"
          onClick={() => toggleSection('auth')}
          className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-2">
            <Shield className="h-4 w-4 text-cyan-600" />
            <span className="text-sm font-medium text-foreground">认证（可选）</span>
          </div>
          <ChevronDown className={cn(
            "h-3.5 w-3.5 text-muted-foreground transition-transform",
            expandedSections.has('auth') && "rotate-180"
          )} />
        </button>
        {expandedSections.has('auth') && (
          <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
            <div className="space-y-2">
              <Label htmlFor="username" className="text-xs font-medium text-foreground">用户名</Label>
              <Input
                id="username"
                {...register('username')}
                placeholder="可选"
                className="h-8 text-xs font-mono"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="password" className="text-xs font-medium text-foreground">密码</Label>
              <Input
                id="password"
                type="password"
                {...register('password')}
                placeholder="可选"
                className="h-8 text-xs font-mono"
              />
            </div>
          </div>
        )}
      </div>

      {/* MQTT 版本 */}
      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">
          协议版本
        </Label>
        <Select
          value={currentVersion}
          onValueChange={(value) => {
            setValue('mqtt_version', value as '3.1.1' | '5.0', { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger className="font-mono text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="5.0">MQTT 5.0</SelectItem>
            <SelectItem value="3.1.1">MQTT 3.1.1</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Clean Session */}
      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div>
          <Label className="text-sm font-medium text-foreground">清除会话</Label>
          <p className="text-xs text-muted-foreground">连接时清除之前的会话状态</p>
        </div>
        <Switch
          checked={watch('clean_session') ?? true}
          onCheckedChange={(checked) => {
            setValue('clean_session', checked, { shouldDirty: true });
            saveData();
          }}
        />
      </div>

      {/* Keep Alive */}
      <div className="space-y-2">
        <Label htmlFor="keep_alive_secs" className="text-sm font-medium text-foreground">
          心跳保活间隔（秒）
        </Label>
        <Input
          id="keep_alive_secs"
          type="number"
          {...register('keep_alive_secs', { valueAsNumber: true })}
          placeholder="60"
          className="font-mono text-sm"
        />
      </div>

      {/* Connect Timeout */}
      <div className="space-y-2">
        <Label htmlFor="connect_timeout_ms" className="text-sm font-medium text-foreground">
          连接超时（毫秒）
        </Label>
        <Input
          id="connect_timeout_ms"
          type="number"
          {...register('connect_timeout_ms', { valueAsNumber: true })}
          placeholder="30000"
          className="font-mono text-sm"
        />
      </div>

      {/* 遗嘱消息 */}
      <div className="border rounded-lg overflow-hidden">
        <button
          type="button"
          onClick={() => toggleSection('will')}
          className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-2">
            <MessageSquare className="h-4 w-4 text-cyan-600" />
            <span className="text-sm font-medium text-foreground">遗嘱消息（可选）</span>
          </div>
          <ChevronDown className={cn(
            "h-3.5 w-3.5 text-muted-foreground transition-transform",
            expandedSections.has('will') && "rotate-180"
          )} />
        </button>
        {expandedSections.has('will') && (
          <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
            <div className="space-y-2">
              <Label htmlFor="will_topic" className="text-xs font-medium text-foreground">遗嘱主题</Label>
              <Input
                id="will_topic"
                {...register('will_topic')}
                placeholder="device/status"
                className="h-8 text-xs font-mono"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="will_payload" className="text-xs font-medium text-foreground">遗嘱载荷</Label>
              <Input
                id="will_payload"
                {...register('will_payload')}
                placeholder='{"status":"offline"}'
                className="h-8 text-xs font-mono"
              />
            </div>
            <div className="space-y-2">
              <Label className="text-xs font-medium text-foreground">遗嘱 QoS</Label>
              <Select
                value={String(watch('will_qos') ?? 0)}
                onValueChange={(value) => {
                  setValue('will_qos', Number(value), { shouldDirty: true });
                  saveData();
                }}
              >
                <SelectTrigger className="h-8 text-xs font-mono">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="0">QoS 0 — 最多一次</SelectItem>
                  <SelectItem value="1">QoS 1 — 至少一次</SelectItem>
                  <SelectItem value="2">QoS 2 — 恰好一次</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-center justify-between">
              <Label className="text-xs font-medium text-foreground">遗嘱保留</Label>
              <Switch
                checked={watch('will_retain') ?? false}
                onCheckedChange={(checked) => {
                  setValue('will_retain', checked, { shouldDirty: true });
                  saveData();
                }}
              />
            </div>
          </div>
        )}
      </div>

      {/* TLS 选项 */}
      <div className="border rounded-lg overflow-hidden">
        <button
          type="button"
          onClick={() => toggleSection('tls')}
          className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-2">
            <Shield className="h-4 w-4 text-cyan-600" />
            <span className="text-sm font-medium text-foreground">TLS 选项</span>
          </div>
          <ChevronDown className={cn(
            "h-3.5 w-3.5 text-muted-foreground transition-transform",
            expandedSections.has('tls') && "rotate-180"
          )} />
        </button>
        {expandedSections.has('tls') && (
          <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
            <div className="flex items-center justify-between">
              <div>
                <Label className="text-xs font-medium text-foreground">启用 TLS</Label>
                <p className="text-[10px] text-muted-foreground">使用 TLS 加密连接 (mqtts://)</p>
              </div>
              <Switch
                checked={watch('use_tls') ?? false}
                onCheckedChange={(checked) => {
                  setValue('use_tls', checked, { shouldDirty: true });
                  saveData();
                }}
              />
            </div>
            <div className="flex items-center justify-between">
              <div>
                <Label className="text-xs font-medium text-foreground">跳过证书验证</Label>
                <p className="text-[10px] text-muted-foreground">不验证服务器 TLS 证书</p>
              </div>
              <Switch
                checked={watch('tls_skip_verify') ?? false}
                onCheckedChange={(checked) => {
                  setValue('tls_skip_verify', checked, { shouldDirty: true });
                  saveData();
                }}
              />
            </div>
          </div>
        )}
      </div>

      {/* MQTT 5.0 高级选项 */}
      {currentVersion === '5.0' && (
        <div className="border rounded-lg overflow-hidden">
          <button
            type="button"
            onClick={() => toggleSection('mqtt5')}
            className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors"
          >
            <div className="flex items-center gap-2">
              <Info className="h-4 w-4 text-cyan-600" />
              <span className="text-sm font-medium text-foreground">MQTT 5.0 高级选项</span>
            </div>
            <ChevronDown className={cn(
              "h-3.5 w-3.5 text-muted-foreground transition-transform",
              expandedSections.has('mqtt5') && "rotate-180"
            )} />
          </button>
          {expandedSections.has('mqtt5') && (
            <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
              <div className="space-y-2">
                <Label htmlFor="session_expiry_interval" className="text-xs font-medium text-foreground">会话过期间隔（秒）</Label>
                <Input
                  id="session_expiry_interval"
                  type="number"
                  {...register('session_expiry_interval', { valueAsNumber: true })}
                  placeholder="可选"
                  className="h-8 text-xs font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="will_delay_interval" className="text-xs font-medium text-foreground">遗嘱延迟间隔（秒）</Label>
                <Input
                  id="will_delay_interval"
                  type="number"
                  {...register('will_delay_interval', { valueAsNumber: true })}
                  placeholder="可选"
                  className="h-8 text-xs font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="receive_maximum" className="text-xs font-medium text-foreground">接收最大值</Label>
                <Input
                  id="receive_maximum"
                  type="number"
                  {...register('receive_maximum', { valueAsNumber: true })}
                  placeholder="可选"
                  className="h-8 text-xs font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="topic_alias_maximum" className="text-xs font-medium text-foreground">主题别名最大值</Label>
                <Input
                  id="topic_alias_maximum"
                  type="number"
                  {...register('topic_alias_maximum', { valueAsNumber: true })}
                  placeholder="可选"
                  className="h-8 text-xs font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="max_packet_size" className="text-xs font-medium text-foreground">最大报文大小（字节）</Label>
                <Input
                  id="max_packet_size"
                  type="number"
                  {...register('max_packet_size', { valueAsNumber: true })}
                  placeholder="可选"
                  className="h-8 text-xs font-mono"
                />
              </div>
            </div>
          )}
        </div>
      )}

      {/* 用户属性 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">用户属性（可选）</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddUserProp}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            添加
          </Button>
        </div>
        <div className="space-y-2">
          {userProps.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">暂无用户属性</p>
            </div>
          ) : (
            userProps.map((pair, index) => (
              <div key={index} className="flex items-center gap-2 p-2 rounded-md border bg-muted/30">
                <Input
                  value={pair[0]}
                  onChange={(e) => handleUserPropChange(index, 0, e.target.value)}
                  onBlur={() => safeSetTimeout(saveData, 100)}
                  placeholder="Key"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Input
                  value={pair[1]}
                  onChange={(e) => handleUserPropChange(index, 1, e.target.value)}
                  onBlur={() => safeSetTimeout(saveData, 100)}
                  placeholder="Value"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  onClick={() => handleRemoveUserProp(index)}
                  className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            ))
          )}
        </div>
      </div>

      {/* 测试连接 */}
      <div className="space-y-2">
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="w-full gap-2"
          disabled={isTesting || !watch('url')?.trim()}
          onClick={handleTestConnection}
        >
          {isTesting ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Radio className="h-4 w-4" />
          )}
          {isTesting ? '正在测试连接...' : '测试连接'}
        </Button>

        {testResult && (
          <div className={cn(
            "rounded-md border px-3 py-2 text-xs",
            testResult.success
              ? "border-emerald-200 dark:border-emerald-800/60 bg-emerald-50 dark:bg-emerald-950/30 text-emerald-700 dark:text-emerald-400"
              : "border-rose-200 dark:border-rose-800/60 bg-rose-50 dark:bg-rose-950/30 text-rose-600 dark:text-rose-400"
          )}>
            {testResult.message}
          </div>
        )}
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">MQTT 连接说明：</p>
          <p>支持 MQTT 5.0 和 3.1.1 协议版本</p>
          <p>遗嘱消息在连接异常断开时由 Broker 自动发布</p>
          <p>MQTT 5.0 高级选项仅在协议版本为 5.0 时可用</p>
        </div>
      </div>
    </form>
  );
}
