/**
 * DbConnect 节点组件 - 数据库连接节点
 * 紫色主题，表示建立数据库连接
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Database } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { DbConnectConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectIsNodeRunning, selectIsDebugging, selectIsNodeDebugPaused } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { getLL } from '@/i18n/i18n-lazy';
import { memo } from 'react';

/** DbConnect 节点数据类型 */
export interface DbConnectNodeData {
  label?: string;
  config?: DbConnectConfig;
}

/** 驱动显示名称映射 */
export const DRIVER_LABELS: Record<string, string> = {
  postgres: 'PostgreSQL',
  mysql: 'MySQL',
  sqlite: 'SQLite',
};

/** 根据配置生成连接摘要 */
function getConnectionSummary(config?: DbConnectConfig): string | null {
  if (!config?.driver) return null;
  const driver = config.driver;

  if (driver === 'sqlite') {
    return config.connection_string || ':memory:';
  }

  const parts: string[] = [];
  if (config.host) {
    parts.push(config.host);
    if (config.port) {
      parts[parts.length - 1] = `${config.host}:${config.port}`;
    }
  }
  if (config.database) {
    parts.push(config.database);
  }

  return parts.length > 0 ? parts.join('/') : null;
}

export const DbConnectNode = memo(function DbConnectNode({ id, data, selected }: NodeProps) {
  const nodeData = data as DbConnectNodeData;
  const config = nodeData.config;
  const driverLabel = config?.driver ? (DRIVER_LABELS[config.driver] ?? config.driver) : null;
  const connectionSummary = getConnectionSummary(config);
  const LL = getLL();
  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const isRunning = useExecutionStore(selectIsNodeRunning(id));
  const isDebugging = useDebugStore(selectIsDebugging);
  const isDebugPaused = useDebugStore(selectIsNodeDebugPaused(id));
  const isDebugPausedAtThisNode = isRunning || isDebugPaused;
  
  const notConfiguredLabel = LL.common.not_configured();

  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-border',
        'bg-background',
        'transition-transform duration-200 ease-out',
        'w-[180px]',
        'flex items-center',
        selected && 'border-[3px] border-violet-500',
        isDebugPausedAtThisNode && 'debug-breathing-violet'
      )}
    >
      {isDebugging && (
        <BreakpointMarker
          hasBreakpoint={hasBreakpoint}
          onToggle={() => toggleBreakpoint(id)}
        />
      )}
      <Handle
        type="target"
        position={Position.Left}
        id="in"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-background !border-2 !border-border',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-violet-500 before:rounded-full"
        )}
      />
      
      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-violet-500 text-white'
        )}>
          <Database className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-violet-600 font-medium uppercase tracking-wide">
            {driverLabel ?? 'DB'}
          </span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || getNodeDisplayName('db_connect')}
          </span>
          {driverLabel ? (
            <span className="text-[10px] text-muted-foreground truncate">
              {connectionSummary ?? notConfiguredLabel}
            </span>
          ) : (
            <span className="text-[10px] text-muted-foreground">{notConfiguredLabel}</span>
          )}
        </div>
      </div>
      
      <Handle
        type="source"
        position={Position.Right}
        id="true"
        className={cn(
          '!w-3.5 !h-3.5 !top-[30%]',
          '!bg-background !border-2 !border-emerald-500 dark:!border-emerald-400',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-emerald-600 dark:before:bg-emerald-500 before:rounded-full"
        )}
      />
      
      <Handle
        type="source"
        position={Position.Right}
        id="false"
        className={cn(
          '!w-3.5 !h-3.5 !top-[70%]',
          '!bg-background !border-2 !border-rose-500 dark:!border-rose-400',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-rose-600 dark:before:bg-rose-500 before:rounded-full"
        )}
      /></div>
  );
});