/**
 * AmqpClose 节点组件 - AMQP 0-9-1 关闭连接节点
 * 紫罗兰色主题，表示关闭 AMQP 连接
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Plug } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { AmqpCloseConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectRunningNodeUuid, selectIsExecutingFlag, selectIsDebugging, selectDebugCurrentNodeUuid } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { memo } from 'react';

/** AmqpClose 节点数据类型 */
export interface AmqpCloseNodeData {
  label?: string;
  config?: AmqpCloseConfig;
}

/**
 * AmqpClose 节点 - AMQP 0-9-1 关闭连接
 * 输入端口 "in"，输出端口 "out"
 */
export const AmqpCloseNode = memo(function AmqpCloseNode({ id, data, selected }: NodeProps) {
  const nodeData = data as AmqpCloseNodeData;
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
          <Plug className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-violet-600 font-medium tracking-wide">
            {getNodeDisplayName('amqp_close')}
          </span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || getNodeDisplayName('amqp_close')}
          </span>
          {nodeData.config?.connection_id && (
            <span className="text-[10px] text-muted-foreground truncate">
              {nodeData.config.connection_id}
            </span>
          )}
        </div>
      </div>
      
      <Handle
        type="source"
        position={Position.Right}
        id="out"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-background !border-2 !border-border',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-violet-500 before:rounded-full"
        )}
      />
    </div>
  );
});