/**
 * TcpClose 节点组件 - TCP 关闭节点
 * 青绿色主题，表示关闭 TCP 连接
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Unplug } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { TcpCloseConfig } from '@/types';
import { useFlowStore, useDebugStore, selectCurrentNodeUuid, selectIsPaused, selectIsDebugging } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';

/** TcpClose 节点数据类型 */
export interface TcpCloseNodeData {
  label?: string;
  config?: TcpCloseConfig;
}

/**
 * TcpClose 节点 - TCP 关闭
 * 输入端口 "in"，两个输出端口 "true"(上) 和 "false"(下)
 */
export function TcpCloseNode({ id, data, selected }: NodeProps) {
  const nodeData = data as TcpCloseNodeData;
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
        selected && 'border-[3px] border-emerald-500',
        isDebugPausedAtThisNode && 'debug-breathing-emerald'
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
          "before:content-[''] before:absolute before:inset-[3px] before:bg-emerald-500 before:rounded-full"
        )}
      />
      
      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-emerald-500 text-white'
        )}>
          <Unplug className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-emerald-600 font-medium tracking-wide">
            TCP-关闭
          </span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || 'TCP 关闭'}
          </span>
        </div>
      </div>
      
      <Handle
        type="source"
        position={Position.Right}
        id="true"
        className={cn(
          '!w-3.5 !h-3.5 !top-[30%]',
          '!bg-background !border-2 !border-border',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-emerald-500 before:rounded-full"
        )}
      />
      
      <Handle
        type="source"
        position={Position.Right}
        id="false"
        className={cn(
          '!w-3.5 !h-3.5 !top-[70%]',
          '!bg-background !border-2 !border-border',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-emerald-500 before:rounded-full"
        )}
      />
      
      <div className="absolute right-0 top-0 bottom-0 flex flex-col justify-center translate-x-5 pointer-events-none">
        <span className="text-[10px] font-bold text-emerald-600 leading-none -mt-2">✓</span>
        <span className="text-[10px] font-bold text-rose-600 leading-none mt-4">✗</span>
      </div>
    </div>
  );
}
