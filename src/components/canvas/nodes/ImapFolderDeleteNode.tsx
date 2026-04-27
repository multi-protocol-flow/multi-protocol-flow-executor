/**
 * ImapFolderDelete 节点组件 - IMAP 删除文件夹节点
 * 蓝色主题，表示删除邮箱文件夹
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { FolderMinus } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ImapFolderDeleteConfig } from '@/types';
import { useFlowStore, useDebugStore, selectCurrentNodeUuid, selectIsPaused, selectIsDebugging } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';

export interface ImapFolderDeleteNodeData {
  label?: string;
  config?: ImapFolderDeleteConfig;
}

export function ImapFolderDeleteNode({ id, data, selected }: NodeProps) {
  const nodeData = data as ImapFolderDeleteNodeData;
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
          <FolderMinus className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-blue-600 font-medium tracking-wide">IMAP-删除文件夹</span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || '删除文件夹'}
          </span>
          {nodeData.config?.folder_name && (
            <span className="text-[10px] text-muted-foreground truncate">
              {nodeData.config.folder_name}
            </span>
          )}
        </div>
      </div>
      <Handle
        type="source" position={Position.Right} id="true"
        className={cn(
          '!w-3.5 !h-3.5 !top-[30%]', '!bg-background !border-2 !border-border',
          'hover:!scale-125', 'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-blue-500 before:rounded-full"
        )}
      />
      <Handle
        type="source" position={Position.Right} id="false"
        className={cn(
          '!w-3.5 !h-3.5 !top-[70%]', '!bg-background !border-2 !border-border',
          'hover:!scale-125', 'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-blue-500 before:rounded-full"
        )}
      />
      <div className="absolute right-0 top-0 bottom-0 flex flex-col justify-center translate-x-5 pointer-events-none">
        <span className="text-[10px] font-bold text-emerald-600 leading-none -mt-2">✓</span>
        <span className="text-[10px] font-bold text-rose-600 leading-none mt-4">✗</span>
      </div>
    </div>
  );
}
