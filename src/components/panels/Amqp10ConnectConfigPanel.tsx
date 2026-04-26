/**
 * AMQP 1.0 Connect 节点配置组件
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef } from 'react';
import {
  amqp10ConnectConfigSchema,
  type Amqp10ConnectConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Radio, Shield, ChevronDown, Info } from 'lucide-react';
import type { Amqp10ConnectConfig as ConfigType } from '@/types';
import { cn } from '@/lib/utils';

interface Props { data: ConfigType; onSave: (data: ConfigType) => void; }

function toFormData(data: ConfigType): Amqp10ConnectConfigFormData {
  return {
    type: 'amqp10_connect',
    url: data.url || '',
    connection_id: data.connection_id || '',
    tls_mode: data.tls_mode || 'none',
    auth_mechanism: data.auth_mechanism || 'PLAIN',
    container_id: data.container_id || '',
  };
}

function fromFormData(formData: Amqp10ConnectConfigFormData): ConfigType {
  return {
    type: 'amqp10_connect',
    url: formData.url || undefined,
    connection_id: formData.connection_id || undefined,
    tls_mode: formData.tls_mode === 'none' ? undefined : formData.tls_mode,
    auth_mechanism: formData.auth_mechanism === 'PLAIN' ? undefined : formData.auth_mechanism,
    container_id: formData.container_id || undefined,
  };
}

export function Amqp10ConnectConfigPanel({ data, onSave }: Props) {
  const {
    register, handleSubmit, reset, getValues, setValue, watch,
    formState: { isDirty },
  } = useForm<Amqp10ConnectConfigFormData>({
    resolver: zodResolver(amqp10ConnectConfigSchema),
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
            <Radio className="h-4 w-4" />
            AMQP 1.0 连接节点
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90">建立 AMQP 1.0 协议连接。</p>
        </div>
      </div>

      <div className="space-y-2">
        <Label htmlFor="url" className="text-sm font-medium text-foreground">服务器 URL</Label>
        <Input id="url" {...register('url')} placeholder="amqp://localhost:5672" className="font-mono text-sm" />
      </div>

      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">连接 ID</Label>
        <Input id="connection_id" {...register('connection_id')} placeholder="留空自动生成" className="font-mono text-sm" />
      </div>

      <div className="space-y-2">
        <Label htmlFor="container_id" className="text-sm font-medium text-foreground">容器 ID</Label>
        <Input id="container_id" {...register('container_id')} placeholder="可选" className="font-mono text-sm" />
      </div>

      <div className="border rounded-lg overflow-hidden">
        <button type="button" onClick={() => toggleSection('tls')} className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors">
          <div className="flex items-center gap-2"><Shield className="h-4 w-4 text-violet-600" /><span className="text-sm font-medium text-foreground">TLS 与认证</span></div>
          <ChevronDown className={cn("h-3.5 w-3.5 text-muted-foreground transition-transform", expandedSections.has('tls') && "rotate-180")} />
        </button>
        {expandedSections.has('tls') && (
          <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
            <div className="space-y-2">
              <Label className="text-xs font-medium text-foreground">TLS 模式</Label>
              <Select value={watch('tls_mode') || 'none'} onValueChange={(v) => { setValue('tls_mode', v as 'none' | 'standard' | 'skip-verify', { shouldDirty: true }); saveData(); }}>
                <SelectTrigger className="h-8 text-xs font-mono"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">不使用 TLS</SelectItem>
                  <SelectItem value="standard">标准 TLS</SelectItem>
                  <SelectItem value="skip-verify">跳过证书验证</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label className="text-xs font-medium text-foreground">认证机制</Label>
              <Select value={watch('auth_mechanism') || 'PLAIN'} onValueChange={(v) => { setValue('auth_mechanism', v as 'PLAIN' | 'ANONYMOUS' | 'EXTERNAL', { shouldDirty: true }); saveData(); }}>
                <SelectTrigger className="h-8 text-xs font-mono"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="PLAIN">PLAIN</SelectItem>
                  <SelectItem value="ANONYMOUS">ANONYMOUS</SelectItem>
                  <SelectItem value="EXTERNAL">EXTERNAL</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        )}
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-violet-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">AMQP 1.0 连接说明：</p>
          <p>AMQP 1.0 协议（Azure Service Bus, ActiveMQ 等兼容）</p>
          <p>与 AMQP 0-9-1 是不同协议，不兼容</p>
        </div>
      </div>
    </form>
  );
}
