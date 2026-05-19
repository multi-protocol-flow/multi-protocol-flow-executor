/**
 * AMQP 1.0 Send 节点配置组件
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef } from 'react';
import {
  amqp10SendConfigSchema,
  type Amqp10SendConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Send, Info, ChevronDown } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { Amqp10SendConfig as ConfigType, Amqp10MessageProperties } from '@/types';
import { cn } from '@/lib/utils';

interface Props { data: ConfigType; onSave: (data: ConfigType) => void; }

function toFormData(data: ConfigType): Amqp10SendConfigFormData {
  return {
    type: 'amqp10_send',
    connection_id: data.connection_id || '',
    address: data.address || '',
    message_body: data.message_body || '',
    message_properties: data.message_properties,
    message_annotations: data.message_annotations ? JSON.stringify(data.message_annotations, null, 2) : '',
  };
}

function fromFormData(formData: Amqp10SendConfigFormData): ConfigType {
  let annotations: Record<string, unknown> | undefined;
  if (formData.message_annotations?.trim()) {
    try { annotations = JSON.parse(formData.message_annotations); } catch { /* 忽略 */ }
  }
  const props = formData.message_properties;
  const hasProps = props && Object.values(props).some(v => v !== undefined && v !== '');
  return {
    type: 'amqp10_send',
    connection_id: formData.connection_id,
    address: formData.address,
    message_body: formData.message_body || undefined,
    message_properties: hasProps ? props as Amqp10MessageProperties : undefined,
    message_annotations: annotations,
  };
}

export function Amqp10SendConfigPanel({ data, onSave }: Props) {
  const {
    register, handleSubmit, reset, getValues,
    formState: { errors, isDirty },
  } = useForm<Amqp10SendConfigFormData>({
    resolver: zodResolver(amqp10SendConfigSchema),
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

  const propsPrefix = 'message_properties';

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      <div className="rounded-lg border border-violet-200/50 bg-gradient-to-br from-violet-50/80 to-purple-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-violet-900 flex items-center gap-2">
            <Send className="h-4 w-4" />
            {getLL().panels.amqp10Send.title()}
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90">{getLL().panels.amqp10Send.description()}</p>
        </div>
      </div>

      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">{getLL().panels.amqp10Send.connectionId()} <span className="text-rose-500">*</span></Label>
        <Input id="connection_id" {...register('connection_id')} placeholder={getLL().panels.enterConnectionId()} className="font-mono text-sm" />
        {errors.connection_id && <p className="text-xs text-rose-500">{errors.connection_id.message}</p>}
      </div>

      <div className="space-y-2">
        <Label htmlFor="address" className="text-sm font-medium text-foreground">{getLL().panels.amqp10Send.targetAddress()} <span className="text-rose-500">*</span></Label>
        <Input id="address" {...register('address')} placeholder="my-queue" className="font-mono text-sm" />
        {errors.address && <p className="text-xs text-rose-500">{errors.address.message}</p>}
      </div>

      <div className="space-y-2">
        <Label htmlFor="message_body" className="text-sm font-medium text-foreground">{getLL().panels.amqp10Send.payload()}</Label>
        <Textarea id="message_body" {...register('message_body')} placeholder={`{"message": "hello"}`} className="min-h-[80px] resize-y font-mono text-xs" rows={4} />
      </div>

      {/* 消息属性 */}
      <div className="border rounded-lg overflow-hidden">
        <button type="button" onClick={() => toggleSection('props')} className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors">
          <div className="flex items-center gap-2"><Info className="h-4 w-4 text-violet-600" /><span className="text-sm font-medium text-foreground">{getLL().panels.advancedSettings()}</span></div>
          <ChevronDown className={cn("h-3.5 w-3.5 text-muted-foreground transition-transform", expandedSections.has('props') && "rotate-180")} />
        </button>
        {expandedSections.has('props') && (
          <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
            <div className="space-y-2">
              <Label htmlFor={`${propsPrefix}.content_type`} className="text-xs font-medium text-foreground">Content-Type</Label>
              <Input id={`${propsPrefix}.content_type`} {...register(`${propsPrefix}.content_type`)} placeholder="application/json" className="h-8 text-xs font-mono" />
            </div>
            <div className="space-y-2">
              <Label htmlFor={`${propsPrefix}.content_encoding`} className="text-xs font-medium text-foreground">Content-Encoding</Label>
              <Input id={`${propsPrefix}.content_encoding`} {...register(`${propsPrefix}.content_encoding`)} placeholder="utf-8" className="h-8 text-xs font-mono" />
            </div>
            <div className="space-y-2">
              <Label htmlFor={`${propsPrefix}.correlation_id`} className="text-xs font-medium text-foreground">关联 ID</Label>
              <Input id={`${propsPrefix}.correlation_id`} {...register(`${propsPrefix}.correlation_id`)} placeholder={getLL().panels.optional()} className="h-8 text-xs font-mono" />
            </div>
            <div className="space-y-2">
              <Label htmlFor={`${propsPrefix}.message_id`} className="text-xs font-medium text-foreground">消息 ID</Label>
              <Input id={`${propsPrefix}.message_id`} {...register(`${propsPrefix}.message_id`)} placeholder={getLL().panels.optional()} className="h-8 text-xs font-mono" />
            </div>
            <div className="space-y-2">
              <Label htmlFor={`${propsPrefix}.reply_to`} className="text-xs font-medium text-foreground">回复地址</Label>
              <Input id={`${propsPrefix}.reply_to`} {...register(`${propsPrefix}.reply_to`)} placeholder={getLL().panels.optional()} className="h-8 text-xs font-mono" />
            </div>
            <div className="space-y-2">
              <Label htmlFor={`${propsPrefix}.subject`} className="text-xs font-medium text-foreground">主题</Label>
              <Input id={`${propsPrefix}.subject`} {...register(`${propsPrefix}.subject`)} placeholder={getLL().panels.optional()} className="h-8 text-xs font-mono" />
            </div>
            <div className="space-y-2">
              <Label htmlFor={`${propsPrefix}.to`} className="text-xs font-medium text-foreground">目标</Label>
              <Input id={`${propsPrefix}.to`} {...register(`${propsPrefix}.to`)} placeholder={getLL().panels.optional()} className="h-8 text-xs font-mono" />
            </div>
          </div>
        )}
      </div>

      <div className="space-y-2">
        <Label htmlFor="message_annotations" className="text-sm font-medium text-foreground">消息注解（JSON）</Label>
        <Textarea id="message_annotations" {...register('message_annotations')} placeholder={`{"x-opt-key": "value"}`} className="min-h-[60px] resize-y font-mono text-xs" rows={3} />
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-violet-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">AMQP 1.0 发送说明：</p>
          <p>目标地址可以是队列名或主题名</p>
          <p>消息属性用于路由和关联请求/响应</p>
        </div>
      </div>
    </form>
  );
}
