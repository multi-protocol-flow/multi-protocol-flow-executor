/**
 * AMQP Consume 节点配置组件
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef } from 'react';
import {
  amqpConsumeConfigSchema,
  type AmqpConsumeConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Inbox, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { AmqpConsumeConfig as ConfigType } from '@/types';

interface Props { data: ConfigType; onSave: (data: ConfigType) => void; }

function toFormData(data: ConfigType): AmqpConsumeConfigFormData {
  return {
    type: 'amqp_consume',
    connection_id: data.connection_id || '',
    queue: data.queue || '',
    consumer_tag: data.consumer_tag || '',
    ack_mode: data.ack_mode || 'auto',
    exclusive: data.exclusive ?? false,
    no_local: data.no_local ?? false,
    prefetch_count: data.prefetch_count ?? 10,
    max_messages: data.max_messages ?? 1,
    timeout_ms: data.timeout_ms ?? 30000,
    auto_ack_on_success: data.auto_ack_on_success ?? true,
    auto_reject_on_failure: data.auto_reject_on_failure ?? true,
    reject_requeue: data.reject_requeue ?? false,
  };
}

function fromFormData(formData: AmqpConsumeConfigFormData): ConfigType {
  return {
    type: 'amqp_consume',
    connection_id: formData.connection_id,
    queue: formData.queue,
    consumer_tag: formData.consumer_tag || undefined,
    ack_mode: formData.ack_mode || undefined,
    exclusive: formData.exclusive || undefined,
    no_local: formData.no_local || undefined,
    prefetch_count: formData.prefetch_count || undefined,
    max_messages: formData.max_messages || undefined,
    timeout_ms: formData.timeout_ms || undefined,
    auto_ack_on_success: formData.auto_ack_on_success || undefined,
    auto_reject_on_failure: formData.auto_reject_on_failure || undefined,
    reject_requeue: formData.reject_requeue || undefined,
  };
}

export function AmqpConsumeConfigPanel({ data, onSave }: Props) {
  const {
    register, handleSubmit, reset, getValues, setValue, watch,
    formState: { errors, isDirty },
  } = useForm<AmqpConsumeConfigFormData>({
    resolver: zodResolver(amqpConsumeConfigSchema),
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

  const currentAckMode = watch('ack_mode') || 'auto';

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      <div className="rounded-lg border border-violet-200/50 bg-gradient-to-br from-violet-50/80 to-purple-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-violet-900 flex items-center gap-2">
            <Inbox className="h-4 w-4" />
            {getLL().panels.amqpConsume.title()}
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90">{getLL().panels.amqpConsume.description()}</p>
        </div>
      </div>

      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">连接 ID <span className="text-rose-500">*</span></Label>
        <Input id="connection_id" {...register('connection_id')} placeholder={getLL().panels.enterConnectionId()} className="font-mono text-sm" />
        {errors.connection_id && <p className="text-xs text-rose-500">{errors.connection_id.message}</p>}
      </div>

      <div className="space-y-2">
        <Label htmlFor="queue" className="text-sm font-medium text-foreground">队列名称 <span className="text-rose-500">*</span></Label>
        <Input id="queue" {...register('queue')} placeholder="my.queue" className="font-mono text-sm" />
        {errors.queue && <p className="text-xs text-rose-500">{errors.queue.message}</p>}
      </div>

      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">确认模式</Label>
        <Select value={currentAckMode} onValueChange={(v) => { setValue('ack_mode', v as 'auto' | 'manual', { shouldDirty: true }); saveData(); }}>
          <SelectTrigger className="font-mono text-sm"><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="auto">自动确认</SelectItem>
            <SelectItem value="manual">手动确认</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="space-y-2">
        <Label htmlFor="consumer_tag" className="text-sm font-medium text-foreground">消费者标签</Label>
        <Input id="consumer_tag" {...register('consumer_tag')} placeholder="留空自动生成" className="font-mono text-sm" />
      </div>

      <div className="space-y-2">
        <Label htmlFor="prefetch_count" className="text-sm font-medium text-foreground">预取数量</Label>
        <Input id="prefetch_count" type="number" {...register('prefetch_count', { valueAsNumber: true })} placeholder="10" className="font-mono text-sm" />
      </div>

      <div className="space-y-2">
        <Label htmlFor="max_messages" className="text-sm font-medium text-foreground">最大消费消息数</Label>
        <Input id="max_messages" type="number" {...register('max_messages', { valueAsNumber: true })} placeholder="1" className="font-mono text-sm" />
      </div>

      <div className="space-y-2">
        <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground">超时（毫秒）</Label>
        <Input id="timeout_ms" type="number" {...register('timeout_ms', { valueAsNumber: true })} placeholder="30000" className="font-mono text-sm" />
      </div>

      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div><Label className="text-sm font-medium text-foreground">独占消费</Label></div>
        <Switch checked={watch('exclusive') ?? false} onCheckedChange={(v) => { setValue('exclusive', v, { shouldDirty: true }); saveData(); }} />
      </div>

      {currentAckMode === 'auto' && (
        <>
          <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
            <div><Label className="text-sm font-medium text-foreground">成功时自动确认</Label></div>
            <Switch checked={watch('auto_ack_on_success') ?? true} onCheckedChange={(v) => { setValue('auto_ack_on_success', v, { shouldDirty: true }); saveData(); }} />
          </div>
          <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
            <div><Label className="text-sm font-medium text-foreground">失败时自动拒绝</Label></div>
            <Switch checked={watch('auto_reject_on_failure') ?? true} onCheckedChange={(v) => { setValue('auto_reject_on_failure', v, { shouldDirty: true }); saveData(); }} />
          </div>
          <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
            <div><Label className="text-sm font-medium text-foreground">拒绝时重新入队</Label></div>
            <Switch checked={watch('reject_requeue') ?? false} onCheckedChange={(v) => { setValue('reject_requeue', v, { shouldDirty: true }); saveData(); }} />
          </div>
        </>
      )}

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-violet-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">消费说明：</p>
          <p>自动确认: 收到消息后自动 ack</p>
          <p>手动确认: 需配合 AMQP Ack 节点手动确认</p>
          <p>预取数量控制消费者一次性能接收多少未确认消息</p>
        </div>
      </div>
    </form>
  );
}
