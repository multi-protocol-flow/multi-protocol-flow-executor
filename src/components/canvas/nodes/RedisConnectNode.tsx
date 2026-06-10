/**
 * RedisConnect 节点组件 - Redis 连接节点
 * 红色主题，表示建立 Redis 连接
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Radio } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { RedisConnectConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectIsNodeRunning, selectIsDebugging, selectIsNodeDebugPaused } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { memo } from 'react';

/** RedisConnect 节点数据类型 */
export interface RedisConnectNodeData {
  label?: string;
  config?: RedisConnectConfig;
}

/**
 * RedisConnect 节点 - Redis 连接
 * 输入端口 "in"，两个输出端口 "true"(上) 和 "false"(下)
 */
export const RedisConnectNode = memo(function RedisConnectNode({ id, data, selected }: NodeProps) {
  const nodeData = data as RedisConnectNodeData;
  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const isRunning = useExecutionStore(selectIsNodeRunning(id));
  const isDebugging = useDebugStore(selectIsDebugging);
  const isDebugPaused = useDebugStore(selectIsNodeDebugPaused(id));
  const isDebugPausedAtThisNode = isRunning || isDebugPaused;

  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-border',
        'bg-background',
        'transition-transform duration-200 ease-out',
        'w-[180px]',
        'flex items-center',
        selected && 'border-[3px] border-red-500',
        isDebugPausedAtThisNode && 'debug-breathing-purple'
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
          "before:content-[''] before:absolute before:inset-[3px] before:bg-red-500 before:rounded-full"
        )}
      />

      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-red-500 text-white'
        )}>
          <Radio className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-red-600 font-medium tracking-wide">
            {getNodeDisplayName('redis_connect')}
          </span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || getNodeDisplayName('redis_connect')}
          </span>
          {nodeData.config?.host && (
            <span className="text-[10px] text-muted-foreground truncate">
              {nodeData.config.host}:{nodeData.config.port ?? 6379}
            </span>
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
          '!bg-background !border-2 !border-red-500 dark:!border-red-400',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-red-600 dark:before:bg-red-500 before:rounded-full"
        )}
      /></div>
  );
});
