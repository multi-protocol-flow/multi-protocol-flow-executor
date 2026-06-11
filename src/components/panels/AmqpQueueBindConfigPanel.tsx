/**
 * AMQP Queue Bind 节点配置组件
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef } from 'react';
import {
  amqpQueueBindConfigSchema,
  type AmqpQueueBindConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Link, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { AmqpQueueBindConfig as ConfigType } from '@/types';

interface Props { data: ConfigType; onSave: (data: ConfigType) => void; }

function toFormData(data: ConfigType): AmqpQueueBindConfigFormData {
  return {
    type: 'amqp_queue_bind',
    connection_id: data.connection_id || '',
    queue: data.queue || '',
    exchange: data.exchange || '',
    routing_key: data.routing_key || '',
    arguments: data.arguments ? JSON.stringify(data.arguments, null, 2) : '',
  };
}

function fromFormData(formData: AmqpQueueBindConfigFormData): ConfigType {
  let args: Record<string, unknown> | undefined;
  if (formData.arguments?.trim()) {
    try { args = JSON.parse(formData.arguments); } catch { /* 忽略 */ }
  }
  return {
    type: 'amqp_queue_bind',
    connection_id: formData.connection_id,
    queue: formData.queue,
    exchange: formData.exchange,
    routing_key: formData.routing_key || undefined,
    arguments: args,
  };
}

export function AmqpQueueBindConfigPanel({ data, onSave }: Props) {
  const {
    register, handleSubmit, reset, getValues,
    formState: { errors, isDirty },
  } = useForm<AmqpQueueBindConfigFormData>({
    resolver: zodResolver(amqpQueueBindConfigSchema),
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
            <Link className="h-4 w-4" />
            {getLL().panels.amqpQueueBind.title()}
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90">{getLL().panels.amqpQueueBind.description()}</p>
        </div>
      </div>

      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">{getLL().panels.amqpQueueBind.connectionId_label()} <span className="text-rose-500">*</span></Label>
        <Input id="connection_id" {...register('connection_id')} placeholder={getLL().panels.enterConnectionId()} className="font-mono text-sm" />
        {errors.connection_id && <p className="text-xs text-rose-500">{errors.connection_id.message}</p>}
      </div>

      <div className="space-y-2">
        <Label htmlFor="queue" className="text-sm font-medium text-foreground">{getLL().panels.amqpQueueBind.queueName_label()} <span className="text-rose-500">*</span></Label>
        <Input id="queue" {...register('queue')} placeholder="my.queue" className="font-mono text-sm" />
        {errors.queue && <p className="text-xs text-rose-500">{errors.queue.message}</p>}
      </div>

      <div className="space-y-2">
        <Label htmlFor="exchange" className="text-sm font-medium text-foreground">{getLL().panels.amqpQueueBind.exchangeName_label()} <span className="text-rose-500">*</span></Label>
        <Input id="exchange" {...register('exchange')} placeholder="my.exchange" className="font-mono text-sm" />
        {errors.exchange && <p className="text-xs text-rose-500">{errors.exchange.message}</p>}
      </div>

      <div className="space-y-2">
        <Label htmlFor="routing_key" className="text-sm font-medium text-foreground">{getLL().panels.amqpQueueBind.routingKey_label()}</Label>
        <Input id="routing_key" {...register('routing_key')} placeholder="routing.key" className="font-mono text-sm" />
      </div>

      <div className="space-y-2">
        <Label htmlFor="arguments" className="text-sm font-medium text-foreground">{getLL().panels.amqpQueueBind.arguments_label()}</Label>
        <Textarea id="arguments" {...register('arguments')} placeholder='{}' className="min-h-[60px] resize-y font-mono text-xs" rows={3} />
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-violet-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.amqpQueueBind.help_title()}</p>
          <p>{getLL().panels.amqpQueueBind.help_line1()}</p>
          <p>{getLL().panels.amqpQueueBind.help_line2()}</p>
        </div>
      </div>
    </form>
  );
}
