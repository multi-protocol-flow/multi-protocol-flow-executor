/**
 * TCP Receive 节点配置组件
 * 支持连接选择、接收超时、成帧配置
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  tcpReceiveConfigSchema,
  type TcpReceiveConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Download, Info } from 'lucide-react';
import type { TcpReceiveConfig as TcpReceiveConfigType, TcpFramingMode } from '@/types';
import type { TcpFramingConfigFormData } from './schemas';
import { useFlowStore } from '@/stores';
import { isTcpConnectConfig } from '@/types/node';

/**
 * 成帧模式选项
 */
const FRAMING_MODE_OPTIONS = [
  { value: 'length_prefix', label: '长度前缀 (Length Prefix)' },
  { value: 'delimiter', label: '分隔符 (Delimiter)' },
  { value: 'fixed_length', label: '固定长度 (Fixed Length)' },
  { value: 'timeout', label: '超时读取 (Timeout)' },
  { value: 'manual', label: '手动控制 (Manual)' },
] as const;

interface TcpReceiveConfigProps {
  /** 当前节点数据 */
  data: TcpReceiveConfigType;
  /** 保存回调 */
  onSave: (data: TcpReceiveConfigType) => void;
}

/**
 * 将 TcpReceiveConfigType 转换为表单数据
 */
function toFormData(data: TcpReceiveConfigType): TcpReceiveConfigFormData {
  return {
    type: 'tcp_receive',
    connection_id: data.connection_id || '',
    receive_timeout_ms: data.receive_timeout_ms ?? 5000,
    framing_mode: data.framing_mode as unknown as TcpFramingMode | undefined,
    framing_config: data.framing_config as TcpFramingConfigFormData | undefined,
  };
}

/**
 * 将表单数据转换为 TcpReceiveConfigType
 */
function fromFormData(formData: TcpReceiveConfigFormData): TcpReceiveConfigType {
  return {
    type: 'tcp_receive',
    connection_id: formData.connection_id,
    receive_timeout_ms: formData.receive_timeout_ms,
    framing_mode: (formData.framing_mode ?? '') as string,
    framing_config: (formData.framing_config ?? {}) as Record<string, unknown>,
  };
}

/**
 * 获取成帧模式的显示名称
 */
function getFramingModeLabel(mode: TcpFramingMode | undefined): string {
  if (!mode) return '未设置';
  return FRAMING_MODE_OPTIONS.find(o => o.value === mode.type)?.label ?? mode.type;
}

export function TcpReceiveConfig({ data, onSave }: TcpReceiveConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<TcpReceiveConfigFormData>({
    resolver: zodResolver(tcpReceiveConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 获取 flow 并提取 tcp_connect 节点
  const flow = useFlowStore((state) => state.flow);

  // 提取可用的 TCP 连接（从 tcp_connect 节点）
  const availableConnections = useMemo(() => {
    if (!flow) return [];
    const connections: Array<{ uuid: string; name: string; host?: string; port?: number }> = [];
    flow.nodes.forEach(node => {
      if (isTcpConnectConfig(node.data)) {
        connections.push({
          uuid: node.uuid,
          name: node.name,
          host: node.data.host,
          port: node.data.port,
        });
      }
    });
    return connections;
  }, [flow]);

  // 监听表单值变化
  const currentFramingMode = watch('framing_mode');

  // 成帧模式选择状态
  const [selectedFramingType, setSelectedFramingType] = useState<string>(
    currentFramingMode?.type ?? 'none'
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
    setSelectedFramingType((data.framing_mode as unknown as TcpFramingMode | undefined)?.type ?? 'none');
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
        console.error('保存 TCP Receive 配置失败:', error);
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
      default:
        newMode = undefined;
        break;
    }
    setValue('framing_mode', newMode as TcpFramingMode, { shouldDirty: true });
    safeSetTimeout(saveData, 100);
  }, [setValue, safeSetTimeout, saveData]);

  /**
   * 更新成帧子字段
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
            <Download className="h-4 w-4" />
            TCP 接收节点
          </h4>
          <p className="text-sm leading-relaxed text-emerald-700/90 whitespace-normal break-words">
            从已建立的 TCP 连接接收数据，支持多种成帧模式。
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
              <SelectValue placeholder="选择 TCP 连接" />
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
            未检测到 TCP Connect 节点，请手动输入连接 ID
          </p>
        )}
      </div>

      {/* 接收超时 */}
      <div className="space-y-2">
        <Label htmlFor="receive_timeout_ms" className="text-sm font-medium text-foreground">
          接收超时
        </Label>
        <Input
          id="receive_timeout_ms"
          type="number"
          min={1000}
          max={300000}
          {...register('receive_timeout_ms', { valueAsNumber: true })}
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">毫秒 (ms)，默认 5000，范围 1000-300000</p>
      </div>

      {/* 成帧配置 */}
      <div className="space-y-3 rounded-md border bg-muted/30 p-3">
        <Label className="text-sm font-medium text-foreground">成帧配置（可选）</Label>
        <p className="text-xs text-muted-foreground">
          定义接收数据的帧解析方式，与连接配置配合使用
        </p>

        {/* 成帧模式选择 */}
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <Select
              value={selectedFramingType}
              onValueChange={handleFramingTypeChange}
            >
              <SelectTrigger className="font-mono text-sm">
                <SelectValue placeholder="不使用成帧" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="none">不使用成帧</SelectItem>
                {FRAMING_MODE_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>

        {/* 成帧子字段 */}
        {selectedFramingType === 'length_prefix' && (
          <div className="space-y-2 pl-2">
            <Label className="text-xs text-muted-foreground">前缀字节数</Label>
            <Select
              value={String(currentFramingMode?.type === 'length_prefix' ? (currentFramingMode as { type: 'length_prefix'; prefix_bytes: number }).prefix_bytes : 4)}
              onValueChange={(val) => updateFramingSubField('prefix_bytes', Number(val))}
            >
              <SelectTrigger className="h-8 font-mono text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {[1, 2, 4, 8].map((n) => (
                  <SelectItem key={n} value={String(n)}>{n} 字节</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        )}

        {selectedFramingType === 'delimiter' && (
          <div className="space-y-2 pl-2">
            <Label className="text-xs text-muted-foreground">分隔符</Label>
            <Input
              value={currentFramingMode?.type === 'delimiter' ? (currentFramingMode as { type: 'delimiter'; delimiter: string }).delimiter : '\n'}
              onChange={(e) => updateFramingSubField('delimiter', e.target.value)}
              className="h-8 font-mono text-xs"
              placeholder="\n"
            />
          </div>
        )}

        {selectedFramingType === 'fixed_length' && (
          <div className="space-y-2 pl-2">
            <Label className="text-xs text-muted-foreground">固定帧长度</Label>
            <Input
              type="number"
              min={1}
              value={currentFramingMode?.type === 'fixed_length' ? (currentFramingMode as { type: 'fixed_length'; length: number }).length : 1024}
              onChange={(e) => updateFramingSubField('length', Number(e.target.value))}
              className="h-8 font-mono text-xs"
            />
          </div>
        )}

        {selectedFramingType === 'timeout' && (
          <div className="space-y-2 pl-2">
            <Label className="text-xs text-muted-foreground">读取超时 (ms)</Label>
            <Input
              type="number"
              min={100}
              value={currentFramingMode?.type === 'timeout' ? (currentFramingMode as { type: 'timeout'; timeout_ms: number }).timeout_ms : 5000}
              onChange={(e) => updateFramingSubField('timeout_ms', Number(e.target.value))}
              className="h-8 font-mono text-xs"
            />
          </div>
        )}

        {selectedFramingType === 'manual' && (
          <p className="text-xs text-muted-foreground pl-2">
            手动模式下不自动成帧，直接接收原始数据
          </p>
        )}
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">接收说明：</p>
          <p>选择已建立的 TCP 连接，配置接收超时和成帧模式</p>
          <p>接收到的数据将存入变量供后续节点使用</p>
          <p>成帧配置: <code className="rounded bg-muted px-1">{getFramingModeLabel(currentFramingMode)}</code></p>
        </div>
      </div>
    </form>
  );
}
