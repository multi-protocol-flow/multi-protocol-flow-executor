/**
 * ImapFetch 节点组件 - IMAP 获取邮件节点
 * 蓝色主题，表示获取邮件内容
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Download } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ImapFetchConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectCurrentNodeUuid, selectIsPaused, selectIsDebugging } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';

/** ImapFetch 节点数据类型 */
export interface ImapFetchNodeData {
  label?: string;
  config?: ImapFetchConfig;
}

export function ImapFetchNode({ id, data, selected }: NodeProps) {
  const nodeData = data as ImapFetchNodeData;
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
        'border-2 border-border', 'bg-background',
        'transition-all duration-200 ease-out', 'w-[180px]', 'flex items-center',
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
          <Download className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-blue-600 font-medium tracking-wide">{getNodeDisplayName('imap_fetch')}</span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || getNodeDisplayName('imap_fetch')}
          </span>
          {nodeData.config?.uid_set && (
            <span className="text-[10px] text-muted-foreground truncate">
              UID: {nodeData.config.uid_set}
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
}
