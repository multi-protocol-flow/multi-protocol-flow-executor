/**
 * Redis Command 节点配置组件
 * 用户在单行文本框中输入完整命令（如 SET key value），自动解析为 command + args
 */
import { useForm } from 'react-hook-form';
import { useCallback, useEffect, useRef, useMemo } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Terminal, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { RedisCommandConfig as RedisCommandConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { isRedisConnectConfig } from '@/types/node';

/** 解析命令文本为 command + args */
function parseCommandText(text: string): { command: string; args: string[] } {
  const trimmed = text.trim();
  if (!trimmed) return { command: '', args: [] };
  const parts = trimmed.split(/\s+/);
  return {
    command: parts[0].toUpperCase(),
    args: parts.slice(1),
  };
}

/** 将 command + args 合并为命令文本 */
function toCommandText(command: string, args: string[]): string {
  const cmd = command || 'GET';
  const parts = [cmd.toUpperCase(), ...(args || [])];
  return parts.filter((p) => p).join(' ');
}

/** 表单数据类型 */
interface RedisCommandFormData {
  type: 'redis_command';
  connection_id: string;
  command_text: string;
}

interface RedisCommandConfigProps {
  data: RedisCommandConfigType;
  onSave: (data: RedisCommandConfigType) => void;
}

function toFormData(data: RedisCommandConfigType): RedisCommandFormData {
  return {
    type: 'redis_command',
    connection_id: data.connection_id || '',
    command_text: toCommandText(data.command, data.args),
  };
}

function fromFormData(formData: RedisCommandFormData): RedisCommandConfigType {
  const parsed = parseCommandText(formData.command_text);
  return {
    type: 'redis_command',
    connection_id: formData.connection_id,
    command: parsed.command,
    args: parsed.args,
  };
}

export function RedisCommandConfig({ data, onSave }: RedisCommandConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { isDirty },
  } = useForm<RedisCommandFormData>({
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  const flow = useFlowStore((state) => state.flow);

  const availableConnections = useMemo(() => {
    if (!flow) return [];
    const connections: Array<{ uuid: string; name: string }> = [];
    flow.nodes.forEach((node) => {
      if (isRedisConnectConfig(node.data)) {
        connections.push({ uuid: node.uuid, name: node.name });
      }
    });
    return connections;
  }, [flow]);

  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

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
        // 静默处理卸载时的保存错误
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

  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 Redis Command 配置失败:', error);
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

  const ll = getLL();
  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-red-200/50 bg-gradient-to-br from-red-50/80 to-rose-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-red-900 flex items-center gap-2">
            <Terminal className="h-4 w-4" />
            {ll.panels.redisCommand.title()}
          </h4>
          <p className="text-sm leading-relaxed text-red-700/90 whitespace-normal break-words">
            {ll.panels.redisCommand.description()}
          </p>
        </div>
      </div>

      {/* 连接选择 */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          {ll.panels.connection()} <span className="text-red-500">*</span>
        </Label>
        {availableConnections.length > 0 ? (
          <Select
            value={watch('connection_id') || ''}
            onValueChange={(value) => {
              setValue('connection_id', value, { shouldDirty: true });
              saveData();
            }}
          >
            <SelectTrigger id="connection_id" className="font-mono">
              <SelectValue placeholder={ll.panels.redisCommand.title()} />
            </SelectTrigger>
            <SelectContent>
              {availableConnections.map((conn) => (
                <SelectItem key={conn.uuid} value={conn.uuid}>
                  <span className="font-mono">{conn.name}</span>
                  <span className="ml-2 text-xs text-muted-foreground">
                    ({conn.uuid.substring(0, 8)}...)
                  </span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        ) : (
          <Input
            id="connection_id"
            {...register('connection_id')}
            placeholder={ll.panels.redisCommand.connectionId()}
            className="font-mono text-sm"
          />
        )}
        {availableConnections.length === 0 && (
          <p className="text-xs text-muted-foreground">
            {ll.panels.redisCommand.noConnectNode()}
          </p>
        )}
      </div>

      {/* 命令输入 */}
      <div className="space-y-2">
        <Label htmlFor="command_text" className="text-sm font-medium text-foreground">
          {ll.panels.redisCommand.command()} <span className="text-red-500">*</span>
        </Label>
        <textarea
          id="command_text"
          {...register('command_text')}
          placeholder="SET key value"
          className="flex min-h-[60px] w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm font-mono shadow-xs placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
          autoComplete="off"
          spellCheck={false}
          rows={2}
        />
      </div>

      {/* 命令帮助 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          {ll.panels.redisCommand.commandHint().split('\n').map((line, i) => (
            <p key={i}>{line}</p>
          ))}
        </div>
      </div>
    </form>
  );
}
