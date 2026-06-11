/**
 * AMQP Management 节点配置组件
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef } from 'react';
import {
  amqpManagementConfigSchema,
  type AmqpManagementConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Settings, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { AmqpManagementConfig as ConfigType } from '@/types';

interface Props { data: ConfigType; onSave: (data: ConfigType) => void; }

function toFormData(data: ConfigType): AmqpManagementConfigFormData {
  return {
    type: 'amqp_management',
    management_url: data.management_url || '',
    username: data.username || '',
    password: data.password || '',
    action: data.action || 'overview',
    vhost: data.vhost || '',
    queue_name: data.queue_name || '',
    connection_id: data.connection_id || '',
  };
}

function fromFormData(formData: AmqpManagementConfigFormData): ConfigType {
  return {
    type: 'amqp_management',
    management_url: formData.management_url || undefined,
    username: formData.username || undefined,
    password: formData.password || undefined,
    action: formData.action,
    vhost: formData.vhost || undefined,
    queue_name: formData.queue_name || undefined,
    connection_id: formData.connection_id || undefined,
  };
}

export function AmqpManagementConfigPanel({ data, onSave }: Props) {
  const {
    register, handleSubmit, reset, getValues, setValue, watch,
    formState: { isDirty },
  } = useForm<AmqpManagementConfigFormData>({
    resolver: zodResolver(amqpManagementConfigSchema),
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

  const currentAction = watch('action') || 'overview';

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      <div className="rounded-lg border border-violet-200/50 bg-gradient-to-br from-violet-50/80 to-purple-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-violet-900 flex items-center gap-2">
            <Settings className="h-4 w-4" />
            {getLL().panels.amqpManagement.title()}
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90">{getLL().panels.amqpManagement.description()}</p>
        </div>
      </div>

      <div className="space-y-2">
        <Label htmlFor="management_url" className="text-sm font-medium text-foreground">Management API URL</Label>
        <Input id="management_url" {...register('management_url')} placeholder="http://localhost:15672" className="font-mono text-sm" />
      </div>

      <div className="space-y-2">
        <Label htmlFor="username" className="text-sm font-medium text-foreground">用户名</Label>
        <Input id="username" {...register('username')} placeholder="guest" className="font-mono text-sm" />
      </div>

      <div className="space-y-2">
        <Label htmlFor="password" className="text-sm font-medium text-foreground">密码</Label>
        <Input id="password" type="password" {...register('password')} placeholder="guest" className="font-mono text-sm" />
      </div>

      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">操作类型</Label>
        <Select value={currentAction} onValueChange={(v) => { setValue('action', v as 'overview' | 'list-queues' | 'list-connections' | 'list-exchanges' | 'list-channels', { shouldDirty: true }); saveData(); }}>
          <SelectTrigger className="font-mono text-sm"><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="overview">概览</SelectItem>
            <SelectItem value="list-queues">列出队列</SelectItem>
            <SelectItem value="list-connections">列出连接</SelectItem>
            <SelectItem value="list-exchanges">列出 Exchange</SelectItem>
            <SelectItem value="list-channels">列出通道</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="space-y-2">
        <Label htmlFor="vhost" className="text-sm font-medium text-foreground">VHost</Label>
        <Input id="vhost" {...register('vhost')} placeholder="/" className="font-mono text-sm" />
      </div>

      {(currentAction === 'list-queues') && (
        <div className="space-y-2">
          <Label htmlFor="queue_name" className="text-sm font-medium text-foreground">队列名称</Label>
          <Input id="queue_name" {...register('queue_name')} placeholder="Leave empty to query all" className="font-mono text-sm" />
        </div>
      )}

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-violet-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">Management API 说明：</p>
          <p>需要 RabbitMQ 启用 management 插件</p>
          <p>默认端口 15672，默认用户 guest/guest</p>
        </div>
      </div>
    </form>
  );
}
