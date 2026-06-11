/**
 * AMQP 1.0 Accept 节点配置组件
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef } from 'react';
import {
  amqp10AcceptConfigSchema,
  type Amqp10AcceptConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { CheckCircle, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { Amqp10AcceptConfig as ConfigType } from '@/types';

interface Props { data: ConfigType; onSave: (data: ConfigType) => void; }

function toFormData(data: ConfigType): Amqp10AcceptConfigFormData {
  return {
    type: 'amqp10_accept',
    connection_id: data.connection_id || '',
    delivery_id: data.delivery_id ?? 0,
  };
}

function fromFormData(formData: Amqp10AcceptConfigFormData): ConfigType {
  return {
    type: 'amqp10_accept',
    connection_id: formData.connection_id,
    delivery_id: formData.delivery_id,
  };
}

export function Amqp10AcceptConfigPanel({ data, onSave }: Props) {
  const {
    register, handleSubmit, reset, getValues,
    formState: { errors, isDirty },
  } = useForm<Amqp10AcceptConfigFormData>({
    resolver: zodResolver(amqp10AcceptConfigSchema),
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
            <CheckCircle className="h-4 w-4" />
            {getLL().panels.amqp10Accept.title()}
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90">{getLL().panels.amqp10Accept.description()}</p>
        </div>
      </div>

      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">{getLL().panels.amqp10Accept.connectionId_label()} <span className="text-rose-500">*</span></Label>
        <Input id="connection_id" {...register('connection_id')} placeholder={getLL().panels.enterConnectionId()} className="font-mono text-sm" />
        {errors.connection_id && <p className="text-xs text-rose-500">{errors.connection_id.message}</p>}
      </div>

      <div className="space-y-2">
        <Label htmlFor="delivery_id" className="text-sm font-medium text-foreground">{getLL().panels.amqp10Accept.deliveryId_label()} <span className="text-rose-500">*</span></Label>
        <Input id="delivery_id" type="number" {...register('delivery_id', { valueAsNumber: true })} placeholder="0" className="font-mono text-sm" />
        {errors.delivery_id && <p className="text-xs text-rose-500">{errors.delivery_id.message}</p>}
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-violet-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.amqp10Accept.help_title()}</p>
          <p>{getLL().panels.amqp10Accept.help_line1()}</p>
        </div>
      </div>
    </form>
  );
}
