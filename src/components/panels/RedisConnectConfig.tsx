/**
 * Redis Connect 节点配置组件
 * 支持主机、端口、密码、数据库索引、连接超时
 */
import { useForm } from 'react-hook-form';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Radio, Info, Loader2, CheckCircle2, XCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';

import { getLL } from '@/i18n/i18n-lazy';
import { useFlowVariables } from '@/hooks/useFlowVariables';
import type { RedisConnectConfig as RedisConnectConfigType } from '@/types';
import { isTauriEnv } from '@/lib/platform';

/** 表单数据类型 */
interface RedisConnectFormData {
  type: 'redis_connect';
  host: string;
  port: number;
  password: string;
  db_index: number;
  connect_timeout_ms: number;
}

interface RedisConnectConfigProps {
  data: RedisConnectConfigType;
  onSave: (data: RedisConnectConfigType) => void;
}

function toFormData(data: RedisConnectConfigType): RedisConnectFormData {
  return {
    type: 'redis_connect',
    host: data.host || '',
    port: data.port ?? 6379,
    password: data.password || '',
    db_index: data.db_index ?? 0,
    connect_timeout_ms: data.connect_timeout_ms ?? 10000,
  };
}

function fromFormData(formData: RedisConnectFormData): RedisConnectConfigType {
  return {
    type: 'redis_connect',
    host: formData.host,
    port: formData.port,
    password: formData.password || undefined,
    db_index: formData.db_index,
    connect_timeout_ms: formData.connect_timeout_ms,
  };
}

export function RedisConnectConfig({ data, onSave }: RedisConnectConfigProps) {
  const {
    register,
    handleSubmit,
    reset,
    getValues,
    formState: { isDirty },
  } = useForm<RedisConnectFormData>({
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 流程变量（用于测试连接时预替换 {{variable}}）
  const { variables } = useFlowVariables();
  const varMap = useMemo(() => new Map(variables.map((v) => [v.name, v.value])), [variables]);

  function resolveVar(value: string): string {
    return value.replace(/\{\{([^}]+)\}\}/g, (_match: string, varPath: string) => {
      const trimmed = varPath.trim();
      if (varMap.has(trimmed)) {
        const resolved = varMap.get(trimmed);
        if (resolved !== null && resolved !== undefined) return String(resolved);
      }
      return _match;
    });
  }

  // 测试连接状态
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ success: boolean; message: string } | null>(null);

  /** 测试 Redis 连接 */
  const handleTestConnection = useCallback(async () => {
    const formData = getValues();
    const host = formData.host?.trim();
    if (!host) return;

    setIsTesting(true);
    setTestResult(null);

    // 预替换 {{variable}} 为流程初始变量的实际值
    const resolvedHost = resolveVar(formData.host);
    const resolvedPassword = resolveVar(formData.password || '');

    try {
      if (!isTauriEnv()) {
        setTestResult({ success: false, message: 'Connection test not available in browser mode. This feature requires the Tauri desktop app.' });
        setIsTesting(false);
        return;
      }
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<boolean>('test_redis_connection', {
        host: resolvedHost,
        port: formData.port,
        password: resolvedPassword || null,
        dbIndex: formData.db_index,
      });

      if (result) {
        setTestResult({ success: true, message: getLL().panels.redisConnect.testSuccess_msg() });
      } else {
        setTestResult({ success: false, message: getLL().panels.redisConnect.testFailed_msg() });
      }
    } catch (error) {
      setTestResult({
        success: false,
        message: typeof error === 'string' ? error : (error instanceof Error ? error.message : String(error)),
      });
    } finally {
      setIsTesting(false);
    }
  }, [getValues, varMap]);

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

  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 Redis Connect 配置失败:', error);
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
            <Radio className="h-4 w-4" />
            {ll.panels.redisConnect.title()}
          </h4>
          <p className="text-sm leading-relaxed text-red-700/90 whitespace-normal break-words">
            {ll.panels.redisConnect.description()}
          </p>
        </div>
      </div>

      {/* 连接配置 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{ll.panels.redisConnect.title()}</p>

        <div className="space-y-2">
          <Label htmlFor="host" className="text-sm font-medium text-foreground">
            {ll.panels.redisConnect.host()} <span className="text-red-500">*</span>
          </Label>
          <Input
            id="host"
            {...register('host')}
            placeholder="localhost"
            className="font-mono text-sm"
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="port" className="text-sm font-medium text-foreground">
            {ll.panels.redisConnect.port()}
          </Label>
          <Input
            id="port"
            type="number"
            {...register('port', { valueAsNumber: true })}
            placeholder="6379"
            className="font-mono text-sm"
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="password" className="text-sm font-medium text-foreground">
            {ll.panels.redisConnect.password()}
          </Label>
          <Input
            id="password"
            type="password"
            {...register('password')}
            placeholder="optional"
            className="font-mono text-sm"
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="db_index" className="text-sm font-medium text-foreground">
            {ll.panels.redisConnect.dbIndex()}
          </Label>
          <Input
            id="db_index"
            type="number"
            {...register('db_index', { valueAsNumber: true })}
            placeholder="0"
            className="font-mono text-sm"
          />
        </div>
      </div>

      {/* 高级设置 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{ll.panels.advancedSettings()}</p>

        <div className="space-y-2">
          <Label htmlFor="connect_timeout_ms" className="text-sm font-medium text-foreground">
            {ll.panels.redisConnect.connectTimeout()}
          </Label>
          <Input
            id="connect_timeout_ms"
            type="number"
            {...register('connect_timeout_ms', { valueAsNumber: true })}
            placeholder="10000"
            className="font-mono text-sm"
          />
        </div>
      </div>

      {/* 测试连接 */}
      <div className="space-y-2 pt-1">
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="w-full gap-2"
          disabled={isTesting}
          onClick={handleTestConnection}
        >
          {isTesting ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Radio className="h-4 w-4" />
          )}
          {isTesting ? ll.panels.sseConnect.testing() : ll.panels.sseConnect.testConnection()}
        </Button>
        {testResult && (
          <div className={`flex items-center gap-2 rounded-md p-2 text-xs ${
            testResult.success ? 'bg-green-50 text-green-700' : 'bg-red-50 text-red-700'
          }`}>
            {testResult.success ? <CheckCircle2 className="h-3.5 w-3.5 shrink-0" /> : <XCircle className="h-3.5 w-3.5 shrink-0" />}
            <span>{testResult.message}</span>
          </div>
        )}
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{ll.panels.redisConnect.description()}</p>
          <p>{ll.panels.redisConnect.port()} 6379</p>
          <p>{ll.panels.redisConnect.dbIndex()} 0-15</p>
        </div>
      </div>
    </form>
  );
}
