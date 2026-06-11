/**
 * AMQP 1.0 Receive 节点配置组件
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef } from 'react';
import {
  amqp10ReceiveConfigSchema,
  type Amqp10ReceiveConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Inbox, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { Amqp10ReceiveConfig as ConfigType } from '@/types';

interface Props { data: ConfigType; onSave: (data: ConfigType) => void; }

function toFormData(data: ConfigType): Amqp10ReceiveConfigFormData {
  return {
    type: 'amqp10_receive',
    connection_id: data.connection_id || '',
    address: data.address || '',
    credit: data.credit ?? 1,
    max_messages: data.max_messages ?? 1,
    timeout_ms: data.timeout_ms ?? 30000,
    auto_accept: data.auto_accept ?? true,
  };
}

function fromFormData(formData: Amqp10ReceiveConfigFormData): ConfigType {
  return {
    type: 'amqp10_receive',
    connection_id: formData.connection_id,
    address: formData.address,
    credit: formData.credit || undefined,
    max_messages: formData.max_messages || undefined,
    timeout_ms: formData.timeout_ms || undefined,
    auto_accept: formData.auto_accept || undefined,
  };
}

export function Amqp10ReceiveConfigPanel({ data, onSave }: Props) {
  const {
    register, handleSubmit, reset, getValues, setValue, watch,
    formState: { errors, isDirty },
  } = useForm<Amqp10ReceiveConfigFormData>({
    resolver: zodResolver(amqp10ReceiveConfigSchema),
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
            <Inbox className="h-4 w-4" />
            {getLL().panels.amqp10Receive.title()}
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90">{getLL().panels.amqp10Receive.description()}</p>
        </div>
      </div>

      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">{getLL().panels.amqp10Receive.connectionId_label()} <span className="text-rose-500">*</span></Label>
        <Input id="connection_id" {...register('connection_id')} placeholder={getLL().panels.enterConnectionId()} className="font-mono text-sm" />
        {errors.connection_id && <p className="text-xs text-rose-500">{errors.connection_id.message}</p>}
      </div>

      <div className="space-y-2">
        <Label htmlFor="address" className="text-sm font-medium text-foreground">{getLL().panels.amqp10Receive.sourceAddress_label()} <span className="text-rose-500">*</span></Label>
        <Input id="address" {...register('address')} placeholder="my-queue" className="font-mono text-sm" />
        {errors.address && <p className="text-xs text-rose-500">{errors.address.message}</p>}
      </div>

      <div className="space-y-2">
        <Label htmlFor="credit" className="text-sm font-medium text-foreground">{getLL().panels.amqp10Receive.credit_label()}</Label>
        <Input id="credit" type="number" {...register('credit', { valueAsNumber: true })} placeholder="1" className="font-mono text-sm" />
        <p className="text-xs text-muted-foreground">{getLL().panels.amqp10Receive.credit_desc()}</p>
      </div>

      <div className="space-y-2">
        <Label htmlFor="max_messages" className="text-sm font-medium text-foreground">{getLL().panels.amqp10Receive.maxMessages_label()}</Label>
        <Input id="max_messages" type="number" {...register('max_messages', { valueAsNumber: true })} placeholder="1" className="font-mono text-sm" />
      </div>

      <div className="space-y-2">
        <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground">{getLL().panels.amqp10Receive.timeout_label()}</Label>
        <Input id="timeout_ms" type="number" {...register('timeout_ms', { valueAsNumber: true })} placeholder="30000" className="font-mono text-sm" />
      </div>

      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div><Label className="text-sm font-medium text-foreground">{getLL().panels.amqp10Receive.autoAccept_label()}</Label><p className="text-xs text-muted-foreground">{getLL().panels.amqp10Receive.autoAccept_desc()}</p></div>
        <Switch checked={watch('auto_accept') ?? true} onCheckedChange={(v) => { setValue('auto_accept', v, { shouldDirty: true }); saveData(); }} />
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-violet-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.amqp10Receive.help_title()}</p>
          <p>{getLL().panels.amqp10Receive.help_line1()}</p>
          <p>{getLL().panels.amqp10Receive.help_line2()}</p>
        </div>
      </div>
    </form>
  );
}
