/**
 * Entry 节点组件 - 流程入口点
 * 绿色主题，表示流程的开始
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Play } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useDebugStore, useExecutionStore, selectIsNodeRunning, selectIsDebugging, selectIsNodeDebugPaused } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { memo } from 'react';

/** Entry 节点数据类型 */
export interface EntryNodeData {
  label?: string;
  config?: unknown;
}

/**
 * Entry 节点 - 流程入口
 * 无输入端口，单个输出端口 "out"
 */
export const EntryNode = memo(function EntryNode({ id, data, selected }: NodeProps) {
  const nodeData = data as EntryNodeData;
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
        selected && 'border-[3px] border-emerald-500',
        isDebugPausedAtThisNode && 'debug-breathing-emerald'
      )}
    >
      {/* 入口节点：调试模式下显示锁定断点（不可取消） */}
      {isDebugging && <BreakpointMarker hasBreakpoint={true} locked />}
      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-emerald-500 text-white'
        )}>
          <Play className="w-3.5 h-3.5 fill-current" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-emerald-600 font-medium uppercase tracking-wide">
            {getNodeDisplayName('entry')}
          </span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || getNodeDisplayName('entry')}
          </span>
        </div>
      </div>
      
      <Handle
        type="source"
        position={Position.Right}
        id="out"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-background !border-2 !border-border',
          '![background:radial-gradient(circle,_theme(colors.emerald.500)_35%,_transparent_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150'
        )}
      />
    </div>
  );
});