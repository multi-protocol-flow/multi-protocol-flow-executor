/**
 * AMQP Close 节点配置组件
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef } from 'react';
import {
  amqpCloseConfigSchema,
  type AmqpCloseConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { XCircle, Info } from 'lucide-react';
import type { AmqpCloseConfig as AmqpCloseConfigType } from '@/types';

interface AmqpCloseConfigPanelProps {
  data: AmqpCloseConfigType;
  onSave: (data: AmqpCloseConfigType) => void;
}

function toFormData(data: AmqpCloseConfigType): AmqpCloseConfigFormData {
  return {
    type: 'amqp_close',
    connection_id: data.connection_id || '',
    reply_code: data.reply_code ?? 200,
    reply_text: data.reply_text || '',
  };
}

function fromFormData(formData: AmqpCloseConfigFormData): AmqpCloseConfigType {
  return {
    type: 'amqp_close',
    connection_id: formData.connection_id,
    reply_code: formData.reply_code || undefined,
    reply_text: formData.reply_text || undefined,
  };
}

export function AmqpCloseConfigPanel({ data, onSave }: AmqpCloseConfigPanelProps) {
  const {
    register,
    handleSubmit,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<AmqpCloseConfigFormData>({
    resolver: zodResolver(amqpCloseConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

  useEffect(() => {
    return () => {
      if (!isActuallyDirtyRef.current) return;
      try {
        const formData = getValues();
        const config = fromFormData(formData);
        onSave(config);
      } catch {
        // 静默处理
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const safeSetTimeout = useCallback((fn: () => void, ms: number) => {
    const id = setTimeout(() => {
      pendingTimersRef.current.delete(id);
      fn();
    }, ms);
    pendingTimersRef.current.add(id);
  }, []);

  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 AMQP Close 配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  const handleBlur = useCallback(() => {
    safeSetTimeout(() => {
      if (!isActuallyDirtyRef.current) return;
      saveData();
    }, 100);
  }, [safeSetTimeout, saveData]);

  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      <div className="rounded-lg border border-violet-200/50 bg-gradient-to-br from-violet-50/80 to-purple-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-violet-900 flex items-center gap-2">
            <XCircle className="h-4 w-4" />
            AMQP 关闭连接节点
          </h4>
          <p className="text-sm leading-relaxed text-violet-700/90 whitespace-normal break-words">
            关闭 AMQP 连接并释放资源。
          </p>
        </div>
      </div>

      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          连接 ID <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="connection_id"
          {...register('connection_id')}
          placeholder="输入连接 ID"
          className="font-mono text-sm"
        />
        {errors.connection_id && (
          <p className="text-xs text-rose-500">{errors.connection_id.message}</p>
        )}
      </div>

      <div className="space-y-2">
        <Label htmlFor="reply_code" className="text-sm font-medium text-foreground">
          关闭回复码
        </Label>
        <Input
          id="reply_code"
          type="number"
          {...register('reply_code', { valueAsNumber: true })}
          placeholder="200"
          className="font-mono text-sm"
        />
      </div>

      <div className="space-y-2">
        <Label htmlFor="reply_text" className="text-sm font-medium text-foreground">
          关闭回复文本
        </Label>
        <Input
          id="reply_text"
          {...register('reply_text')}
          placeholder="OK"
          className="font-mono text-sm"
        />
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-violet-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">关闭说明：</p>
          <p>选择要关闭的 AMQP 连接 ID</p>
          <p>回复码 200 表示正常关闭</p>
        </div>
      </div>
    </form>
  );
}
