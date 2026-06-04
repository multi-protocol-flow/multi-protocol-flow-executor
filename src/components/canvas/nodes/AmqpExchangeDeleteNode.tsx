/**
 * AmqpExchangeDelete 节点组件 - AMQP 0-9-1 删除交换器节点
 * 紫罗兰色主题，表示删除 AMQP 交换器
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Trash2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { AmqpExchangeDeleteConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectRunningNodeUuid, selectIsExecutingFlag, selectIsDebugging, selectDebugCurrentNodeUuid } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { memo } from 'react';

/** AmqpExchangeDelete 节点数据类型 */
export interface AmqpExchangeDeleteNodeData {
  label?: string;
  config?: AmqpExchangeDeleteConfig;
}

/**
 * AmqpExchangeDelete 节点 - AMQP 0-9-1 删除交换器
 * 输入端口 "in"，两个输出端口 "true"(上) 和 "false"(下)
 */
export const AmqpExchangeDeleteNode = memo(function AmqpExchangeDeleteNode({ id, data, selected }: NodeProps) {
  const nodeData = data as AmqpExchangeDeleteNodeData;
  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const runningNodeUuid = useExecutionStore(selectRunningNodeUuid);
  const isExecuting = useExecutionStore(selectIsExecutingFlag);
  const isRunning = isExecuting && runningNodeUuid === id;
  const debugPausedNodeUuid = useDebugStore(selectDebugCurrentNodeUuid);
  const isDebuggingStatus = useDebugStore(selectIsDebugging);
  const isDebugPaused = isDebuggingStatus && debugPausedNodeUuid === id;
  const isDebugPausedAtThisNode = isRunning || isDebugPaused;
  const isDebugging = useDebugStore(selectIsDebugging);
  
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
          "before:content-[''] before:absolute before:inset-[3px] before:bg-violet-500 before:rounded-full"
        )}
      />
      
      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-violet-500 text-white'
        )}>
          <Trash2 className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-violet-600 font-medium tracking-wide">
            {getNodeDisplayName('amqp_exchange_delete')}
          </span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || getNodeDisplayName('amqp_exchange_delete')}
          </span>
          {nodeData.config?.exchange && (
            <span className="text-[10px] text-muted-foreground truncate">
              {nodeData.config.exchange}
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
          '!bg-background !border-2 !border-rose-500 dark:!border-rose-400',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-rose-600 dark:before:bg-rose-500 before:rounded-full"
        )}
      /></div>
  );
});