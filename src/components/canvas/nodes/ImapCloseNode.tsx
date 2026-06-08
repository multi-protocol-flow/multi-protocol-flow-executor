/**
 * ImapClose 节点组件 - IMAP 关闭连接节点
 * 蓝色主题，表示关闭 IMAP 连接
 * 输入端口 "in"，单输出端口 "out"
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Unplug } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ImapCloseConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectIsNodeRunning, selectIsDebugging, selectIsNodeDebugPaused } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { memo } from 'react';

export interface ImapCloseNodeData {
  label?: string;
  config?: ImapCloseConfig;
}

export const ImapCloseNode = memo(function ImapCloseNode({ id, data, selected }: NodeProps) {
  const nodeData = data as ImapCloseNodeData;
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
        'border-2 border-border', 'bg-background',
        'transition-transform duration-200 ease-out', 'w-[180px]', 'flex items-center',
        selected && 'border-[3px] border-blue-500',
        isDebugPausedAtThisNode && 'debug-breathing-purple'
      )}
    >
      {isDebugging && (
        <BreakpointMarker hasBreakpoint={hasBreakpoint} onToggle={() => toggleBreakpoint(id)} />
      )}
      <Handle
        type="target" position={Position.Left} id="in"
        className={cn(
          '!w-3.5 !h-3.5', '!bg-background !border-2 !border-border',
          'hover:!scale-125', 'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-blue-500 before:rounded-full"
        )}
      />
      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn('shrink-0 p-1.5 rounded-lg', 'bg-blue-500 text-white')}>
          <Unplug className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-blue-600 font-medium tracking-wide">{getNodeDisplayName('imap_close')}</span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || getNodeDisplayName('imap_close')}
          </span>
        </div>
      </div>
      <Handle
        type="source" position={Position.Right} id="out"
        className={cn(
          '!w-3.5 !h-3.5', '!bg-background !border-2 !border-border',
          'hover:!scale-125', 'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-blue-500 before:rounded-full"
        )}
      />
    </div>
  );
});