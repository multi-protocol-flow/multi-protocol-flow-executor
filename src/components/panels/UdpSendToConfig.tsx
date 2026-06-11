/**
 * UDP SendTo 节点配置组件
 * 支持连接选择、数据输入、编码方式、目标地址/端口、发送超时
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import {
  udpSendToConfigSchema,
  type UdpSendToConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Send, Info } from 'lucide-react';
import type { UdpSendToConfig as UdpSendToConfigType } from '@/types';
import { getLL } from '@/i18n/i18n-lazy';
import { useFlowStore } from '@/stores';
import { isUdpBindConfig } from '@/types/node';

interface UdpSendToConfigProps {
  /** 当前节点数据 */
  data: UdpSendToConfigType;
  /** 保存回调 */
  onSave: (data: UdpSendToConfigType) => void;
}

/**
 * 数据编码方式选项
 */
const DATA_ENCODING_OPTIONS = [
  { value: 'text', label: '文本 (Text)' },
  { value: 'hex', label: '十六进制 (Hex)' },
  { value: 'base64', label: 'Base64' },
] as const;

/**
 * 将 UdpSendToConfigType 转换为表单数据
 */
function toFormData(data: UdpSendToConfigType): UdpSendToConfigFormData {
  return {
    type: 'udp_send_to',
    connection_id: data.connection_id || '',
    data: data.data || '',
    data_encoding: (data.data_encoding as 'text' | 'hex' | 'base64') ?? 'text',
    target_addr: data.target_addr || '',
    target_port: data.target_port ?? 0,
    send_timeout_ms: data.send_timeout_ms ?? 30000,
  };
}

/**
 * 将表单数据转换为 UdpSendToConfigType
 */
function fromFormData(formData: UdpSendToConfigFormData): UdpSendToConfigType {
  return {
    type: 'udp_send_to',
    connection_id: formData.connection_id,
    data: formData.data,
    data_encoding: formData.data_encoding,
    target_addr: formData.target_addr,
    target_port: formData.target_port,
    send_timeout_ms: formData.send_timeout_ms,
  };
}

export function UdpSendToConfig({ data, onSave }: UdpSendToConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<UdpSendToConfigFormData>({
    resolver: zodResolver(udpSendToConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 获取 flow 并提取 udp_bind 节点
  const flow = useFlowStore((state) => state.flow);

  // 提取可用的 UDP 连接（从 udp_bind 节点）
  const availableConnections = useMemo(() => {
    if (!flow) return [];
    const connections: Array<{ uuid: string; name: string; local_addr?: string; local_port?: number }> = [];
    flow.nodes.forEach(node => {
      if (isUdpBindConfig(node.data)) {
        connections.push({
          uuid: node.uuid,
          name: node.name,
          local_addr: node.data.local_addr,
          local_port: node.data.local_port,
        });
      }
    });
    return connections;
  }, [flow]);

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
        console.error('保存 UDP SendTo 配置失败:', error);
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

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-orange-200/50 bg-gradient-to-br from-orange-50/80 to-amber-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-orange-900 flex items-center gap-2">
            <Send className="h-4 w-4" />
            {getLL().panels.udpSendTo.title()}
          </h4>
          <p className="text-sm leading-relaxed text-orange-700/90 whitespace-normal break-words">
            {getLL().panels.udpSendTo.description()}
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
            {getLL().panels.udpSendTo.noConnectNode()}
          </p>
        )}
      </div>

      {/* 目标地址 */}
      <div className="space-y-2">
        <Label htmlFor="target_addr" className="text-sm font-medium text-foreground">
          {getLL().panels.udpSendTo.targetAddr()} <span className="text-rose-500">*</span>
        </Label>
        <div className="relative">
          <Send className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <Input
            id="target_addr"
            {...register('target_addr')}
            placeholder={getLL().panels.hostExample()}
            className="pl-9 font-mono text-sm"
          />
        </div>
        {errors.target_addr && (
          <p className="text-xs text-rose-500">{errors.target_addr.message}</p>
        )}
        <p className="text-xs text-muted-foreground">数据报发送的目标主机地址</p>
      </div>

      {/* 目标端口 */}
      <div className="space-y-2">
        <Label htmlFor="target_port" className="text-sm font-medium text-foreground">
          {getLL().panels.udpSendTo.targetPort()} <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="target_port"
          type="number"
          min={1}
          max={65535}
          {...register('target_port', { valueAsNumber: true })}
          className="font-mono text-sm"
        />
        {errors.target_port && (
          <p className="text-xs text-rose-500">{errors.target_port.message}</p>
        )}
        <p className="text-xs text-muted-foreground">范围 1-65535</p>
      </div>

      {/* 数据编码方式 */}
      <div className="space-y-2">
        <Label htmlFor="data_encoding" className="text-sm font-medium text-foreground">
          {getLL().panels.udpSendTo.dataEncoding()}
        </Label>
        <Select
          value={watch('data_encoding') || 'text'}
          onValueChange={(value) => {
            setValue('data_encoding', value as 'text' | 'hex' | 'base64', { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger id="data_encoding" className="font-mono text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {DATA_ENCODING_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          选择数据的编码格式，发送前会自动解码为原始字节
        </p>
      </div>

      {/* 数据输入 */}
      <div className="space-y-2">
        <Label htmlFor="data" className="text-sm font-medium text-foreground">
          {getLL().panels.udpSendTo.dataInput()}
        </Label>
        <Textarea
          id="data"
          {...register('data')}
          placeholder={
            watch('data_encoding') === 'hex'
              ? '48656C6C6F20576F726C64'
              : watch('data_encoding') === 'base64'
                ? 'SGVsbG8gV29ybGQ='
                : '输入要发送的数据...'
          }
          className="min-h-[120px] resize-y font-mono text-sm"
          rows={5}
        />
        {errors.data && (
          <p className="text-xs text-rose-500">{errors.data.message}</p>
        )}
        <p className="text-xs text-muted-foreground">
          {watch('data_encoding') === 'hex'
            ? '十六进制格式，如 48656C6C6F'
            : watch('data_encoding') === 'base64'
              ? 'Base64 编码的字符串'
              : '纯文本数据，支持变量引用 {{variable}}'}
        </p>
      </div>

      {/* 发送超时 */}
      <div className="space-y-2">
        <Label htmlFor="send_timeout_ms" className="text-sm font-medium text-foreground">
          {getLL().panels.udpSendTo.sendTimeout()}
        </Label>
        <Input
          id="send_timeout_ms"
          type="number"
          min={1000}
          max={300000}
          {...register('send_timeout_ms', { valueAsNumber: true })}
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">{getLL().panels.udpSendTo.sendTimeoutHint()}</p>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.udpSendTo.senderInfo()}</p>
          <p>{getLL().panels.udpSendTo.senderInfo1()}</p>
          <p>{getLL().panels.udpSendTo.senderInfo2()}</p>
</div>
      </div>
    </form>
  );
}
