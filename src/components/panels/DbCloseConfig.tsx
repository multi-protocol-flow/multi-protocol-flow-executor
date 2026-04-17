/**
 * DB Close 节点配置组件
 * 关闭数据库连接
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import {
  dbCloseConfigSchema,
  type DbCloseConfigFormData,
} from './schemas';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { XCircle } from 'lucide-react';
import type { DbCloseConfig as DbCloseConfigType, DbConnectConfig as DbConnectConfigType } from '@/types/node';
import { useFlowStore } from '@/stores/flow';

interface DbCloseConfigProps {
  /** 当前节点数据 */
  data: DbCloseConfigType;
  /** 保存回调 */
  onSave: (data: DbCloseConfigType) => void;
}

/**
 * 将 DbCloseConfigType 转换为表单数据
 */
function toFormData(data: DbCloseConfigType): DbCloseConfigFormData {
  return {
    type: 'db_close',
    connection_uuid: data.connection_uuid || '',
  };
}

/**
 * 将表单数据转换为 DbCloseConfigType
 */
function fromFormData(formData: DbCloseConfigFormData): DbCloseConfigType {
  return {
    type: 'db_close',
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

export function DbCloseConfig({ data, onSave }: DbCloseConfigProps) {
  const {
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<DbCloseConfigFormData>({
    resolver: zodResolver(dbCloseConfigSchema),
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
        console.error('保存 DB Close 配置失败:', error);
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
      {/* 主说明卡片 - 灰色主题 */}
      <div className="rounded-lg border border-border/50 bg-gradient-to-br from-muted/80 to-muted/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-foreground flex items-center gap-2">
            <XCircle className="h-4 w-4" />
            关闭连接节点
          </h4>
          <p className="text-sm leading-relaxed text-muted-foreground whitespace-normal break-words">
            关闭指定的数据库连接，释放相关资源。
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
