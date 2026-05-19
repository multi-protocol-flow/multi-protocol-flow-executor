/**
 * DbConnect 节点组件 - 数据库连接节点
 * 紫色主题，表示建立数据库连接
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Database } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { DbConnectConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectCurrentNodeUuid, selectIsPaused, selectIsDebugging } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';

/** DbConnect 节点数据类型 */
export interface DbConnectNodeData {
  label?: string;
  config?: DbConnectConfig;
}

/** 驱动显示名称映射 */
const DRIVER_LABELS: Record<string, string> = {
  postgres: 'PostgreSQL',
  mysql: 'MySQL',
  sqlite: 'SQLite',
  redis: 'Redis',
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

export function DbConnectNode({ id, data, selected }: NodeProps) {
  const nodeData = data as DbConnectNodeData;
  const config = nodeData.config;
  const driverLabel = config?.driver ? (DRIVER_LABELS[config.driver] ?? config.driver) : null;
  const connectionSummary = getConnectionSummary(config);
  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const currentNodeUuid = useDebugStore(selectCurrentNodeUuid);
  const isPaused = useDebugStore(selectIsPaused);
  const isExecuting = useExecutionStore(s => s.isExecuting);
  const isDebugPausedAtThisNode = (isPaused || isExecuting) && currentNodeUuid === id;
  const isDebugging = useDebugStore(selectIsDebugging);
  
  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-border',
        'bg-background',
        'transition-all duration-200 ease-out',
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
              {connectionSummary ?? '未配置'}
            </span>
          ) : (
            <span className="text-[10px] text-muted-foreground">未配置</span>
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
}
