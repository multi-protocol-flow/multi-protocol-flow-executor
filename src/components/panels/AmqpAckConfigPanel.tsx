/**
 * AMQP Ack 节点配置组件
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef } from 'react';
import {
  amqpAckConfigSchema,
  type AmqpAckConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { CheckCircle, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { AmqpAckConfig as ConfigType } from '@/types';

interface Props { data: ConfigType; onSave: (data: ConfigType) => void; }

function toFormData(data: ConfigType): AmqpAckConfigFormData {
  return {
    type: 'amqp_ack',
    connection_id: data.connection_id || '',
    action: data.action || 'ack',
    delivery_tag: data.delivery_tag ?? 0,
    multiple: data.multiple ?? false,
    requeue: data.requeue ?? true,
  };
}

function fromFormData(formData: AmqpAckConfigFormData): ConfigType {
  return {
    type: 'amqp_ack',
    connection_id: formData.connection_id,
    action: formData.action,
    delivery_tag: formData.delivery_tag,
    multiple: formData.multiple || undefined,
    requeue: formData.requeue || undefined,
  };
}

export function AmqpAckConfigPanel({ data, onSave }: Props) {
  const {
    register, handleSubmit, reset, getValues, setValue, watch,
    formState: { errors, isDirty },
  } = useForm<AmqpAckConfigFormData>({
    resolver: zodResolver(amqpAckConfigSchema),
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

  const currentAction = watch('action') || 'ack';

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      <div className="rounded-lg border border-violet-200/50 bg-gradient-to-br from-violet-50/80 to-purple-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-violet-900 flex items-center gap-2">
            <CheckCircle className="h-4 w-4" />
            {getLL().panels.amqpAck.title()}
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90">{getLL().panels.amqpAck.description()}</p>
        </div>
      </div>

      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">{getLL().panels.amqpAck.connectionId_label()} <span className="text-rose-500">*</span></Label>
        <Input id="connection_id" {...register('connection_id')} placeholder={getLL().panels.enterConnectionId()} className="font-mono text-sm" />
        {errors.connection_id && <p className="text-xs text-rose-500">{errors.connection_id.message}</p>}
      </div>

      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">{getLL().panels.amqpAck.action_label()}</Label>
        <Select value={currentAction} onValueChange={(v) => { setValue('action', v as 'ack' | 'nack' | 'reject', { shouldDirty: true }); saveData(); }}>
          <SelectTrigger className="font-mono text-sm"><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="ack">{getLL().panels.amqpAck.ack_option()}</SelectItem>
            <SelectItem value="nack">{getLL().panels.amqpAck.nack_option()}</SelectItem>
            <SelectItem value="reject">{getLL().panels.amqpAck.reject_option()}</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="space-y-2">
        <Label htmlFor="delivery_tag" className="text-sm font-medium text-foreground">{getLL().panels.amqpAck.deliveryTag_label()} <span className="text-rose-500">*</span></Label>
        <Input id="delivery_tag" type="number" {...register('delivery_tag', { valueAsNumber: true })} placeholder="0" className="font-mono text-sm" />
        {errors.delivery_tag && <p className="text-xs text-rose-500">{errors.delivery_tag.message}</p>}
      </div>

      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div><Label className="text-sm font-medium text-foreground">{getLL().panels.amqpAck.multiple_label()}</Label><p className="text-xs text-muted-foreground">{getLL().panels.amqpAck.multiple_desc()}</p></div>
        <Switch checked={watch('multiple') ?? false} onCheckedChange={(v) => { setValue('multiple', v, { shouldDirty: true }); saveData(); }} />
      </div>

      {currentAction !== 'ack' && (
        <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
          <div><Label className="text-sm font-medium text-foreground">{getLL().panels.amqpAck.requeue_label()}</Label><p className="text-xs text-muted-foreground">{getLL().panels.amqpAck.requeue_desc()}</p></div>
          <Switch checked={watch('requeue') ?? true} onCheckedChange={(v) => { setValue('requeue', v, { shouldDirty: true }); saveData(); }} />
        </div>
      )}

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-violet-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.amqpAck.help_title()}</p>
          <p>{getLL().panels.amqpAck.help_line1()}</p>
          <p>{getLL().panels.amqpAck.help_line2()}</p>
          <p>{getLL().panels.amqpAck.help_line3()}</p>
        </div>
      </div>
    </form>
  );
}
