/**
 * ImapMove 节点组件 - IMAP 移动邮件节点
 * 蓝色主题，表示移动邮件到目标邮箱
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { ArrowRightLeft } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ImapMoveConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectIsNodeRunning, selectIsDebugging, selectIsNodeDebugPaused } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { memo } from 'react';

export interface ImapMoveNodeData {
  label?: string;
  config?: ImapMoveConfig;
}

export const ImapMoveNode = memo(function ImapMoveNode({ id, data, selected }: NodeProps) {
  const nodeData = data as ImapMoveNodeData;
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
          <ArrowRightLeft className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-blue-600 font-medium tracking-wide">{getNodeDisplayName('imap_move')}</span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || getNodeDisplayName('imap_move')}
          </span>
          {nodeData.config?.destination_mailbox && (
            <span className="text-[10px] text-muted-foreground truncate">
              → {nodeData.config.destination_mailbox}
            </span>
          )}
        </div>
      </div>
      <Handle
        type="source" position={Position.Right} id="true"
        className={cn(
          '!w-3.5 !h-3.5 !top-[30%]', '!bg-background !border-2 !border-emerald-500 dark:!border-emerald-400',
          'hover:!scale-125', 'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-emerald-600 dark:before:bg-emerald-500 before:rounded-full"
        )}
      />
      <Handle
        type="source" position={Position.Right} id="false"
        className={cn(
          '!w-3.5 !h-3.5 !top-[70%]', '!bg-background !border-2 !border-rose-500 dark:!border-rose-400',
          'hover:!scale-125', 'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-rose-600 dark:before:bg-rose-500 before:rounded-full"
        )}
      /></div>
  );
});