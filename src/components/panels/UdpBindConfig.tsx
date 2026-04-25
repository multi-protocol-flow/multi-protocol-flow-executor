/**
 * UDP Bind 节点配置组件
 * 支持本地绑定地址、端口、地址复用配置
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef } from 'react';
import {
  udpBindConfigSchema,
  type UdpBindConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Radio, Info } from 'lucide-react';
import type { UdpBindConfig as UdpBindConfigType } from '@/types';

interface UdpBindConfigProps {
  /** 当前节点数据 */
  data: UdpBindConfigType;
  /** 保存回调 */
  onSave: (data: UdpBindConfigType) => void;
}

/**
 * 将 UdpBindConfigType 转换为表单数据
 */
function toFormData(data: UdpBindConfigType): UdpBindConfigFormData {
  return {
    type: 'udp_bind',
    local_addr: data.local_addr ?? '0.0.0.0',
    local_port: data.local_port ?? 0,
    reuse_addr: data.reuse_addr ?? true,
  };
}

/**
 * 将表单数据转换为 UdpBindConfigType
 */
function fromFormData(formData: UdpBindConfigFormData): UdpBindConfigType {
  return {
    type: 'udp_bind',
    local_addr: formData.local_addr,
    local_port: formData.local_port,
    reuse_addr: formData.reuse_addr,
  };
}

export function UdpBindConfig({ data, onSave }: UdpBindConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<UdpBindConfigFormData>({
    resolver: zodResolver(udpBindConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 监听表单值变化
  const reuseAddr = watch('reuse_addr');

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
        console.error('保存 UDP Bind 配置失败:', error);
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
            <Radio className="h-4 w-4" />
            UDP 绑定节点
          </h4>
          <p className="text-sm leading-relaxed text-orange-700/90 whitespace-normal break-words">
            绑定 UDP 套接字到指定本地地址和端口，用于后续的发送和接收操作。
          </p>
        </div>
      </div>

      {/* 本地绑定地址 */}
      <div className="space-y-2">
        <Label htmlFor="local_addr" className="text-sm font-medium text-foreground">
          本地绑定地址
        </Label>
        <div className="relative">
          <Radio className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <Input
            id="local_addr"
            {...register('local_addr')}
            placeholder="0.0.0.0"
            className="pl-9 font-mono text-sm"
          />
        </div>
        {errors.local_addr && (
          <p className="text-xs text-rose-500">{errors.local_addr.message}</p>
        )}
        <p className="text-xs text-muted-foreground">默认 0.0.0.0 绑定所有网卡</p>
      </div>

      {/* 本地端口 */}
      <div className="space-y-2">
        <Label htmlFor="local_port" className="text-sm font-medium text-foreground">
          本地端口 <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="local_port"
          type="number"
          min={0}
          max={65535}
          {...register('local_port', { valueAsNumber: true })}
          className="font-mono text-sm"
        />
        {errors.local_port && (
          <p className="text-xs text-rose-500">{errors.local_port.message}</p>
        )}
        <p className="text-xs text-muted-foreground">范围 0-65535，0 表示随机分配</p>
      </div>

      {/* 地址复用 */}
      <div className="space-y-3 rounded-md border bg-muted/30 p-3">
        <div className="flex items-center space-x-2">
          <Switch
            id="reuse_addr"
            checked={reuseAddr ?? true}
            onCheckedChange={(checked) => {
              setValue('reuse_addr', checked, { shouldDirty: true });
              saveData();
            }}
          />
          <Label htmlFor="reuse_addr" className="text-sm font-medium text-foreground cursor-pointer">
            允许地址复用 (SO_REUSEADDR)
          </Label>
        </div>
        <p className="text-xs text-muted-foreground">
          启用后允许绑定处于 TIME_WAIT 状态的地址和端口
        </p>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">绑定说明：</p>
          <p>UDP 是无连接协议，绑定后即可发送和接收数据报</p>
          <p>绑定 0.0.0.0 会监听所有可用网卡</p>
          <p>端口 0 表示由操作系统随机分配可用端口</p>
        </div>
      </div>
    </form>
  );
}
