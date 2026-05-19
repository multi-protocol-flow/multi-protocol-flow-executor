/**
 * Amqp10Receive 节点组件 - AMQP 1.0 接收消息节点
 * 深紫色主题，表示接收 AMQP 1.0 消息
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Inbox } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { Amqp10ReceiveConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectCurrentNodeUuid, selectIsPaused, selectIsDebugging } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';

/** Amqp10Receive 节点数据类型 */
export interface Amqp10ReceiveNodeData {
  label?: string;
  config?: Amqp10ReceiveConfig;
}

/**
 * Amqp10Receive 节点 - AMQP 1.0 接收消息
 * 输入端口 "in"，两个输出端口 "true"(上) 和 "false"(下)
 */
export function Amqp10ReceiveNode({ id, data, selected }: NodeProps) {
  const nodeData = data as Amqp10ReceiveNodeData;
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
        selected && 'border-[3px] border-purple-500',
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
          "before:content-[''] before:absolute before:inset-[3px] before:bg-purple-500 before:rounded-full"
        )}
      />
      
      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-purple-500 text-white'
        )}>
          <Inbox className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-purple-600 font-medium tracking-wide">
            {getNodeDisplayName('amqp10_receive')}
          </span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || getNodeDisplayName('amqp10_receive')}
          </span>
          {nodeData.config?.address && (
            <span className="text-[10px] text-muted-foreground truncate">
              {nodeData.config.address}
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
}
