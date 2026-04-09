/**
 * WsClose 节点组件 - WebSocket 关闭节点
 * 玫瑰色主题，表示关闭 WebSocket 连接
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Plug } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { WsCloseConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectCurrentNodeUuid, selectIsPaused } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';

/** WsClose 节点数据类型 */
export interface WsCloseNodeData {
  label?: string;
  config?: WsCloseConfig;
}

/**
 * WsClose 节点 - WebSocket 关闭
 * 输入端口 "in"，两个输出端口 "true"(上) 和 "false"(下)
 */
export function WsCloseNode({ id, data, selected }: NodeProps) {
  const nodeData = data as WsCloseNodeData;
  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const currentNodeUuid = useDebugStore(selectCurrentNodeUuid);
  const isPaused = useDebugStore(selectIsPaused);
  const isDebugPausedAtThisNode = isPaused && currentNodeUuid === id;
  const getNodeExecutionStatus = useExecutionStore((state) => state.getNodeExecutionStatus);
  const executionStatus = getNodeExecutionStatus(id);
  const executionClassName = executionStatus !== 'pending' ? `execution-${executionStatus}` : '';
  
  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-black',
        'bg-white',
        'transition-all duration-200 ease-out',
        'w-[180px]',
        'flex items-center',
        selected && 'border-[3px] border-rose-500',
        isDebugPausedAtThisNode && 'debug-breathing-rose',
        executionClassName
      )}
    >
      <BreakpointMarker
        hasBreakpoint={hasBreakpoint}
        onToggle={() => toggleBreakpoint(id)}
      />
      <Handle
        type="target"
        position={Position.Left}
        id="in"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-white !border-2 !border-black',
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
          <Plug className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-rose-600 font-medium uppercase tracking-wide">
            WS
          </span>
          <span className="text-sm font-semibold text-black break-words">
            {nodeData.label || 'WS 关闭'}
          </span>
        </div>
      </div>
      
      <Handle
        type="source"
        position={Position.Right}
        id="true"
        className={cn(
          '!w-3.5 !h-3.5 !top-[30%]',
          '!bg-white !border-2 !border-black',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-rose-500 before:rounded-full"
        )}
      />
      
      <Handle
        type="source"
        position={Position.Right}
        id="false"
        className={cn(
          '!w-3.5 !h-3.5 !top-[70%]',
          '!bg-white !border-2 !border-black',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-rose-500 before:rounded-full"
        )}
      />
      
      <div className="absolute right-0 top-0 bottom-0 flex flex-col justify-center translate-x-5 pointer-events-none">
        <span className="text-[10px] font-bold text-emerald-600 leading-none -mt-2">T</span>
        <span className="text-[10px] font-bold text-rose-600 leading-none mt-4">F</span>
      </div>
    </div>
  );
}
