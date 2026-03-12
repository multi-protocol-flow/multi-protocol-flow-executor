/**
 * DB Connect 节点配置组件
 * 支持连接选择、超时配置、测试查询
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo } from 'react';
import {
  dbConnectConfigSchema,
  type DbConnectConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { Database, Info } from 'lucide-react';
import type { DbConnectConfig as DbConnectConfigType } from '@/types/node';
import { useConnectionsStore, selectDbConnections } from '@/stores/connections';

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
    connection_uuid: data.connection_uuid || '',
    test_query: data.test_query || '',
    connect_timeout_ms: data.connect_timeout_ms ?? 30000,
  };
}

/**
 * 将表单数据转换为 DbConnectConfigType
 */
function fromFormData(formData: DbConnectConfigFormData): DbConnectConfigType {
  return {
    type: 'db_connect',
    connection_uuid: formData.connection_uuid,
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

export function DbConnectConfig({ data, onSave }: DbConnectConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    formState: { errors },
  } = useForm<DbConnectConfigFormData>({
    resolver: zodResolver(dbConnectConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 获取连接列表
  const connections = useConnectionsStore(selectDbConnections);

  // 当前选中的连接 UUID
  const selectedConnectionUuid = watch('connection_uuid');

  // 查找选中的连接
  const selectedConnection = useMemo(() => {
    return connections.find((c) => c.id === selectedConnectionUuid);
  }, [connections, selectedConnectionUuid]);

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
    setTimeout(saveData, 100);
  }, [saveData]);

  /**
   * 处理连接选择变更
   */
  const handleConnectionChange = useCallback((value: string) => {
    setValue('connection_uuid', value);
    saveData();
  }, [setValue, saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-purple-200/50 bg-gradient-to-br from-purple-50/80 to-violet-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-purple-900 flex items-center gap-2">
            <Database className="h-4 w-4" />
            数据库连接节点
          </h4>
          <p className="text-sm leading-relaxed text-purple-700/90 whitespace-normal break-words">
            建立数据库连接，支持 PostgreSQL、MySQL、SQLite、Redis。连接成功后可在后续节点中执行查询操作。
          </p>
        </div>
      </div>

      {/* 连接选择下拉框 */}
      <div className="space-y-2">
        <Label htmlFor="connection_uuid" className="text-sm font-medium text-black">
          数据库连接 <span className="text-rose-500">*</span>
        </Label>
        <Select
          value={selectedConnectionUuid || ''}
          onValueChange={handleConnectionChange}
        >
          <SelectTrigger id="connection_uuid" className="font-mono">
            <SelectValue placeholder="选择已配置的数据库连接" />
          </SelectTrigger>
          <SelectContent>
            {connections.length === 0 ? (
              <div className="px-2 py-4 text-center text-sm text-muted-foreground">
                暂无可用的数据库连接
              </div>
            ) : (
              connections.map((conn) => (
                <SelectItem key={conn.id} value={conn.id}>
                  <div className="flex items-center gap-2">
                    <span className={`font-medium ${getDriverColor(conn.driver)}`}>
                      [{getDriverLabel(conn.driver)}]
                    </span>
                    <span>{conn.name}</span>
                  </div>
                </SelectItem>
              ))
            )}
          </SelectContent>
        </Select>
        {errors.connection_uuid && (
          <p className="text-xs text-rose-500">{errors.connection_uuid.message}</p>
        )}
        {connections.length === 0 && (
          <p className="text-xs text-amber-600">
            请先在设置中配置数据库连接
          </p>
        )}
      </div>

      {/* 选中连接的只读信息卡片 */}
      {selectedConnection && (
        <div className="rounded-lg border border-purple-100 bg-purple-50/30 p-4 space-y-3">
          <h5 className="text-xs font-semibold text-purple-800 uppercase tracking-wide">
            连接信息
          </h5>
          <div className="grid grid-cols-2 gap-3 text-sm">
            {/* 数据库类型 */}
            <div>
              <span className="text-muted-foreground text-xs">数据库类型</span>
              <p className={`font-medium ${getDriverColor(selectedConnection.driver)}`}>
                {getDriverLabel(selectedConnection.driver)}
              </p>
            </div>
            {/* 连接状态 */}
            <div>
              <span className="text-muted-foreground text-xs">状态</span>
              <p className={`font-medium ${
                selectedConnection.status === 'connected' ? 'text-green-600' :
                selectedConnection.status === 'error' ? 'text-red-600' :
                selectedConnection.status === 'testing' ? 'text-amber-600' :
                'text-gray-500'
              }`}>
                {selectedConnection.status === 'connected' ? '已连接' :
                 selectedConnection.status === 'error' ? '连接错误' :
                 selectedConnection.status === 'testing' ? '测试中' :
                 '未连接'}
              </p>
            </div>
            {/* 主机和端口 */}
            {selectedConnection.host && (
              <div>
                <span className="text-muted-foreground text-xs">主机</span>
                <p className="font-mono text-xs">
                  {selectedConnection.host}
                  {selectedConnection.port && `:${selectedConnection.port}`}
                </p>
              </div>
            )}
            {/* 数据库名 */}
            {selectedConnection.database && (
              <div>
                <span className="text-muted-foreground text-xs">数据库</span>
                <p className="font-mono text-xs">{selectedConnection.database}</p>
              </div>
            )}
            {/* SQLite 文件路径 */}
            {selectedConnection.driver === 'sqlite' && selectedConnection.connection_string && (
              <div className="col-span-2">
                <span className="text-muted-foreground text-xs">文件路径</span>
                <p className="font-mono text-xs truncate" title={selectedConnection.connection_string}>
                  {selectedConnection.connection_string}
                </p>
              </div>
            )}
            {/* 用户名 */}
            {selectedConnection.username && (
              <div>
                <span className="text-muted-foreground text-xs">用户名</span>
                <p className="font-mono text-xs">{selectedConnection.username}</p>
              </div>
            )}
          </div>
          {/* 错误信息 */}
          {selectedConnection.last_error && (
            <div className="mt-2 p-2 rounded bg-red-50 border border-red-100">
              <p className="text-xs text-red-600">
                <span className="font-medium">错误: </span>
                {selectedConnection.last_error}
              </p>
            </div>
          )}
        </div>
      )}

      {/* 连接超时 */}
      <div className="space-y-2">
        <Label htmlFor="connect_timeout_ms" className="text-sm font-medium text-black">
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
        <Label htmlFor="test_query" className="text-sm font-medium text-black">
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

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-sky-50/50 border border-sky-200/50 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-sky-700/90 space-y-1">
          <p className="font-medium text-sky-800">使用说明：</p>
          <p>1. 选择已配置的数据库连接</p>
          <p>2. 可选配置连接超时和测试查询</p>
          <p>3. 连接成功后，使用 DB Query 节点执行查询</p>
          <p className="text-sky-600/70 mt-2">
            提示：连接管理在设置页面进行配置
          </p>
        </div>
      </div>
    </form>
  );
}
