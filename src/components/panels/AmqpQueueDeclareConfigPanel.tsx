/**
 * AMQP Queue Declare 节点配置组件
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef } from 'react';
import {
  amqpQueueDeclareConfigSchema,
  type AmqpQueueDeclareConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Switch } from '@/components/ui/switch';
import { ListPlus, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { AmqpQueueDeclareConfig as ConfigType } from '@/types';

interface Props { data: ConfigType; onSave: (data: ConfigType) => void; }

function toFormData(data: ConfigType): AmqpQueueDeclareConfigFormData {
  return {
    type: 'amqp_queue_declare',
    connection_id: data.connection_id || '',
    queue: data.queue ?? '',
    passive: data.passive ?? false,
    durable: data.durable ?? true,
    exclusive: data.exclusive ?? false,
    auto_delete: data.auto_delete ?? false,
    arguments: data.arguments ? JSON.stringify(data.arguments, null, 2) : '',
  };
}

function fromFormData(formData: AmqpQueueDeclareConfigFormData): ConfigType {
  let args: Record<string, unknown> | undefined;
  if (formData.arguments?.trim()) {
    try { args = JSON.parse(formData.arguments); } catch { /* 忽略 */ }
  }
  return {
    type: 'amqp_queue_declare',
    connection_id: formData.connection_id,
    queue: formData.queue || undefined,
    passive: formData.passive || undefined,
    durable: formData.durable || undefined,
    exclusive: formData.exclusive || undefined,
    auto_delete: formData.auto_delete || undefined,
    arguments: args,
  };
}

export function AmqpQueueDeclareConfigPanel({ data, onSave }: Props) {
  const {
    register, handleSubmit, reset, getValues, setValue, watch,
    formState: { errors, isDirty },
  } = useForm<AmqpQueueDeclareConfigFormData>({
    resolver: zodResolver(amqpQueueDeclareConfigSchema),
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
            <ListPlus className="h-4 w-4" />
            {getLL().panels.amqpQueue.title()}
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90">{getLL().panels.amqpQueue.description()}</p>
        </div>
      </div>

      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">连接 ID <span className="text-rose-500">*</span></Label>
        <Input id="connection_id" {...register('connection_id')} placeholder={getLL().panels.enterConnectionId()} className="font-mono text-sm" />
        {errors.connection_id && <p className="text-xs text-rose-500">{errors.connection_id.message}</p>}
      </div>

      <div className="space-y-2">
        <Label htmlFor="queue" className="text-sm font-medium text-foreground">队列名称</Label>
        <Input id="queue" {...register('queue')} placeholder="Auto-generate if empty" className="font-mono text-sm" />
        <p className="text-xs text-muted-foreground">留空时由 Broker 自动生成队列名称</p>
      </div>

      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div><Label className="text-sm font-medium text-foreground">持久化</Label></div>
        <Switch checked={watch('durable') ?? true} onCheckedChange={(v) => { setValue('durable', v, { shouldDirty: true }); saveData(); }} />
      </div>

      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div><Label className="text-sm font-medium text-foreground">被动声明</Label><p className="text-xs text-muted-foreground">仅检查是否存在</p></div>
        <Switch checked={watch('passive') ?? false} onCheckedChange={(v) => { setValue('passive', v, { shouldDirty: true }); saveData(); }} />
      </div>

      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div><Label className="text-sm font-medium text-foreground">独占队列</Label><p className="text-xs text-muted-foreground">仅当前连接可用</p></div>
        <Switch checked={watch('exclusive') ?? false} onCheckedChange={(v) => { setValue('exclusive', v, { shouldDirty: true }); saveData(); }} />
      </div>

      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div><Label className="text-sm font-medium text-foreground">自动删除</Label><p className="text-xs text-muted-foreground">最后一个消费者断开后删除</p></div>
        <Switch checked={watch('auto_delete') ?? false} onCheckedChange={(v) => { setValue('auto_delete', v, { shouldDirty: true }); saveData(); }} />
      </div>

      <div className="space-y-2">
        <Label htmlFor="arguments" className="text-sm font-medium text-foreground">额外参数（JSON）</Label>
        <Textarea id="arguments" {...register('arguments')} placeholder='{"x-message-ttl": 86400000, "x-max-priority": 10}' className="min-h-[60px] resize-y font-mono text-xs" rows={3} />
        <p className="text-xs text-muted-foreground">常用: x-message-ttl, x-dead-letter-exchange, x-max-priority, x-queue-type</p>
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-violet-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">Queue 声明说明：</p>
          <p>持久化队列在 Broker 重启后保留</p>
          <p>独占队列仅对创建它的连接可见</p>
        </div>
      </div>
    </form>
  );
}
