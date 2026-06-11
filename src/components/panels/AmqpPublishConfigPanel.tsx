/**
 * AMQP Publish 节点配置组件
 * 支持消息体、属性、消息头、确认模式
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef } from 'react';
import {
  amqpPublishConfigSchema,
  type AmqpPublishConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Send, Info, ChevronDown } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { AmqpPublishConfig as ConfigType } from '@/types';
import { cn } from '@/lib/utils';

interface Props { data: ConfigType; onSave: (data: ConfigType) => void; }

function toFormData(data: ConfigType): AmqpPublishConfigFormData {
  return {
    type: 'amqp_publish',
    connection_id: data.connection_id || '',
    exchange: data.exchange || '',
    routing_key: data.routing_key || '',
    body: data.body || '',
    body_type: data.body_type || 'text',
    content_type: data.content_type || '',
    content_encoding: data.content_encoding || '',
    delivery_mode: data.delivery_mode ?? 2,
    priority: data.priority ?? 0,
    message_id: data.message_id || '',
    correlation_id: data.correlation_id || '',
    reply_to: data.reply_to || '',
    expiration: data.expiration || '',
    timestamp: data.timestamp,
    message_type: data.message_type || '',
    user_id: data.user_id || '',
    app_id: data.app_id || '',
    cluster_id: data.cluster_id || '',
    headers: data.headers ? JSON.stringify(data.headers, null, 2) : '',
    mandatory: data.mandatory ?? false,
    confirm_mode: data.confirm_mode || 'none',
  };
}

function fromFormData(formData: AmqpPublishConfigFormData): ConfigType {
  let headers: Record<string, unknown> | undefined;
  if (formData.headers?.trim()) {
    try { headers = JSON.parse(formData.headers); } catch { /* 忽略 */ }
  }
  return {
    type: 'amqp_publish',
    connection_id: formData.connection_id,
    exchange: formData.exchange || undefined,
    routing_key: formData.routing_key || undefined,
    body: formData.body || undefined,
    body_type: formData.body_type || undefined,
    content_type: formData.content_type || undefined,
    content_encoding: formData.content_encoding || undefined,
    delivery_mode: formData.delivery_mode || undefined,
    priority: formData.priority || undefined,
    message_id: formData.message_id || undefined,
    correlation_id: formData.correlation_id || undefined,
    reply_to: formData.reply_to || undefined,
    expiration: formData.expiration || undefined,
    timestamp: formData.timestamp || undefined,
    message_type: formData.message_type || undefined,
    user_id: formData.user_id || undefined,
    app_id: formData.app_id || undefined,
    cluster_id: formData.cluster_id || undefined,
    headers,
    mandatory: formData.mandatory || undefined,
    confirm_mode: formData.confirm_mode === 'none' ? undefined : formData.confirm_mode,
  };
}

export function AmqpPublishConfigPanel({ data, onSave }: Props) {
  const {
    register, handleSubmit, reset, getValues, setValue, watch,
    formState: { errors, isDirty },
  } = useForm<AmqpPublishConfigFormData>({
    resolver: zodResolver(amqpPublishConfigSchema),
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

  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set());

  const toggleSection = useCallback((section: string) => {
    setExpandedSections(prev => {
      const next = new Set(prev);
      if (next.has(section)) next.delete(section); else next.add(section);
      return next;
    });
  }, []);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      <div className="rounded-lg border border-violet-200/50 bg-gradient-to-br from-violet-50/80 to-purple-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-violet-900 flex items-center gap-2">
            <Send className="h-4 w-4" />
            {getLL().panels.amqpPublish.title()}
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90">{getLL().panels.amqpPublish.description()}</p>
        </div>
      </div>

      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">连接 ID <span className="text-rose-500">*</span></Label>
        <Input id="connection_id" {...register('connection_id')} placeholder={getLL().panels.enterConnectionId()} className="font-mono text-sm" />
        {errors.connection_id && <p className="text-xs text-rose-500">{errors.connection_id.message}</p>}
      </div>

      <div className="space-y-2">
        <Label htmlFor="exchange" className="text-sm font-medium text-foreground">Exchange</Label>
        <Input id="exchange" {...register('exchange')} placeholder={getLL().panels.leaveEmptyDefaultExchange()} className="font-mono text-sm" />
      </div>

      <div className="space-y-2">
        <Label htmlFor="routing_key" className="text-sm font-medium text-foreground">路由键</Label>
        <Input id="routing_key" {...register('routing_key')} placeholder="routing.key" className="font-mono text-sm" />
      </div>

      <div className="space-y-2">
        <Label htmlFor="body" className="text-sm font-medium text-foreground">消息体</Label>
        <Textarea id="body" {...register('body')} placeholder='{"message": "hello"}' className="min-h-[80px] resize-y font-mono text-xs" rows={4} />
      </div>

      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">消息体类型</Label>
        <Select value={watch('body_type') || 'text'} onValueChange={(v) => { setValue('body_type', v as 'text' | 'json' | 'binary', { shouldDirty: true }); saveData(); }}>
          <SelectTrigger className="font-mono text-sm"><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="text">文本 (Text)</SelectItem>
            <SelectItem value="json">JSON</SelectItem>
            <SelectItem value="binary">二进制 (Binary)</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">投递模式</Label>
        <Select value={String(watch('delivery_mode') ?? 2)} onValueChange={(v) => { setValue('delivery_mode', Number(v), { shouldDirty: true }); saveData(); }}>
          <SelectTrigger className="font-mono text-sm"><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="1">1 — 非持久化</SelectItem>
            <SelectItem value="2">2 — 持久化</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">确认模式</Label>
        <Select value={watch('confirm_mode') || 'none'} onValueChange={(v) => { setValue('confirm_mode', v as 'none' | 'publisher-confirms' | 'transaction', { shouldDirty: true }); saveData(); }}>
          <SelectTrigger className="font-mono text-sm"><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="none">无确认</SelectItem>
            <SelectItem value="publisher-confirms">Publisher Confirms</SelectItem>
            <SelectItem value="transaction">事务</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div><Label className="text-sm font-medium text-foreground">Mandatory</Label><p className="text-xs text-muted-foreground">消息无法路由时返回给发送者</p></div>
        <Switch checked={watch('mandatory') ?? false} onCheckedChange={(v) => { setValue('mandatory', v, { shouldDirty: true }); saveData(); }} />
      </div>

      {/* 消息属性 */}
      <div className="border rounded-lg overflow-hidden">
        <button type="button" onClick={() => toggleSection('props')} className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors">
          <div className="flex items-center gap-2"><Info className="h-4 w-4 text-violet-600" /><span className="text-sm font-medium text-foreground">消息属性</span></div>
          <ChevronDown className={cn("h-3.5 w-3.5 text-muted-foreground transition-transform", expandedSections.has('props') && "rotate-180")} />
        </button>
        {expandedSections.has('props') && (
          <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
            <div className="space-y-2">
              <Label htmlFor="content_type" className="text-xs font-medium text-foreground">Content-Type</Label>
              <Input id="content_type" {...register('content_type')} placeholder="application/json" className="h-8 text-xs font-mono" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="content_encoding" className="text-xs font-medium text-foreground">Content-Encoding</Label>
              <Input id="content_encoding" {...register('content_encoding')} placeholder="utf-8" className="h-8 text-xs font-mono" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="priority" className="text-xs font-medium text-foreground">优先级 (0-9)</Label>
              <Input id="priority" type="number" {...register('priority', { valueAsNumber: true })} placeholder="0" className="h-8 text-xs font-mono" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="message_id" className="text-xs font-medium text-foreground">消息 ID</Label>
              <Input id="message_id" {...register('message_id')} placeholder={getLL().panels.optional()} className="h-8 text-xs font-mono" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="correlation_id" className="text-xs font-medium text-foreground">关联 ID</Label>
              <Input id="correlation_id" {...register('correlation_id')} placeholder={getLL().panels.optional()} className="h-8 text-xs font-mono" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="reply_to" className="text-xs font-medium text-foreground">回复队列</Label>
              <Input id="reply_to" {...register('reply_to')} placeholder={getLL().panels.optional()} className="h-8 text-xs font-mono" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="expiration" className="text-xs font-medium text-foreground">过期时间</Label>
              <Input id="expiration" {...register('expiration')} placeholder="TTL (ms)" className="h-8 text-xs font-mono" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="message_type" className="text-xs font-medium text-foreground">消息类型</Label>
              <Input id="message_type" {...register('message_type')} placeholder={getLL().panels.optional()} className="h-8 text-xs font-mono" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="user_id" className="text-xs font-medium text-foreground">用户 ID</Label>
              <Input id="user_id" {...register('user_id')} placeholder={getLL().panels.optional()} className="h-8 text-xs font-mono" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="app_id" className="text-xs font-medium text-foreground">应用 ID</Label>
              <Input id="app_id" {...register('app_id')} placeholder={getLL().panels.optional()} className="h-8 text-xs font-mono" />
            </div>
          </div>
        )}
      </div>

      <div className="space-y-2">
        <Label htmlFor="headers" className="text-sm font-medium text-foreground">消息头（JSON）</Label>
        <Textarea id="headers" {...register('headers')} placeholder='{"x-custom-header": "value"}' className="min-h-[60px] resize-y font-mono text-xs" rows={3} />
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-violet-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">发布说明：</p>
          <p>Exchange 为空时使用默认 Exchange，路由键作为队列名</p>
          <p>持久化消息需要队列也是持久化的才能在 Broker 重启后保留</p>
        </div>
      </div>
    </form>
  );
}
