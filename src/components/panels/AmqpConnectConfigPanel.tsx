/**
 * AMQP Connect 节点配置组件
 * 支持 URL、心跳、通道、TLS、认证、自动恢复
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef } from 'react';
import {
  amqpConnectConfigSchema,
  type AmqpConnectConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Radio, Shield, ChevronDown, Info } from 'lucide-react';
import type { AmqpConnectConfig as AmqpConnectConfigType } from '@/types';
import { cn } from '@/lib/utils';

interface AmqpConnectConfigPanelProps {
  data: AmqpConnectConfigType;
  onSave: (data: AmqpConnectConfigType) => void;
}

function toFormData(data: AmqpConnectConfigType): AmqpConnectConfigFormData {
  return {
    type: 'amqp_connect',
    url: data.url || '',
    connection_id: data.connection_id || '',
    heartbeat: data.heartbeat ?? 60,
    channel_max: data.channel_max,
    frame_max: data.frame_max,
    connection_timeout_ms: data.connection_timeout_ms ?? 30000,
    tls_mode: data.tls_mode || 'none',
    auth_mechanism: data.auth_mechanism || 'PLAIN',
    auto_recover: data.auto_recover ?? false,
    reconnect_max_retries: data.reconnect_max_retries ?? 3,
  };
}

function fromFormData(formData: AmqpConnectConfigFormData): AmqpConnectConfigType {
  return {
    type: 'amqp_connect',
    url: formData.url || undefined,
    connection_id: formData.connection_id || undefined,
    heartbeat: formData.heartbeat || undefined,
    channel_max: formData.channel_max || undefined,
    frame_max: formData.frame_max || undefined,
    connection_timeout_ms: formData.connection_timeout_ms || undefined,
    tls_mode: formData.tls_mode === 'none' ? undefined : formData.tls_mode,
    auth_mechanism: formData.auth_mechanism === 'PLAIN' ? undefined : formData.auth_mechanism,
    auto_recover: formData.auto_recover || undefined,
    reconnect_max_retries: formData.auto_recover ? (formData.reconnect_max_retries || undefined) : undefined,
  };
}

export function AmqpConnectConfigPanel({ data, onSave }: AmqpConnectConfigPanelProps) {
  const {
    register,
    handleSubmit,
    reset,
    getValues,
    setValue,
    watch,
    formState: { errors, isDirty },
  } = useForm<AmqpConnectConfigFormData>({
    resolver: zodResolver(amqpConnectConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

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

  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 AMQP Connect 配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  const handleBlur = useCallback(() => {
    safeSetTimeout(() => {
      if (!isActuallyDirtyRef.current) return;
      saveData();
    }, 100);
  }, [safeSetTimeout, saveData]);

  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set());

  const toggleSection = useCallback((section: string) => {
    setExpandedSections(prev => {
      const next = new Set(prev);
      if (next.has(section)) next.delete(section);
      else next.add(section);
      return next;
    });
  }, []);

  const currentAutoRecover = watch('auto_recover') ?? false;

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-violet-200/50 bg-gradient-to-br from-violet-50/80 to-purple-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-violet-900 flex items-center gap-2">
            <Radio className="h-4 w-4" />
            AMQP 连接节点
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90 whitespace-normal break-words">
            建立 AMQP 0-9-1 Broker 连接，用于消息队列操作。
          </p>
        </div>
      </div>

      {/* Broker URL */}
      <div className="space-y-2">
        <Label htmlFor="url" className="text-sm font-medium text-foreground">
          Broker 地址
        </Label>
        <div className="relative">
          <Radio className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <Input
            id="url"
            {...register('url')}
            placeholder="amqp://guest:guest@localhost:5672"
            className="pl-9 font-mono text-sm"
          />
        </div>
        {errors.url && (
          <p className="text-xs text-rose-500">{errors.url.message}</p>
        )}
      </div>

      {/* 连接 ID */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          连接 ID
        </Label>
        <Input
          id="connection_id"
          {...register('connection_id')}
          placeholder="留空自动生成"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">留空时将自动生成唯一连接 ID</p>
      </div>

      {/* 心跳 */}
      <div className="space-y-2">
        <Label htmlFor="heartbeat" className="text-sm font-medium text-foreground">
          心跳间隔（秒）
        </Label>
        <Input
          id="heartbeat"
          type="number"
          {...register('heartbeat', { valueAsNumber: true })}
          placeholder="60"
          className="font-mono text-sm"
        />
      </div>

      {/* 连接超时 */}
      <div className="space-y-2">
        <Label htmlFor="connection_timeout_ms" className="text-sm font-medium text-foreground">
          连接超时（毫秒）
        </Label>
        <Input
          id="connection_timeout_ms"
          type="number"
          {...register('connection_timeout_ms', { valueAsNumber: true })}
          placeholder="30000"
          className="font-mono text-sm"
        />
      </div>

      {/* TLS 选项 */}
      <div className="border rounded-lg overflow-hidden">
        <button
          type="button"
          onClick={() => toggleSection('tls')}
          className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-2">
            <Shield className="h-4 w-4 text-violet-600" />
            <span className="text-sm font-medium text-foreground">TLS 与认证</span>
          </div>
          <ChevronDown className={cn(
            "h-3.5 w-3.5 text-muted-foreground transition-transform",
            expandedSections.has('tls') && "rotate-180"
          )} />
        </button>
        {expandedSections.has('tls') && (
          <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
            <div className="space-y-2">
              <Label className="text-xs font-medium text-foreground">TLS 模式</Label>
              <Select
                value={watch('tls_mode') || 'none'}
                onValueChange={(value) => {
                  setValue('tls_mode', value as 'none' | 'standard' | 'skip-verify', { shouldDirty: true });
                  saveData();
                }}
              >
                <SelectTrigger className="h-8 text-xs font-mono">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">不使用 TLS</SelectItem>
                  <SelectItem value="standard">标准 TLS</SelectItem>
                  <SelectItem value="skip-verify">跳过证书验证</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label className="text-xs font-medium text-foreground">认证机制</Label>
              <Select
                value={watch('auth_mechanism') || 'PLAIN'}
                onValueChange={(value) => {
                  setValue('auth_mechanism', value as 'PLAIN' | 'AMQPLAIN' | 'EXTERNAL' | 'RABBIT-CR-DEMO', { shouldDirty: true });
                  saveData();
                }}
              >
                <SelectTrigger className="h-8 text-xs font-mono">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="PLAIN">PLAIN</SelectItem>
                  <SelectItem value="AMQPLAIN">AMQPLAIN</SelectItem>
                  <SelectItem value="EXTERNAL">EXTERNAL</SelectItem>
                  <SelectItem value="RABBIT-CR-DEMO">RABBIT-CR-DEMO</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="channel_max" className="text-xs font-medium text-foreground">最大通道数</Label>
              <Input
                id="channel_max"
                type="number"
                {...register('channel_max', { valueAsNumber: true })}
                placeholder="可选"
                className="h-8 text-xs font-mono"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="frame_max" className="text-xs font-medium text-foreground">最大帧大小</Label>
              <Input
                id="frame_max"
                type="number"
                {...register('frame_max', { valueAsNumber: true })}
                placeholder="可选"
                className="h-8 text-xs font-mono"
              />
            </div>
          </div>
        )}
      </div>

      {/* 自动恢复 */}
      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div>
          <Label className="text-sm font-medium text-foreground">自动恢复</Label>
          <p className="text-xs text-muted-foreground">连接断开时自动重连</p>
        </div>
        <Switch
          checked={currentAutoRecover}
          onCheckedChange={(checked) => {
            setValue('auto_recover', checked, { shouldDirty: true });
            saveData();
          }}
        />
      </div>

      {currentAutoRecover && (
        <div className="space-y-2">
          <Label htmlFor="reconnect_max_retries" className="text-sm font-medium text-foreground">
            最大重连次数
          </Label>
          <Input
            id="reconnect_max_retries"
            type="number"
            {...register('reconnect_max_retries', { valueAsNumber: true })}
            placeholder="3"
            className="font-mono text-sm"
          />
        </div>
      )}

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-violet-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">AMQP 连接说明：</p>
          <p>支持 AMQP 0-9-1 协议（RabbitMQ 等兼容）</p>
          <p>URL 格式: amqp://user:pass@host:port/vhost</p>
          <p>启用自动恢复可在连接断开时自动重连</p>
        </div>
      </div>
    </form>
  );
}
