/**
 * DB Commit 节点配置组件
 * 提交数据库事务
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import {
  dbCommitConfigSchema,
  type DbCommitConfigFormData,
} from './schemas';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Check } from 'lucide-react';
import type { DbCommitConfig as DbCommitConfigType, DbConnectConfig as DbConnectConfigType } from '@/types/node';
import { useFlowStore } from '@/stores/flow';

interface DbCommitConfigProps {
  /** 当前节点数据 */
  data: DbCommitConfigType;
  /** 保存回调 */
  onSave: (data: DbCommitConfigType) => void;
}

/**
 * 将 DbCommitConfigType 转换为表单数据
 */
function toFormData(data: DbCommitConfigType): DbCommitConfigFormData {
  return {
    type: 'db_commit',
    connection_uuid: data.connection_uuid || '',
  };
}

/**
 * 将表单数据转换为 DbCommitConfigType
 */
function fromFormData(formData: DbCommitConfigFormData): DbCommitConfigType {
  return {
    type: 'db_commit',
    connection_uuid: formData.connection_uuid,
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

export function DbCommitConfig({ data, onSave }: DbCommitConfigProps) {
  const {
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<DbCommitConfigFormData>({
    resolver: zodResolver(dbCommitConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 获取当前流程中的 db_connect 节点
  const nodes = useFlowStore((state) => state.flow?.nodes ?? []);
  const dbConnectNodes = useMemo(
    () => nodes.filter((n) => n.data?.type === 'db_connect'),
    [nodes],
  );

  // 当前选中的连接 UUID
  const selectedConnectionUuid = watch('connection_uuid');

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

  // 当外部数据变化时，重置表单
  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

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
        console.error('保存 DB Commit 配置失败:', error);
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

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 - 绿色主题 */}
      <div className="rounded-lg border border-green-200/50 bg-gradient-to-br from-green-50/80 to-emerald-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-green-900 flex items-center gap-2">
            <Check className="h-4 w-4" />
            提交事务节点
          </h4>
          <p className="text-sm leading-relaxed text-green-700/90 whitespace-normal break-words">
            提交当前数据库事务，使事务中的所有更改永久生效。
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
    </form>
  );
}
