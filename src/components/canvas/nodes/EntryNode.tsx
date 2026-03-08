/**
 * Entry 节点组件 - 流程入口点
 * 绿色主题，表示流程的开始
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Play } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useFlowStore, useDebugStore, selectCurrentNodeUuid, selectIsPaused } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';

/** Entry 节点数据类型 */
export interface EntryNodeData {
  label?: string;
  config?: unknown;
}

/**
 * Entry 节点 - 流程入口
 * 无输入端口，单个输出端口 "out"
 */
export function EntryNode({ id, data, selected }: NodeProps) {
  const nodeData = data as EntryNodeData;
  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const currentNodeUuid = useDebugStore(selectCurrentNodeUuid);
  const isPaused = useDebugStore(selectIsPaused);
  const isDebugPausedAtThisNode = isPaused && currentNodeUuid === id;
  
  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-black',
        'bg-white',
        'transition-all duration-200 ease-out',
        'w-[180px]',
        'flex items-center',
        selected && 'border-[3px] border-emerald-500',
        isDebugPausedAtThisNode && 'debug-breathing-emerald'
      )}
    >
      <BreakpointMarker
        hasBreakpoint={hasBreakpoint}
        onToggle={() => toggleBreakpoint(id)}
      />
      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-emerald-500 text-white'
        )}>
          <Play className="w-3.5 h-3.5 fill-current" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-emerald-600 font-medium uppercase tracking-wide">
            入口
          </span>
          <span className="text-sm font-semibold text-black break-words">
            {nodeData.label || '流程入口'}
          </span>
        </div>
      </div>
      
      <Handle
        type="source"
        position={Position.Right}
        id="out"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-white !border-2 !border-black',
          '![background:radial-gradient(circle,_theme(colors.emerald.500)_35%,_white_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150'
        )}
      />
    </div>
  );
}
