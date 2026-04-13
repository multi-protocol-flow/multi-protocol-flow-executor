/**
 * DB Begin 节点配置组件
 * 开始数据库事务，支持隔离级别配置
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo } from 'react';
import {
  dbBeginConfigSchema,
  type DbBeginConfigFormData,
  type DbIsolationLevelFormData,
} from './schemas';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { ArrowRight } from 'lucide-react';
import type { DbBeginConfig as DbBeginConfigType } from '@/types/node';
import { useConnectionsStore, selectDbConnections } from '@/stores/connections';

interface DbBeginConfigProps {
  /** 当前节点数据 */
  data: DbBeginConfigType;
  /** 保存回调 */
  onSave: (data: DbBeginConfigType) => void;
}

/**
 * 隔离级别选项
 */
const ISOLATION_LEVEL_OPTIONS: { value: DbIsolationLevelFormData; label: string }[] = [
  { value: 'default', label: '默认' },
  { value: 'read_uncommitted', label: '读未提交 (Read Uncommitted)' },
  { value: 'read_committed', label: '读已提交 (Read Committed)' },
  { value: 'repeatable_read', label: '可重复读 (Repeatable Read)' },
  { value: 'serializable', label: '串行化 (Serializable)' },
];

/**
 * 将 DbBeginConfigType 转换为表单数据
 */
function toFormData(data: DbBeginConfigType): DbBeginConfigFormData {
  return {
    type: 'db_begin',
    connection_uuid: data.connection_uuid || '',
    isolation_level: data.isolation_level,
  };
}

/**
 * 将表单数据转换为 DbBeginConfigType
 */
function fromFormData(formData: DbBeginConfigFormData): DbBeginConfigType {
  return {
    type: 'db_begin',
    connection_uuid: formData.connection_uuid,
    isolation_level: formData.isolation_level,
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

export function DbBeginConfig({ data, onSave }: DbBeginConfigProps) {
  const {
    handleSubmit,
    watch,
    setValue,
    reset,
    formState: { errors },
  } = useForm<DbBeginConfigFormData>({
    resolver: zodResolver(dbBeginConfigSchema),
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
        console.error('保存 DB Begin 配置失败:', error);
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
   * 处理隔离级别选择变更
   */
  const handleIsolationLevelChange = useCallback((value: string) => {
    setValue('isolation_level', value as DbIsolationLevelFormData);
    saveData();
  }, [setValue, saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 - 橙色主题 */}
      <div className="rounded-lg border border-orange-200/50 bg-gradient-to-br from-orange-50/80 to-amber-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-orange-900 flex items-center gap-2">
            <ArrowRight className="h-4 w-4" />
            开始事务节点
          </h4>
          <p className="text-sm leading-relaxed text-orange-700/90 whitespace-normal break-words">
            开始一个数据库事务。可以指定事务隔离级别，默认使用数据库默认隔离级别。
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

      {/* 隔离级别选择 */}
      <div className="space-y-2">
        <Label htmlFor="isolation_level" className="text-sm font-medium text-foreground">
          隔离级别
        </Label>
        <Select
          value={watch('isolation_level') || 'default'}
          onValueChange={handleIsolationLevelChange}
        >
          <SelectTrigger id="isolation_level">
            <SelectValue placeholder="选择事务隔离级别" />
          </SelectTrigger>
          <SelectContent>
            {ISOLATION_LEVEL_OPTIONS.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          设置事务的隔离级别，默认使用数据库默认级别
        </p>
      </div>

      {/* 选中连接的只读信息卡片 */}
      {selectedConnection && (
        <div className="rounded-lg border border-orange-100 bg-orange-50/30 p-4 space-y-3">
          <h5 className="text-xs font-semibold text-orange-800 uppercase tracking-wide">
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
    </form>
  );
}
