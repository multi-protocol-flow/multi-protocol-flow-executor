/**
 * SseDisconnect 节点组件 - SSE 断开节点
 * 玫红色主题，表示断开 SSE 连接
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Unplug } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { SseDisconnectConfig } from '@/types';
import { useFlowStore, useDebugStore, selectCurrentNodeUuid, selectIsPaused, selectIsDebugging } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';

/** SseDisconnect 节点数据类型 */
export interface SseDisconnectNodeData {
  label?: string;
  config?: SseDisconnectConfig;
}

/**
 * SseDisconnect 节点 - SSE 断开
 * 输入端口 "in"，单输出端口 "out"
 */
export function SseDisconnectNode({ id, data, selected }: NodeProps) {
  const nodeData = data as SseDisconnectNodeData;
  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const currentNodeUuid = useDebugStore(selectCurrentNodeUuid);
  const isPaused = useDebugStore(selectIsPaused);
  const isDebugPausedAtThisNode = isPaused && currentNodeUuid === id;
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
        selected && 'border-[3px] border-rose-500',
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
          "before:content-[''] before:absolute before:inset-[3px] before:bg-rose-500 before:rounded-full"
        )}
      />
      
      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-rose-500 text-white'
        )}>
          <Unplug className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-rose-600 font-medium tracking-wide">
            SSE-断开
          </span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || 'SSE 断开'}
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
          "before:content-[''] before:absolute before:inset-[3px] before:bg-rose-500 before:rounded-full"
        )}
      />
    </div>
  );
}
