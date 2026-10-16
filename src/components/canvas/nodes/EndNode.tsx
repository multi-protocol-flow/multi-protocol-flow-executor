/**
 * End 节点组件 - 流程结束点
 * 紫色主题，表示流程的终点
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Square } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useDebugStore, useExecutionStore, selectIsNodeRunning, selectIsDebugging, selectIsNodeDebugPaused } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { memo } from 'react';
import { NodeTitle } from './NodeTitle';

/** End 节点数据类型 */
export interface EndNodeData {
  label?: string;
  config?: unknown;
}

/**
 * End 节点 - 流程终点
 * 左侧输入端口，无输出端口
 */
export const EndNode = memo(function EndNode({ id, data, selected }: NodeProps) {
  const nodeData = data as EndNodeData;
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
        'w-[180px]',
        'flex items-center',
        selected && 'border-[3px] border-purple-500',
        isDebugPausedAtThisNode && 'debug-breathing-purple'
      )}
    >
      {/* End 节点：调试模式下显示锁定断点（不可取消） */}
      {isDebugging && <BreakpointMarker hasBreakpoint={true} locked />}
      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-purple-500 text-white'
        )}>
          <Square className="w-3.5 h-3.5 fill-current" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-purple-600 font-medium uppercase tracking-wide">
            {getNodeDisplayName('end')}
          </span>
          <NodeTitle
            nodeId={id}
            label={nodeData.label}
            defaultLabel={getNodeDisplayName('end')}
          />
        </div>
      </div>

      <Handle
        type="target"
        position={Position.Left}
        id="in"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-background !border-2 !border-border',
          '![background:radial-gradient(circle,_theme(colors.purple.500)_35%,_transparent_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150'
        )}
      />
    </div>
  );
});