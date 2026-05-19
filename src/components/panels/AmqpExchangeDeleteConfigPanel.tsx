/**
 * AMQP Exchange Delete 节点配置组件
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef } from 'react';
import {
  amqpExchangeDeleteConfigSchema,
  type AmqpExchangeDeleteConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Trash2, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { AmqpExchangeDeleteConfig as ConfigType } from '@/types';

interface Props {
  data: ConfigType;
  onSave: (data: ConfigType) => void;
}

function toFormData(data: ConfigType): AmqpExchangeDeleteConfigFormData {
  return {
    type: 'amqp_exchange_delete',
    connection_id: data.connection_id || '',
    exchange: data.exchange || '',
    if_unused: data.if_unused ?? true,
  };
}

function fromFormData(formData: AmqpExchangeDeleteConfigFormData): ConfigType {
  return {
    type: 'amqp_exchange_delete',
    connection_id: formData.connection_id,
    exchange: formData.exchange,
    if_unused: formData.if_unused || undefined,
  };
}

export function AmqpExchangeDeleteConfigPanel({ data, onSave }: Props) {
  const {
    register,
    handleSubmit,
    reset,
    getValues,
    setValue,
    watch,
    formState: { errors, isDirty },
  } = useForm<AmqpExchangeDeleteConfigFormData>({
    resolver: zodResolver(amqpExchangeDeleteConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());
  useEffect(() => {
    return () => { pendingTimersRef.current.forEach(id => clearTimeout(id)); pendingTimersRef.current.clear(); };
  }, []);

  useEffect(() => {
    return () => {
      if (!isActuallyDirtyRef.current) return;
      try { onSave(fromFormData(getValues())); } catch { /* 静默 */ }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const safeSetTimeout = useCallback((fn: () => void, ms: number) => {
    const id = setTimeout(() => { pendingTimersRef.current.delete(id); fn(); }, ms);
    pendingTimersRef.current.add(id);
  }, []);

  useEffect(() => { reset(toFormData(data)); }, [data, reset]);

  const saveData = useCallback(() => {
    handleSubmit((fd) => {
      try { onSave(fromFormData(fd)); } catch (e) { console.error('保存失败:', e); }
    })();
  }, [handleSubmit, onSave]);

  const handleBlur = useCallback(() => {
    safeSetTimeout(() => { if (!isActuallyDirtyRef.current) return; saveData(); }, 100);
  }, [safeSetTimeout, saveData]);

  useEffect(() => {
    const h = () => { saveData(); };
    window.addEventListener('flush-config-panel', h);
    return () => { window.removeEventListener('flush-config-panel', h); };
  }, [saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      <div className="rounded-lg border border-violet-200/50 bg-gradient-to-br from-violet-50/80 to-purple-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-violet-900 flex items-center gap-2">
            <Trash2 className="h-4 w-4" />
            {getLL().panels.amqpExchange.title()}
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90">{getLL().panels.amqpExchange.description()}</p>
        </div>
      </div>

      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">连接 ID <span className="text-rose-500">*</span></Label>
        <Input id="connection_id" {...register('connection_id')} placeholder={getLL().panels.enterConnectionId()} className="font-mono text-sm" />
        {errors.connection_id && <p className="text-xs text-rose-500">{errors.connection_id.message}</p>}
      </div>

      <div className="space-y-2">
        <Label htmlFor="exchange" className="text-sm font-medium text-foreground">Exchange 名称 <span className="text-rose-500">*</span></Label>
        <Input id="exchange" {...register('exchange')} placeholder="my.exchange" className="font-mono text-sm" />
        {errors.exchange && <p className="text-xs text-rose-500">{errors.exchange.message}</p>}
      </div>

      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div><Label className="text-sm font-medium text-foreground">仅在未被使用时删除</Label><p className="text-xs text-muted-foreground">有队列绑定时拒绝删除</p></div>
        <Switch checked={watch('if_unused') ?? true} onCheckedChange={(v) => { setValue('if_unused', v, { shouldDirty: true }); saveData(); }} />
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-violet-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">Exchange 删除说明：</p>
          <p>启用 if_unused 可安全删除，避免影响已有绑定</p>
        </div>
      </div>
    </form>
  );
}
