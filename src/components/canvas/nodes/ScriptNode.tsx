/**
 * Script 节点组件 - 脚本执行节点
 * 紫色主题，表示自定义脚本执行
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Code } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ScriptNodeConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectIsNodeRunning, selectIsDebugging, selectIsNodeDebugPaused } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { memo } from 'react';
import { NodeTitle } from './NodeTitle';

/** Script 节点数据类型 */
export interface ScriptNodeData {
  label?: string;
  config?: ScriptNodeConfig;
}

/**
 * Script 节点 - 脚本执行
 * 输入端口 "input"，两个输出端口 "true"(上) 和 "false"(下)
 */
export const ScriptNode = memo(function ScriptNode({ id, data, selected }: NodeProps) {
  const nodeData = data as ScriptNodeData;
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
        'border-2 border-border',
        'bg-background',
        'w-[180px]',
        'flex items-center',
        selected && 'border-violet-500 ring-2 ring-violet-500/20',
        isDebugPausedAtThisNode && 'debug-breathing-violet'
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
          '!w-3.5 !h-3.5 !top-1/2',
          '!bg-background !border-2 !border-border',
          '![background:radial-gradient(circle,_theme(colors.violet.500)_35%,_transparent_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150'
        )}
      />
      
      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-indigo-500 text-white'
        )}>
          <Code className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-indigo-600 font-medium uppercase tracking-wide">
            {getNodeDisplayName('script')}
          </span>
          <NodeTitle
            nodeId={id}
            label={nodeData.label}
            defaultLabel={getNodeDisplayName('script')}
          />
        </div>
      </div>
      
      <Handle
        type="source"
        position={Position.Right}
        id="true"
        className={cn(
          '!w-3.5 !h-3.5 !top-[30%]',
          '!bg-background !border-2 !border-emerald-500 dark:!border-emerald-400',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-emerald-600 dark:before:bg-emerald-500 before:rounded-full"
        )}
      />
      
      <Handle
        type="source"
        position={Position.Right}
        id="false"
        className={cn(
          '!w-3.5 !h-3.5 !top-[70%]',
          '!bg-background !border-2 !border-rose-500 dark:!border-rose-400',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-rose-600 dark:before:bg-rose-500 before:rounded-full"
        )}
      />
    </div>
  );
});