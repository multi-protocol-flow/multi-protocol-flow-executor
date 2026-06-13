/**
 * TCP Connect 节点配置组件
 * 支持主机地址、端口、TLS、成帧配置、连接超时
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef, useState } from 'react';
import {
  tcpConnectConfigSchema,
  type TcpConnectConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { Cable, Info, AlertTriangle, ChevronDown, ChevronRight } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { TcpConnectConfig as TcpConnectConfigType, TcpFramingMode } from '@/types';

interface TcpConnectConfigProps {
  /** 当前节点数据 */
  data: TcpConnectConfigType;
  /** 保存回调 */
  onSave: (data: TcpConnectConfigType) => void;
}

/** 成帧模式选项 */
function getFramingModeOptions(): Array<{ value: string; label: string }> {
  const ll = getLL().panels.tcpConnect;
  return [
    { value: 'length_prefix', label: ll.lengthPrefix_option() },
    { value: 'delimiter', label: ll.delimiter_option() },
    { value: 'fixed_length', label: ll.fixedLength_option() },
    { value: 'timeout', label: ll.timeoutRead_option() },
    { value: 'manual', label: ll.manualControl_option() },
  ];
}

/**
 * 将 TcpConnectConfigType 转换为表单数据
 */
function toFormData(data: TcpConnectConfigType): TcpConnectConfigFormData {
  return {
    type: 'tcp_connect',
    host: data.host || '',
    port: data.port ?? 0,
    use_tls: data.use_tls ?? false,
    tls_skip_verify: data.tls_skip_verify ?? false,
    connect_timeout_ms: data.connect_timeout_ms ?? 30000,
    framing_mode: data.framing_mode,
    framing_config: data.framing_config,
    no_delay: data.no_delay ?? false,
    keepalive: data.keepalive,
    tls_ca_cert: data.tls_ca_cert ?? '',
  };
}

/**
 * 将表单数据转换为 TcpConnectConfigType
 */
function fromFormData(formData: TcpConnectConfigFormData): TcpConnectConfigType {
  const result: TcpConnectConfigType = {
    type: 'tcp_connect',
    host: formData.host,
    port: formData.port,
    use_tls: formData.use_tls,
    tls_skip_verify: formData.tls_skip_verify,
    connect_timeout_ms: formData.connect_timeout_ms,
    framing_mode: formData.framing_mode,
    framing_config: formData.framing_config,
    no_delay: formData.no_delay,
    tls_ca_cert: formData.tls_ca_cert || undefined,
  };
  // 仅在 keepalive 有值时才添加
  if (formData.keepalive) {
    result.keepalive = formData.keepalive;
  }
  return result;
}

/**
 * 获取成帧模式的显示名称
 */
function getFramingModeLabel(mode: TcpFramingMode | undefined): string {
  if (!mode) return getLL().panels.tcpConnect.notSet();
  return getFramingModeOptions().find(o => o.value === mode.type)?.label ?? mode.type;
}

export function TcpConnectConfig({ data, onSave }: TcpConnectConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<TcpConnectConfigFormData>({
    resolver: zodResolver(tcpConnectConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 监听表单值变化
  const useTls = watch('use_tls');
  const currentFramingMode = watch('framing_mode');

  // 成帧模式选择状态（UI 层面选择 basic mode type）
  const [selectedFramingType, setSelectedFramingType] = useState<string>(
    currentFramingMode?.type ?? 'none'
  );

  // Keepalive 配置展开状态
  const [keepaliveExpanded, setKeepaliveExpanded] = useState<boolean>(
    !!data.keepalive
  );

  // 追踪待执行的 setTimeout，组件卸载时清除
  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

  // 追踪表单实际修改状态
  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

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
    setSelectedFramingType(data.framing_mode?.type ?? 'none');
    setKeepaliveExpanded(!!data.keepalive);
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
        console.error('保存 TCP Connect 配置失败:', error);
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
   * 处理成帧模式类型切换
   */
  const handleFramingTypeChange = useCallback((type: string) => {
    setSelectedFramingType(type);
    let newMode: TcpFramingMode | undefined;
    switch (type) {
      case 'length_prefix':
        newMode = { type: 'length_prefix', prefix_bytes: 4 };
        break;
      case 'delimiter':
        newMode = { type: 'delimiter', delimiter: '\n' };
        break;
      case 'fixed_length':
        newMode = { type: 'fixed_length', length: 1024 };
        break;
      case 'timeout':
        newMode = { type: 'timeout', timeout_ms: 5000 };
        break;
      case 'manual':
        newMode = { type: 'manual' };
        break;
      case 'none':
        newMode = undefined;
        break;
      default:
        newMode = undefined;
    }
    setValue('framing_mode', newMode as TcpFramingMode, { shouldDirty: true });
    safeSetTimeout(saveData, 100);
  }, [setValue, safeSetTimeout, saveData]);

  /**
   * 更新成帧子字段（length_prefix 的 prefix_bytes 等）
   */
  const updateFramingSubField = useCallback((field: string, value: number | string) => {
    const currentMode = getValues('framing_mode');
    if (!currentMode) return;
    const updated = { ...currentMode, [field]: value } as TcpFramingMode;
    setValue('framing_mode', updated, { shouldDirty: true });
    safeSetTimeout(saveData, 100);
  }, [getValues, setValue, safeSetTimeout, saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-emerald-200/50 bg-gradient-to-br from-emerald-50/80 to-teal-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-emerald-900 flex items-center gap-2">
            <Cable className="h-4 w-4" />
            {getLL().panels.tcpConnect.title()}
          </h4>
          <p className="text-sm leading-relaxed text-emerald-700/90 whitespace-normal break-words">
            {getLL().panels.tcpConnect.description()}
          </p>
        </div>
      </div>

      {/* 主机地址 */}
      <div className="space-y-2">
        <Label htmlFor="host" className="text-sm font-medium text-foreground">
          {getLL().panels.tcpConnect.host()} <span className="text-rose-500">*</span>
        </Label>
        <div className="relative">
          <Cable className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <Input
            id="host"
            {...register('host')}
            placeholder={getLL().panels.tcpConnect.hostPlaceholder()}
            className="pl-9 font-mono text-sm"
          />
        </div>
        {errors.host && (
          <p className="text-xs text-rose-500">{errors.host.message}</p>
        )}
      </div>

      {/* 端口 */}
      <div className="space-y-2">
        <Label htmlFor="port" className="text-sm font-medium text-foreground">
          {getLL().panels.tcpConnect.port()} <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="port"
          type="number"
          min={1}
          max={65535}
          {...register('port', { valueAsNumber: true })}
          className="font-mono text-sm"
        />
        {errors.port && (
          <p className="text-xs text-rose-500">{errors.port.message}</p>
        )}
        <p className="text-xs text-muted-foreground">{getLL().panels.tcpConnect.portRange()}</p>
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
            {getLL().panels.tcpConnect.enableTls()}
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
                {getLL().panels.tcpConnect.skipCertVerify()}
              </Label>
            </div>
            {watch('tls_skip_verify') && (
              <div className="flex items-start gap-2 rounded-md border border-amber-200 bg-amber-50 p-2.5 dark:border-amber-800/60 dark:bg-amber-950/30">
                <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-500" />
                <p className="text-xs leading-relaxed text-amber-700 dark:text-amber-400">
                  {getLL().panels.tcpConnect.skipCertVerifyWarning()}
                </p>
              </div>
            )}
          </>
        )}
      </div>

      {/* 连接超时 */}
      <div className="space-y-2">
        <Label htmlFor="connect_timeout_ms" className="text-sm font-medium text-foreground">
          {getLL().panels.tcpConnect.connectTimeout()}
        </Label>
        <Input
          id="connect_timeout_ms"
          type="number"
          min={1000}
          max={300000}
          {...register('connect_timeout_ms', { valueAsNumber: true })}
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">{getLL().panels.tcpConnect.timeoutHint()}</p>
      </div>

      {/* 高级选项 */}
      <div className="space-y-3 rounded-md border bg-muted/30 p-3">
        <Label className="text-sm font-medium text-foreground">{getLL().panels.advancedSettings()}</Label>

        {/* NoDelay 开关 */}
        <div className="flex items-center space-x-2">
          <Switch
            id="no_delay"
            checked={watch('no_delay') || false}
            onCheckedChange={(checked) => {
              setValue('no_delay', checked, { shouldDirty: true });
              saveData();
            }}
          />
          <Label htmlFor="no_delay" className="text-sm font-medium text-foreground cursor-pointer">
            {getLL().panels.tcpConnect.noDelay()}
          </Label>
        </div>
        <p className="text-xs text-muted-foreground">
          {getLL().panels.tcpConnect.noDelayDesc()}
        </p>

        {/* Keepalive 配置 */}
        <div className="space-y-2">
          <button
            type="button"
            className="flex items-center gap-1.5 text-sm font-medium text-foreground hover:text-foreground/80 transition-colors"
            onClick={() => {
              const expanded = !keepaliveExpanded;
              setKeepaliveExpanded(expanded);
              if (expanded && !getValues('keepalive')) {
                setValue('keepalive', { idle_secs: 60, interval_secs: 10, probe_count: 3 }, { shouldDirty: true });
                saveData();
              } else if (!expanded) {
                setValue('keepalive', undefined as unknown as { idle_secs: number; interval_secs: number; probe_count: number }, { shouldDirty: true });
                saveData();
              }
            }}
          >
            {keepaliveExpanded ? (
              <ChevronDown className="h-3.5 w-3.5" />
            ) : (
              <ChevronRight className="h-3.5 w-3.5" />
            )}
            {getLL().panels.tcpConnect.keepaliveConfig()}
          </button>

          {keepaliveExpanded && (
            <div className="space-y-3 pl-2 border-l-2 border-muted">
              <div className="space-y-1.5">
                <Label className="text-xs text-muted-foreground">{getLL().panels.tcpConnect.keepaliveIdleSecs()}</Label>
                <Input
                  type="number"
                  min={1}
                  value={watch('keepalive')?.idle_secs ?? 60}
                  onChange={(e) => {
                    const current = getValues('keepalive') ?? { idle_secs: 60, interval_secs: 10, probe_count: 3 };
                    setValue('keepalive', { ...current, idle_secs: Number(e.target.value) || 60 }, { shouldDirty: true });
                    safeSetTimeout(saveData, 100);
                  }}
                  className="h-8 font-mono text-xs"
                />
              </div>
              <div className="space-y-1.5">
                <Label className="text-xs text-muted-foreground">{getLL().panels.tcpConnect.keepaliveIntervalSecs()}</Label>
                <Input
                  type="number"
                  min={1}
                  value={watch('keepalive')?.interval_secs ?? 10}
                  onChange={(e) => {
                    const current = getValues('keepalive') ?? { idle_secs: 60, interval_secs: 10, probe_count: 3 };
                    setValue('keepalive', { ...current, interval_secs: Number(e.target.value) || 10 }, { shouldDirty: true });
                    safeSetTimeout(saveData, 100);
                  }}
                  className="h-8 font-mono text-xs"
                />
              </div>
              <div className="space-y-1.5">
                <Label className="text-xs text-muted-foreground">{getLL().panels.tcpConnect.keepaliveProbeCount()}</Label>
                <Input
                  type="number"
                  min={1}
                  value={watch('keepalive')?.probe_count ?? 3}
                  onChange={(e) => {
                    const current = getValues('keepalive') ?? { idle_secs: 60, interval_secs: 10, probe_count: 3 };
                    setValue('keepalive', { ...current, probe_count: Number(e.target.value) || 3 }, { shouldDirty: true });
                    safeSetTimeout(saveData, 100);
                  }}
                  className="h-8 font-mono text-xs"
                />
              </div>
            </div>
          )}
        </div>

        {/* CA 证书（仅 TLS 启用时显示） */}
        {useTls && (
          <div className="space-y-2">
            <Label htmlFor="tls_ca_cert" className="text-sm font-medium text-foreground">
              {getLL().panels.tcpConnect.caCertPem()}
            </Label>
            <Textarea
              id="tls_ca_cert"
              {...register('tls_ca_cert')}
              placeholder="-----BEGIN CERTIFICATE-----&#10;...&#10;-----END CERTIFICATE-----"
              className="min-h-[100px] resize-y font-mono text-xs"
              rows={5}
            />
            <p className="text-xs text-muted-foreground">
              {getLL().panels.tcpConnect.caCertDesc()}
            </p>
          </div>
        )}
      </div>

      {/* 成帧配置 */}
      <div className="space-y-3 rounded-md border bg-muted/30 p-3">
        <Label className="text-sm font-medium text-foreground">{getLL().panels.tcpConnect.framingConfig()}</Label>
        <p className="text-xs text-muted-foreground">
          {getLL().panels.tcpConnect.framingDesc()}
        </p>

        {/* 成帧模式选择 */}
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <Select
              value={selectedFramingType}
              onValueChange={handleFramingTypeChange}
            >
              <SelectTrigger className="font-mono text-sm">
                <SelectValue placeholder={getLL().panels.tcpConnect.noFraming()} />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="none">{getLL().panels.tcpConnect.noFraming()}</SelectItem>
                {getFramingModeOptions().map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>

        {/* 成帧子字段：根据选中模式动态显示 */}
        {selectedFramingType === 'length_prefix' && (
          <div className="space-y-2 pl-2">
            <Label className="text-xs text-muted-foreground">{getLL().panels.tcpConnect.prefixBytes()}</Label>
            <Select
              value={String(currentFramingMode?.type === 'length_prefix' ? (currentFramingMode as { type: 'length_prefix'; prefix_bytes: number }).prefix_bytes : 4)}
              onValueChange={(val) => updateFramingSubField('prefix_bytes', Number(val))}
            >
              <SelectTrigger className="h-8 font-mono text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {[1, 2, 4, 8].map((n) => (
                  <SelectItem key={n} value={String(n)}>{getLL().panels.tcpConnect.bytes({ n })}</SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">{getLL().panels.tcpConnect.prefixBytesDesc()}</p>
          </div>
        )}

        {selectedFramingType === 'delimiter' && (
          <div className="space-y-2 pl-2">
            <Label className="text-xs text-muted-foreground">{getLL().panels.tcpConnect.delimiter()}</Label>
            <Input
              value={currentFramingMode?.type === 'delimiter' ? (currentFramingMode as { type: 'delimiter'; delimiter: string }).delimiter : '\n'}
              onChange={(e) => updateFramingSubField('delimiter', e.target.value)}
              className="h-8 font-mono text-xs"
              placeholder="\n"
            />
            <p className="text-xs text-muted-foreground">{getLL().panels.tcpConnect.delimiterDesc()}</p>
          </div>
        )}

        {selectedFramingType === 'fixed_length' && (
          <div className="space-y-2 pl-2">
            <Label className="text-xs text-muted-foreground">{getLL().panels.tcpConnect.fixedLength()}</Label>
            <Input
              type="number"
              min={1}
              value={currentFramingMode?.type === 'fixed_length' ? (currentFramingMode as { type: 'fixed_length'; length: number }).length : 1024}
              onChange={(e) => updateFramingSubField('length', Number(e.target.value))}
              className="h-8 font-mono text-xs"
            />
            <p className="text-xs text-muted-foreground">{getLL().panels.tcpConnect.fixedLengthDesc()}</p>
          </div>
        )}

        {selectedFramingType === 'timeout' && (
          <div className="space-y-2 pl-2">
            <Label className="text-xs text-muted-foreground">{getLL().panels.tcpConnect.readTimeout()}</Label>
            <Input
              type="number"
              min={100}
              value={currentFramingMode?.type === 'timeout' ? (currentFramingMode as { type: 'timeout'; timeout_ms: number }).timeout_ms : 5000}
              onChange={(e) => updateFramingSubField('timeout_ms', Number(e.target.value))}
              className="h-8 font-mono text-xs"
            />
            <p className="text-xs text-muted-foreground">{getLL().panels.tcpConnect.readTimeoutDesc()}</p>
          </div>
        )}

        {selectedFramingType === 'manual' && (
          <p className="text-xs text-muted-foreground pl-2">
            {getLL().panels.tcpConnect.manualModeDesc()}
          </p>
        )}
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.connection()}</p>
          <p>{getLL().panels.tcpConnect.connectionDesc()}</p>
          <p>{getLL().panels.tcpConnect.framingExplanation()}</p>
          <p>{getLL().panels.tcpConnect.currentFraming()} <code className="rounded bg-muted px-1">{getFramingModeLabel(currentFramingMode)}</code></p>
        </div>
      </div>
    </form>
  );
}
