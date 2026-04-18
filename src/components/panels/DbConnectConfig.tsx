/**
 * DB Connect 节点配置组件
 * 显示连接摘要卡片 + "配置数据库连接" 按钮打开 ConnectionDialog
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef, useState } from 'react';
import {
  dbConnectConfigSchema,
  type DbConnectConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Button } from '@/components/ui/button';
import { Database, Settings2, Server, User, FileText } from 'lucide-react';
import type { DbConnectConfig as DbConnectConfigType } from '@/types/node';
import {
  ConnectionDialog,
  type ConnectionFormData,
} from '@/components/dialogs/ConnectionDialog';

interface DbConnectConfigProps {
  /** 当前节点数据 */
  data: DbConnectConfigType;
  /** 保存回调 */
  onSave: (data: DbConnectConfigType) => void;
}

/**
 * 将 DbConnectConfigType 转换为表单数据
 */
function toFormData(data: DbConnectConfigType): DbConnectConfigFormData {
  return {
    type: 'db_connect',
    driver: data.driver || '',
    connection_string: data.connection_string || '',
    host: data.host || '',
    port: data.port,
    database: data.database || '',
    username: data.username || '',
    password: data.password || '',
    min_connections: data.min_connections,
    max_connections: data.max_connections,
    test_query: data.test_query || '',
    connect_timeout_ms: data.connect_timeout_ms,
  };
}

/**
 * 将表单数据转换为 DbConnectConfigType
 */
function fromFormData(formData: DbConnectConfigFormData): DbConnectConfigType {
  return {
    type: 'db_connect',
    driver: formData.driver && formData.driver.trim() ? formData.driver.trim() : undefined,
    connection_string: formData.connection_string && formData.connection_string.trim() ? formData.connection_string.trim() : undefined,
    host: formData.host && formData.host.trim() ? formData.host.trim() : undefined,
    port: formData.port ?? undefined,
    database: formData.database && formData.database.trim() ? formData.database.trim() : undefined,
    username: formData.username && formData.username.trim() ? formData.username.trim() : undefined,
    password: formData.password && formData.password.trim() ? formData.password.trim() : undefined,
    min_connections: formData.min_connections,
    max_connections: formData.max_connections,
    test_query: formData.test_query && formData.test_query.trim() ? formData.test_query.trim() : undefined,
    connect_timeout_ms: formData.connect_timeout_ms,
  };
}

/**
 * 获取数据库驱动的显示名称
 */
function getDriverLabel(driver: string): string {
  const labels: Record<string, string> = {
    postgres: 'PostgreSQL',
    mysql: 'MySQL',
    sqlite: 'SQLite',
    redis: 'Redis',
  };
  return labels[driver] || driver;
}

/**
 * 获取数据库驱动的图标颜色
 */
function getDriverColor(driver: string): string {
  const colors: Record<string, string> = {
    postgres: 'text-blue-600',
    mysql: 'text-orange-600',
    sqlite: 'text-green-600',
    redis: 'text-red-600',
  };
  return colors[driver] || 'text-purple-600';
}

/**
 * 获取数据库驱动的徽标背景色
 */
function getDriverBadgeBg(driver: string): string {
  const colors: Record<string, string> = {
    postgres: 'bg-blue-50 dark:bg-blue-950/50 border-blue-200 dark:border-blue-800',
    mysql: 'bg-orange-50 dark:bg-orange-950/50 border-orange-200 dark:border-orange-800',
    sqlite: 'bg-green-50 dark:bg-green-950/50 border-green-200 dark:border-green-800',
    redis: 'bg-red-50 dark:bg-red-950/50 border-red-200 dark:border-red-800',
  };
  return colors[driver] || 'bg-purple-50 dark:bg-purple-950/50 border-purple-200 dark:border-purple-800';
}

/**
 * 构建连接目标显示文本
 */
function getConnectionTarget(data: DbConnectConfigType): string | null {
  if (data.driver === 'sqlite' && data.connection_string) {
    return data.connection_string;
  }
  const parts: string[] = [];
  if (data.host) {
    parts.push(data.host);
    if (data.port) {
      parts[0] = `${data.host}:${data.port}`;
    }
  }
  if (data.database) {
    parts.push(data.database);
  }
  return parts.length > 0 ? parts.join(' / ') : null;
}

/**
 * 将 DbConnectConfigType 转换为 ConnectionDialog 的 initialData
 */
function toConnectionInitialData(data: DbConnectConfigType): Partial<ConnectionFormData> {
  return {
    name: '', // 面板内编辑不需要名称，但 dialog 要求
    driver: (data.driver as ConnectionFormData['driver']) || 'postgres',
    host: data.host,
    port: data.port,
    database: data.database,
    username: data.username,
    password: data.password,
    connection_string: data.connection_string,
    min_connections: data.min_connections ?? 1,
    max_connections: data.max_connections ?? 10,
  };
}

export function DbConnectConfig({ data, onSave }: DbConnectConfigProps) {
  const [showDialog, setShowDialog] = useState(false);

  const {
    register,
    handleSubmit,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<DbConnectConfigFormData>({
    resolver: zodResolver(dbConnectConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 追踪表单实际修改状态（useRef 用于卸载时读取）
  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  // 追踪待执行的 setTimeout，组件卸载时清除，防止保存到错误的节点
  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

  // 组件卸载时同步保存（仅当用户实际修改过表单时）
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

  // 当外部数据变化时，重置表单
  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  /**
   * 保存表单数据
   */
  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 DB Connect 配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  /**
   * 处理失焦事件
   */
  const handleBlur = useCallback(() => {
    safeSetTimeout(() => {
      if (!isActuallyDirtyRef.current) return;
      saveData();
    }, 100);
  }, [saveData]);

  // 监听 Ctrl+S 刷新事件，将表单数据同步到 store
  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  /**
   * ConnectionDialog 保存回调 — 将对话框数据合并到节点配置
   */
  const handleDialogSave = useCallback((dialogData: ConnectionFormData) => {
    const config: DbConnectConfigType = {
      type: 'db_connect',
      driver: dialogData.driver,
      connection_string: dialogData.connection_string,
      host: dialogData.host,
      port: dialogData.port,
      database: dialogData.database,
      username: dialogData.username,
      password: dialogData.password,
      min_connections: dialogData.min_connections,
      max_connections: dialogData.max_connections,
      // 保留原有的 test_query 和 connect_timeout_ms
      test_query: data.test_query,
      connect_timeout_ms: data.connect_timeout_ms,
    };
    onSave(config);
    // 同步重置表单
    reset(toFormData(config));
  }, [data.test_query, data.connect_timeout_ms, onSave, reset]);

  const hasConnection = !!data.driver;
  const connectionTarget = getConnectionTarget(data);

  return (
    <>
      <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
        {/* 连接摘要卡片 */}
        {hasConnection ? (
          <div className="rounded-lg border border-indigo-200/60 dark:border-indigo-800/60 bg-gradient-to-br from-indigo-50/80 dark:from-indigo-950/40 to-sky-50/50 dark:to-sky-950/30 p-4 space-y-3 overflow-hidden">
            {/* 驱动类型徽标 */}
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md border text-xs font-semibold ${getDriverBadgeBg(data.driver!)}`}>
                  <Database className={`h-3.5 w-3.5 ${getDriverColor(data.driver!)}`} />
                  <span className={getDriverColor(data.driver!)}>{getDriverLabel(data.driver!)}</span>
                </div>
              </div>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => setShowDialog(true)}
                className="h-7 gap-1.5 text-xs text-muted-foreground hover:text-foreground"
              >
                <Settings2 className="h-3.5 w-3.5" />
                修改配置
              </Button>
            </div>

            {/* 连接详情 */}
            <div className="space-y-2">
              {connectionTarget && (
                <div className="flex items-center gap-2 text-sm">
                  <Server className="h-3.5 w-3.5 shrink-0 text-indigo-500" />
                  <span className="font-mono text-xs text-indigo-800 dark:text-indigo-300 truncate" title={connectionTarget}>
                    {connectionTarget}
                  </span>
                </div>
              )}
              {data.username && (
                <div className="flex items-center gap-2 text-sm">
                  <User className="h-3.5 w-3.5 shrink-0 text-indigo-400" />
                  <span className="text-xs text-indigo-700/80 dark:text-indigo-400/80">{data.username}</span>
                </div>
              )}
              {(data.min_connections != null || data.max_connections != null) && (
                <div className="flex items-center gap-2 text-sm">
                  <FileText className="h-3.5 w-3.5 shrink-0 text-indigo-400" />
                  <span className="text-xs text-indigo-700/60 dark:text-indigo-400/60">
                    连接池 {data.min_connections ?? 1} - {data.max_connections ?? 10}
                  </span>
                </div>
              )}
            </div>
          </div>
        ) : (
          /* 未配置状态 */
          <div className="rounded-lg border border-dashed border-muted-foreground/30 bg-muted/20 p-5 space-y-3">
            <div className="flex items-center justify-center gap-2 text-muted-foreground">
              <Database className="h-5 w-5" />
              <span className="text-sm font-medium">未配置数据库连接</span>
            </div>
            <p className="text-center text-xs text-muted-foreground/70">
              配置数据库驱动、连接参数和认证信息
            </p>
            <div className="flex justify-center">
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => setShowDialog(true)}
                className="gap-1.5"
              >
                <Settings2 className="h-3.5 w-3.5" />
                配置数据库连接
              </Button>
            </div>
          </div>
        )}

        {/* 已配置时的修改按钮（在摘要卡片下方再放一个主按钮，方便操作） */}
        {hasConnection && (
          <Button
            type="button"
            variant="outline"
            className="w-full gap-2"
            onClick={() => setShowDialog(true)}
          >
            <Settings2 className="h-4 w-4" />
            配置数据库连接
          </Button>
        )}

        {/* 连接超时 */}
        <div className="space-y-2">
          <Label htmlFor="connect_timeout_ms" className="text-sm font-medium text-foreground">
            连接超时
          </Label>
          <Input
            id="connect_timeout_ms"
            type="number"
            min={1000}
            max={300000}
            {...register('connect_timeout_ms', { valueAsNumber: true })}
          />
          <p className="text-xs text-muted-foreground">毫秒 (ms)，范围 1000-300000，默认 30000</p>
          {errors.connect_timeout_ms && (
            <p className="text-xs text-rose-500">{errors.connect_timeout_ms.message}</p>
          )}
        </div>

        {/* 测试查询 */}
        <div className="space-y-2">
          <Label htmlFor="test_query" className="text-sm font-medium text-foreground">
            测试查询（可选）
          </Label>
          <Textarea
            id="test_query"
            {...register('test_query')}
            placeholder="SELECT 1"
            className="min-h-[80px] resize-y font-mono text-xs"
            rows={3}
          />
          <p className="text-xs text-muted-foreground">
            连接成功后执行的测试查询，用于验证连接有效性
          </p>
          {errors.test_query && (
            <p className="text-xs text-rose-500">{errors.test_query.message}</p>
          )}
        </div>
      </form>

      {/* 受控连接配置对话框 */}
      <ConnectionDialog
        open={showDialog}
        onOpenChange={setShowDialog}
        onSave={handleDialogSave}
        initialData={toConnectionInitialData(data)}
      />
    </>
  );
}
