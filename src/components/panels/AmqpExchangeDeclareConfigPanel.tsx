/**
 * AMQP Exchange Declare 节点配置组件
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef } from 'react';
import {
  amqpExchangeDeclareConfigSchema,
  type AmqpExchangeDeclareConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Layers, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { AmqpExchangeDeclareConfig as ConfigType } from '@/types';

interface Props {
  data: ConfigType;
  onSave: (data: ConfigType) => void;
}

function toFormData(data: ConfigType): AmqpExchangeDeclareConfigFormData {
  return {
    type: 'amqp_exchange_declare',
    connection_id: data.connection_id || '',
    exchange: data.exchange || '',
    exchange_type: data.exchange_type || 'direct',
    custom_type: data.custom_type || '',
    passive: data.passive ?? false,
    durable: data.durable ?? true,
    auto_delete: data.auto_delete ?? false,
    internal: data.internal ?? false,
    arguments: data.arguments ? JSON.stringify(data.arguments, null, 2) : '',
  };
}

function fromFormData(formData: AmqpExchangeDeclareConfigFormData): ConfigType {
  let args: Record<string, unknown> | undefined;
  if (formData.arguments?.trim()) {
    try { args = JSON.parse(formData.arguments); } catch { /* 忽略无效 JSON */ }
  }
  return {
    type: 'amqp_exchange_declare',
    connection_id: formData.connection_id,
    exchange: formData.exchange,
    exchange_type: formData.exchange_type,
    custom_type: formData.exchange_type === 'custom' ? (formData.custom_type || undefined) : undefined,
    passive: formData.passive || undefined,
    durable: formData.durable || undefined,
    auto_delete: formData.auto_delete || undefined,
    internal: formData.internal || undefined,
    arguments: args,
  };
}

export function AmqpExchangeDeclareConfigPanel({ data, onSave }: Props) {
  const {
    register,
    handleSubmit,
    reset,
    getValues,
    setValue,
    watch,
    formState: { errors, isDirty },
  } = useForm<AmqpExchangeDeclareConfigFormData>({
    resolver: zodResolver(amqpExchangeDeclareConfigSchema),
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

  const currentExchangeType = watch('exchange_type') || 'direct';

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      <div className="rounded-lg border border-violet-200/50 bg-gradient-to-br from-violet-50/80 to-purple-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-violet-900 flex items-center gap-2">
            <Layers className="h-4 w-4" />
            {getLL().panels.amqpExchangeDeclare.title()}
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90">{getLL().panels.amqpExchangeDeclare.description()}</p>
        </div>
      </div>

      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">{getLL().panels.amqpExchangeDeclare.connectionId_label()} <span className="text-rose-500">*</span></Label>
        <Input id="connection_id" {...register('connection_id')} placeholder={getLL().panels.enterConnectionId()} className="font-mono text-sm" />
        {errors.connection_id && <p className="text-xs text-rose-500">{errors.connection_id.message}</p>}
      </div>

      <div className="space-y-2">
        <Label htmlFor="exchange" className="text-sm font-medium text-foreground">{getLL().panels.amqpExchangeDeclare.exchangeName_label()} <span className="text-rose-500">*</span></Label>
        <Input id="exchange" {...register('exchange')} placeholder="my.exchange" className="font-mono text-sm" />
        {errors.exchange && <p className="text-xs text-rose-500">{errors.exchange.message}</p>}
      </div>

      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">{getLL().panels.amqpExchangeDeclare.exchangeType_label()}</Label>
        <Select
          value={currentExchangeType}
          onValueChange={(value) => {
            setValue('exchange_type', value as 'direct' | 'fanout' | 'topic' | 'headers' | 'custom', { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger className="font-mono text-sm"><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="direct">{getLL().panels.amqpExchangeDeclare.direct_option()}</SelectItem>
            <SelectItem value="fanout">{getLL().panels.amqpExchangeDeclare.fanout_option()}</SelectItem>
            <SelectItem value="topic">{getLL().panels.amqpExchangeDeclare.topic_option()}</SelectItem>
            <SelectItem value="headers">{getLL().panels.amqpExchangeDeclare.headers_option()}</SelectItem>
            <SelectItem value="custom">{getLL().panels.amqpExchangeDeclare.custom_option()}</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {currentExchangeType === 'custom' && (
        <div className="space-y-2">
          <Label htmlFor="custom_type" className="text-sm font-medium text-foreground">{getLL().panels.amqpExchangeDeclare.customType_label()}</Label>
          <Input id="custom_type" {...register('custom_type')} placeholder="x-delayed-message" className="font-mono text-sm" />
        </div>
      )}

      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div><Label className="text-sm font-medium text-foreground">{getLL().panels.amqpExchangeDeclare.durable_label()}</Label><p className="text-xs text-muted-foreground">{getLL().panels.amqpExchangeDeclare.durable_desc()}</p></div>
        <Switch checked={watch('durable') ?? true} onCheckedChange={(v) => { setValue('durable', v, { shouldDirty: true }); saveData(); }} />
      </div>

      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div><Label className="text-sm font-medium text-foreground">{getLL().panels.amqpExchangeDeclare.passive_label()}</Label><p className="text-xs text-muted-foreground">{getLL().panels.amqpExchangeDeclare.passive_desc()}</p></div>
        <Switch checked={watch('passive') ?? false} onCheckedChange={(v) => { setValue('passive', v, { shouldDirty: true }); saveData(); }} />
      </div>

      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div><Label className="text-sm font-medium text-foreground">{getLL().panels.amqpExchangeDeclare.autoDelete_label()}</Label><p className="text-xs text-muted-foreground">{getLL().panels.amqpExchangeDeclare.autoDelete_desc()}</p></div>
        <Switch checked={watch('auto_delete') ?? false} onCheckedChange={(v) => { setValue('auto_delete', v, { shouldDirty: true }); saveData(); }} />
      </div>

      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div><Label className="text-sm font-medium text-foreground">{getLL().panels.amqpExchangeDeclare.internal_label()}</Label><p className="text-xs text-muted-foreground">{getLL().panels.amqpExchangeDeclare.internal_desc()}</p></div>
        <Switch checked={watch('internal') ?? false} onCheckedChange={(v) => { setValue('internal', v, { shouldDirty: true }); saveData(); }} />
      </div>

      <div className="space-y-2">
        <Label htmlFor="arguments" className="text-sm font-medium text-foreground">{getLL().panels.amqpExchangeDeclare.arguments_label()}</Label>
        <Textarea id="arguments" {...register('arguments')} placeholder='{"alternate-exchange": "my.ae"}' className="min-h-[60px] resize-y font-mono text-xs" rows={3} />
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-violet-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.amqpExchangeDeclare.help_title()}</p>
          <p>{getLL().panels.amqpExchangeDeclare.help_line1()}</p>
          <p>{getLL().panels.amqpExchangeDeclare.help_line2()}</p>
          <p>{getLL().panels.amqpExchangeDeclare.help_line3()}</p>
          <p>{getLL().panels.amqpExchangeDeclare.help_line4()}</p>
        </div>
      </div>
    </form>
  );
}
