/**
 * DB Query 节点配置组件
 * 支持 SQL 编辑、参数化查询、结果映射等功能
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import {
  dbQueryConfigSchema,
  type DbQueryConfigFormData,
  type DbFetchModeFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { Button } from '@/components/ui/button';
import { Database, Info, AlertTriangle, Plus, Trash2 } from 'lucide-react';
import type { DbQueryConfig as DbQueryConfigType, DbConnectConfig as DbConnectConfigType } from '@/types/node';
import { useFlowStore, selectNodes } from '@/stores/flow';
import { getLL } from '@/i18n/i18n-lazy';

interface DbQueryConfigProps {
  /** 当前节点数据 */
  data: DbQueryConfigType;
  /** 保存回调 */
  onSave: (data: DbQueryConfigType) => void;
}

/**
 * 危险操作正则表达式
 */
const DANGEROUS_OPERATIONS = /\b(INSERT|UPDATE|DELETE|DROP|TRUNCATE|ALTER|CREATE)\b/i;

/**
 * 检测 SQL 中是否包含危险操作
 */
function detectDangerousOperations(sql: string): string[] {
  const operations: string[] = [];
  const match = sql.match(DANGEROUS_OPERATIONS);
  if (match) {
    // 找出所有危险操作
    const found = new Set<string>();
    let m: RegExpExecArray | null;
    const regex = new RegExp(DANGEROUS_OPERATIONS.source, 'gi');
    while ((m = regex.exec(sql)) !== null) {
      found.add(m[1].toUpperCase());
    }
    operations.push(...Array.from(found));
  }
  return operations;
}

/**
 * 将 DbQueryConfigType 转换为表单数据
 */
function toFormData(data: DbQueryConfigType): DbQueryConfigFormData {
  return {
    type: 'db_query',
    connection_uuid: data.connection_uuid || '',
    sql: data.sql || '',
    params: data.params || [],
    fetch_mode: data.fetch_mode || 'all',
    result_to: data.result_to || '',
    timeout_ms: data.timeout_ms ?? 30000,
    // Redis 字段不在此任务中实现，但需要保留
    redis_command: data.redis_command,
    redis_key: data.redis_key,
    redis_value: data.redis_value,
    redis_ttl: data.redis_ttl,
  };
}

/**
 * 将表单数据转换为 DbQueryConfigType
 */
function fromFormData(formData: DbQueryConfigFormData): DbQueryConfigType {
  return {
    type: 'db_query',
    connection_uuid: formData.connection_uuid,
    sql: formData.sql && formData.sql.trim() ? formData.sql.trim() : undefined,
    params: formData.params && formData.params.length > 0 ? formData.params : undefined,
    fetch_mode: formData.fetch_mode,
    result_to: formData.result_to && formData.result_to.trim() ? formData.result_to.trim() : undefined,
    timeout_ms: formData.timeout_ms,
    // Redis 字段保留
    redis_command: formData.redis_command,
    redis_key: formData.redis_key,
    redis_value: formData.redis_value,
    redis_ttl: formData.redis_ttl,
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

export function DbQueryConfig({ data, onSave }: DbQueryConfigProps) {
  const {
    register,
    handleSubmit,
    control,
    watch,
    setValue,
    getValues,
    reset,
    formState: { errors, isDirty },
  } = useForm<DbQueryConfigFormData>({
    resolver: zodResolver(dbQueryConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 获取当前流程中的 db_connect 节点
  const nodes = useFlowStore(selectNodes);
  const dbConnectNodes = useMemo(
    () => nodes.filter((n) => n.data?.type === 'db_connect'),
    [nodes],
  );

  // 参数化查询的 useFieldArray
  const paramsFieldArray = useFieldArray({
    control,
    name: 'params' as never,
  });

  // 当前选中的连接 UUID
  const selectedConnectionUuid = watch('connection_uuid');

  // 当前 SQL
  const currentSql = watch('sql') || '';

  // 检测危险操作
  const dangerousOperations = useMemo(() => {
    return detectDangerousOperations(currentSql);
  }, [currentSql]);

  // 当外部数据变化时，重置表单
  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

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

  /**
   * 安全的延迟执行包装器，注册定时器以便卸载时清理
   */
  const safeSetTimeout = useCallback((fn: () => void, ms: number) => {
    const id = setTimeout(() => {
      pendingTimersRef.current.delete(id);
      fn();
    }, ms);
    pendingTimersRef.current.add(id);
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

  /**
   * 保存表单数据
   */
  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 DB Query 配置失败:', error);
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
      }, [safeSetTimeout, saveData]);

  // 监听 Ctrl+S 刷新事件，将表单数据同步到 store
  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  /**
   * 处理连接选择变更
   */
  const handleConnectionChange = useCallback((value: string) => {
    setValue('connection_uuid', value, { shouldDirty: true });
    saveData();
  }, [setValue, saveData]);

  /**
   * 处理 fetch_mode 变更
   */
  const handleFetchModeChange = useCallback((value: string) => {
    setValue('fetch_mode', value as DbFetchModeFormData, { shouldDirty: true });
    saveData();
  }, [setValue, saveData]);

  /**
   * {getLL().panels.dbQuery.addParam()}
   */
  const handleAddParam = useCallback(() => {
    paramsFieldArray.append('');
  }, [paramsFieldArray]);

  /**
   * 删除参数
   */
  const handleRemoveParam = useCallback((index: number) => {
    paramsFieldArray.remove(index);
    safeSetTimeout(saveData, 100);
  }, [paramsFieldArray, saveData, safeSetTimeout]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-purple-200/50 bg-gradient-to-br from-purple-50/80 to-violet-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-purple-900 flex items-center gap-2">
            <Database className="h-4 w-4" />
            {getLL().panels.dbQuery.title()}
          </h4>
          <p className="text-sm leading-relaxed text-purple-700/90 whitespace-normal break-words">
            {getLL().panels.dbQuery.description()}
          </p>
        </div>
      </div>

      {/* 连接选择下拉框 */}
      <div className="space-y-2">
        <Label htmlFor="connection_uuid" className="text-sm font-medium text-foreground">
          {getLL().panels.dbCommon.databaseConnection()} <span className="text-rose-500">*</span>
        </Label>
        <Select
          value={selectedConnectionUuid || ''}
          onValueChange={handleConnectionChange}
        >
          <SelectTrigger id="connection_uuid" className="font-mono">
            <SelectValue placeholder={getLL().panels.dbCommon.selectConnectionPlaceholder()} />
          </SelectTrigger>
          <SelectContent>
            {dbConnectNodes.length === 0 ? (
              <div className="px-2 py-4 text-center text-sm text-muted-foreground">
                {getLL().panels.dbCommon.noConnectionNodes()}
              </div>
            ) : (
              dbConnectNodes.map((node) => {
                const config = node.data as DbConnectConfigType;
                const driver = config.driver || '';
                const host = config.host || '';
                const port = config.port ? `:${config.port}` : '';
                return (
                  <SelectItem key={node.uuid} value={node.uuid}>
                    <div className="flex items-center gap-1.5 flex-nowrap min-w-0">
                      <span className="shrink-0">{node.name}</span>
                      <span className={`shrink-0 font-medium text-xs ${getDriverColor(driver)}`}>
                        [{getDriverLabel(driver || getLL().panels.notSet())}]
                      </span>
                      {host && (
                        <span className="text-muted-foreground text-xs truncate" title={`${host}${port}`}>
                          {host}{port}
                        </span>
                      )}
                    </div>
                  </SelectItem>
                );
              })
            )}
          </SelectContent>
        </Select>
        {errors.connection_uuid && (
          <p className="text-xs text-rose-500">{errors.connection_uuid.message}</p>
        )}
        {dbConnectNodes.length === 0 && (
          <p className="text-xs text-amber-600">
            {getLL().panels.dbCommon.addConnectionNodeFirst()}
          </p>
        )}
      </div>

      {/* SQL 编辑区域 */}
      <div className="space-y-2">
        <Label htmlFor="sql" className="text-sm font-medium text-foreground">
          {getLL().panels.dbQuery.sqlQuery()}
        </Label>
        <Textarea
          id="sql"
          {...register('sql')}
          placeholder="SELECT * FROM users WHERE id = ?"
          className="min-h-[120px] resize-y font-mono text-xs"
          rows={6}
        />
        {errors.sql && (
          <p className="text-xs text-rose-500">{errors.sql.message}</p>
        )}
        <p className="text-xs text-muted-foreground">
          {getLL().panels.dbQuery.sqlPlaceholderHelp()}
        </p>
      </div>

      {/* 危险操作警告 */}
      {dangerousOperations.length > 0 && (
        <div className="rounded-lg border border-orange-200 bg-gradient-to-r from-orange-50 to-amber-50 p-4">
          <div className="flex items-start gap-3">
            <AlertTriangle className="h-5 w-5 text-orange-600 shrink-0 mt-0.5" />
            <div className="space-y-1">
              <h5 className="text-sm font-semibold text-orange-800">
                {getLL().panels.dbQuery.dangerousOpsDetected()}
              </h5>
              <p className="text-sm text-orange-700">
                <span className="font-medium">{getLL().panels.dbQuery.operationType()}</span>
                {dangerousOperations.join(' / ')}
              </p>
              <p className="text-xs text-orange-600/90">
                {getLL().panels.dbQuery.dangerousOpsWarning()}
              </p>
            </div>
          </div>
        </div>
      )}

      {/* 参数化查询 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">{getLL().panels.dbQuery.queryParamsOptional()}</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddParam}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            {getLL().panels.dbQuery.addParam()}
          </Button>
        </div>
        <div className="space-y-2">
          {paramsFieldArray.fields.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">{getLL().panels.dbQuery.noQueryParams()}</p>
              <p className="mt-1 text-xs text-muted-foreground/60">
                {getLL().panels.dbQuery.clickAddParamHint()}
              </p>
            </div>
          ) : (
            paramsFieldArray.fields.map((field, index) => (
              <div key={field.id} className="flex items-center gap-2">
                <span className="text-xs text-muted-foreground w-6 shrink-0">
                  {index + 1}.
                </span>
                <Input
                  {...register(`params.${index}`)}
                  placeholder={getLL().panels.dbQuery.paramValue()}
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  onClick={() => handleRemoveParam(index)}
                  className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            ))
          )}
        </div>
        {paramsFieldArray.fields.length > 0 && (
          <p className="text-xs text-muted-foreground">
            {getLL().panels.dbQuery.paramOrderHelp()}
          </p>
        )}
      </div>

      {/* fetch_mode 选择 */}
      <div className="space-y-2">
        <Label htmlFor="fetch_mode" className="text-sm font-medium text-foreground">
          {getLL().panels.dbQuery.fetchMode()}
        </Label>
        <Select
          value={watch('fetch_mode') || 'all'}
          onValueChange={handleFetchModeChange}
        >
          <SelectTrigger id="fetch_mode" className="font-mono">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">{getLL().panels.dbQuery.fetchModeAll()}</SelectItem>
            <SelectItem value="one">{getLL().panels.dbQuery.fetchModeOne()}</SelectItem>
            <SelectItem value="scalar">{getLL().panels.dbQuery.fetchModeScalar()}</SelectItem>
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          {getLL().panels.dbQuery.fetchModeHelp()}
        </p>
      </div>

      {/* 查询超时 */}
      <div className="space-y-2">
        <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground">
          {getLL().panels.dbQuery.queryTimeout()}
        </Label>
        <Input
          id="timeout_ms"
          type="number"
          min={1000}
          max={300000}
          {...register('timeout_ms', { valueAsNumber: true })}
        />
        <p className="text-xs text-muted-foreground">{getLL().panels.dbConnect.timeoutHint()}</p>
        {errors.timeout_ms && (
          <p className="text-xs text-rose-500">{errors.timeout_ms.message}</p>
        )}
      </div>

      {/* 结果存储变量名 */}
      <div className="space-y-2">
        <Label htmlFor="result_to" className="text-sm font-medium text-foreground">
          {getLL().panels.dbQuery.resultVariableOptional()}
        </Label>
        <Input
          id="result_to"
          {...register('result_to')}
          placeholder="query_result"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          {getLL().panels.dbQuery.resultVariableHelp('{{变量名}}')}
        </p>
        {errors.result_to && (
          <p className="text-xs text-rose-500">{errors.result_to.message}</p>
        )}
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-sky-50/60 dark:bg-sky-950/40 border border-sky-200/50 dark:border-sky-700/40 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500 dark:text-sky-400" />
        <div className="text-xs text-sky-700/90 dark:text-sky-300/90 space-y-1">
          <p className="font-medium text-sky-800 dark:text-sky-200">{getLL().panels.dbQuery.usageNotes()}</p>
          <p>{getLL().panels.dbQuery.usageStep1()}</p>
          <p>{getLL().panels.dbQuery.usageStep2()}</p>
          <p>{getLL().panels.dbQuery.usageStep3()}</p>
          <p>{getLL().panels.dbQuery.usageStep4()}</p>
        </div>
      </div>
    </form>
  );
}
