/**
 * DB Query 节点配置组件
 * 支持 SQL 编辑、参数化查询、结果映射等功能
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo } from 'react';
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
import { Database, Info, AlertTriangle, ArrowRightFromLine, Plus, Trash2 } from 'lucide-react';
import type { DbQueryConfig as DbQueryConfigType, DbConnectConfig as DbConnectConfigType } from '@/types/node';
import { useFlowStore } from '@/stores/flow';

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
    output_mappings: data.output_mappings || [],
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
    output_mappings: formData.output_mappings && formData.output_mappings.length > 0 ? formData.output_mappings : undefined,
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

/**
 * fetch_mode 选项
 */
const FETCH_MODE_OPTIONS = [
  { value: 'all', label: '获取所有行 (all)' },
  { value: 'one', label: '获取单行 (one)' },
  { value: 'scalar', label: '获取单个值 (scalar)' },
] as const;

export function DbQueryConfig({ data, onSave }: DbQueryConfigProps) {
  const {
    register,
    handleSubmit,
    control,
    watch,
    setValue,
    reset,
    formState: { errors },
  } = useForm<DbQueryConfigFormData>({
    resolver: zodResolver(dbQueryConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 获取当前流程中的 db_connect 节点
  const nodes = useFlowStore((state) => state.flow?.nodes ?? []);
  const dbConnectNodes = useMemo(
    () => nodes.filter((n) => n.data?.type === 'db_connect'),
    [nodes],
  );

  // 参数化查询的 useFieldArray
  const paramsFieldArray = useFieldArray({
    control,
    name: 'params' as never,
  });

  // output_mappings 的 useFieldArray
  const outputMappingsFieldArray = useFieldArray({
    control,
    name: 'output_mappings' as never,
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
    setTimeout(saveData, 100);
  }, [saveData]);

  /**
   * 处理连接选择变更
   */
  const handleConnectionChange = useCallback((value: string) => {
    setValue('connection_uuid', value);
    saveData();
  }, [setValue, saveData]);

  /**
   * 处理 fetch_mode 变更
   */
  const handleFetchModeChange = useCallback((value: string) => {
    setValue('fetch_mode', value as DbFetchModeFormData);
    saveData();
  }, [setValue, saveData]);

  /**
   * 添加参数
   */
  const handleAddParam = useCallback(() => {
    paramsFieldArray.append('');
  }, [paramsFieldArray]);

  /**
   * 删除参数
   */
  const handleRemoveParam = useCallback((index: number) => {
    paramsFieldArray.remove(index);
    setTimeout(saveData, 100);
  }, [paramsFieldArray, saveData]);

  /**
   * 添加输出映射
   */
  const handleAddOutputMapping = useCallback(() => {
    outputMappingsFieldArray.append({ source: '', target: '', description: '' });
  }, [outputMappingsFieldArray]);

  /**
   * 删除输出映射
   */
  const handleRemoveOutputMapping = useCallback((index: number) => {
    outputMappingsFieldArray.remove(index);
    setTimeout(saveData, 100);
  }, [outputMappingsFieldArray, saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-purple-200/50 bg-gradient-to-br from-purple-50/80 to-violet-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-purple-900 flex items-center gap-2">
            <Database className="h-4 w-4" />
            数据库查询节点
          </h4>
          <p className="text-sm leading-relaxed text-purple-700/90 whitespace-normal break-words">
            执行 SQL 查询语句，支持参数化查询和结果映射。查询结果可存储到变量供后续节点使用。
          </p>
        </div>
      </div>

      {/* 连接选择下拉框 */}
      <div className="space-y-2">
        <Label htmlFor="connection_uuid" className="text-sm font-medium text-foreground">
          数据库连接 <span className="text-rose-500">*</span>
        </Label>
        <Select
          value={selectedConnectionUuid || ''}
          onValueChange={handleConnectionChange}
        >
          <SelectTrigger id="connection_uuid" className="font-mono">
            <SelectValue placeholder="选择流程中的数据库连接节点" />
          </SelectTrigger>
          <SelectContent>
            {dbConnectNodes.length === 0 ? (
              <div className="px-2 py-4 text-center text-sm text-muted-foreground">
                无可用连接节点
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
                        [{getDriverLabel(driver || '未设置')}]
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
            请先在流程中添加数据库连接节点
          </p>
        )}
      </div>

      {/* SQL 编辑区域 */}
      <div className="space-y-2">
        <Label htmlFor="sql" className="text-sm font-medium text-foreground">
          SQL 查询语句
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
          支持 ? 占位符进行参数化查询，参数在下方配置
        </p>
      </div>

      {/* 危险操作警告 */}
      {dangerousOperations.length > 0 && (
        <div className="rounded-lg border border-orange-200 bg-gradient-to-r from-orange-50 to-amber-50 p-4">
          <div className="flex items-start gap-3">
            <AlertTriangle className="h-5 w-5 text-orange-600 shrink-0 mt-0.5" />
            <div className="space-y-1">
              <h5 className="text-sm font-semibold text-orange-800">
                检测到修改操作
              </h5>
              <p className="text-sm text-orange-700">
                <span className="font-medium">操作类型：</span>
                {dangerousOperations.join(' / ')}
              </p>
              <p className="text-xs text-orange-600/90">
                此操作将修改数据库数据，请谨慎执行。建议在执行前备份数据。
              </p>
            </div>
          </div>
        </div>
      )}

      {/* 参数化查询 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">查询参数（可选）</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddParam}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            添加参数
          </Button>
        </div>
        <div className="space-y-2">
          {paramsFieldArray.fields.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">暂无查询参数</p>
              <p className="mt-1 text-xs text-muted-foreground/60">
                点击「添加参数」配置参数化查询
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
                  placeholder="参数值"
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
            参数按顺序对应 SQL 中的 ? 占位符
          </p>
        )}
      </div>

      {/* fetch_mode 选择 */}
      <div className="space-y-2">
        <Label htmlFor="fetch_mode" className="text-sm font-medium text-foreground">
          获取模式
        </Label>
        <Select
          value={watch('fetch_mode') || 'all'}
          onValueChange={handleFetchModeChange}
        >
          <SelectTrigger id="fetch_mode" className="font-mono">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {FETCH_MODE_OPTIONS.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          all=返回所有行，one=返回第一行，scalar=返回单个值
        </p>
      </div>

      {/* 查询超时 */}
      <div className="space-y-2">
        <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground">
          查询超时
        </Label>
        <Input
          id="timeout_ms"
          type="number"
          min={1000}
          max={300000}
          {...register('timeout_ms', { valueAsNumber: true })}
        />
        <p className="text-xs text-muted-foreground">毫秒 (ms)，范围 1000-300000，默认 30000</p>
        {errors.timeout_ms && (
          <p className="text-xs text-rose-500">{errors.timeout_ms.message}</p>
        )}
      </div>

      {/* 结果存储变量名 */}
      <div className="space-y-2">
        <Label htmlFor="result_to" className="text-sm font-medium text-foreground">
          结果存储变量名（可选）
        </Label>
        <Input
          id="result_to"
          {...register('result_to')}
          placeholder="query_result"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          查询结果将存储到此变量，可通过 {'{{变量名}}'} 在后续节点中引用
        </p>
        {errors.result_to && (
          <p className="text-xs text-rose-500">{errors.result_to.message}</p>
        )}
      </div>

      {/* 输出变量映射 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <ArrowRightFromLine className="h-4 w-4 text-muted-foreground" />
            <Label className="text-sm font-medium text-foreground">输出变量映射（可选）</Label>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddOutputMapping}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            添加映射
          </Button>
        </div>

        {/* 映射列表 */}
        <div className="space-y-2">
          {outputMappingsFieldArray.fields.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">暂无输出映射</p>
              <p className="mt-1 text-xs text-muted-foreground/60">
                点击「添加映射」配置变量提取规则
              </p>
            </div>
          ) : (
            outputMappingsFieldArray.fields.map((field, index) => {
              const targetError = errors.output_mappings?.[index]?.target;
              const sourceError = errors.output_mappings?.[index]?.source;

              // 检查变量名是否重复
              const outputMappingsList = watch('output_mappings') || [];
              const currentTarget = outputMappingsList[index]?.target || '';
              const duplicateTargets = outputMappingsList
                .map((m, i) => (m.target && i !== index && m.target === currentTarget ? i : -1))
                .filter(i => i !== -1);
              const hasDuplicate = duplicateTargets.length > 0 && currentTarget.trim() !== '';

              // JSONPath 常用预设
              const jsonPathPresets = [
                { value: '$.rows', label: '所有行' },
                { value: '$.rows[0]', label: '第一行' },
                { value: '$.rows[0].id', label: '第一行ID' },
                { value: '$.affected', label: '影响行数' },
              ];

              return (
                <div key={field.id} className="space-y-2 p-3 rounded-md border bg-muted/30">
                  {/* 第一行：变量名 + 删除按钮 */}
                  <div className="flex items-center gap-2">
                    <Input
                      {...register(`output_mappings.${index}.target`)}
                      placeholder="变量名（字母、数字、下划线）"
                      className={`h-8 flex-1 font-mono text-xs ${targetError || hasDuplicate ? 'border-rose-500 focus-visible:ring-rose-500' : ''}`}
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => handleRemoveOutputMapping(index)}
                      className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>

                  {/* 第二行：源字段 */}
                  <div className="relative">
                    <Input
                      {...register(`output_mappings.${index}.source`)}
                      placeholder="源字段（如 $.rows[0].name）"
                      className={`h-8 w-full font-mono text-xs ${sourceError ? 'border-rose-500 focus-visible:ring-rose-500' : ''}`}
                      list={`jsonpath-presets-dbquery-${index}`}
                    />
                    <datalist id={`jsonpath-presets-dbquery-${index}`}>
                      {jsonPathPresets.map((preset) => (
                        <option key={preset.value} value={preset.value} />
                      ))}
                    </datalist>
                  </div>

                  {/* 错误提示 */}
                  {sourceError && (
                    <p className="text-xs text-rose-500">{sourceError.message}</p>
                  )}
                  {targetError && (
                    <p className="text-xs text-rose-500">{targetError.message}</p>
                  )}
                  {hasDuplicate && !targetError && (
                    <p className="text-xs text-rose-500">变量名已存在</p>
                  )}

                  {/* 第三行：说明 */}
                  <Input
                    {...register(`output_mappings.${index}.description`)}
                    placeholder="说明（可选）"
                    className="h-8 w-full text-xs"
                  />
                </div>
              );
            })
          )}
        </div>

        {/* 提示信息 */}
        {outputMappingsFieldArray.fields.length > 0 && (
          <div className="flex items-start gap-2 rounded-md bg-muted/30 p-2">
            <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
            <div className="text-xs text-muted-foreground space-y-1">
              <p className="font-medium text-foreground">JSONPath 语法：</p>
              <p><code className="rounded bg-muted px-1">$.rows</code> 所有行</p>
              <p><code className="rounded bg-muted px-1">$.rows[0]</code> 第一行</p>
              <p><code className="rounded bg-muted px-1">$.rows[0].id</code> 第一行的 id 字段</p>
              <p><code className="rounded bg-muted px-1">$.affected</code> 影响的行数</p>
              <p className="text-muted-foreground/70 mt-1">变量名：字母、数字、下划线，1-64 字符</p>
            </div>
          </div>
        )}
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-sky-50/50 border border-sky-200/50 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-sky-700/90 space-y-1">
          <p className="font-medium text-sky-800">使用说明：</p>
          <p>1. 选择已建立的数据库连接</p>
          <p>2. 编写 SQL 查询语句，支持 ? 占位符</p>
          <p>3. 可选配置参数化查询和结果映射</p>
          <p>4. 查询结果可存储到变量供后续节点使用</p>
          <p className="text-sky-600/70 mt-2">
            提示：使用 output_mappings 可精确提取查询结果中的特定字段
          </p>
        </div>
      </div>
    </form>
  );
}
