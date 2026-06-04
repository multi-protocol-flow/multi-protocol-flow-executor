/**
 * VariableExtractor 节点组件 - 变量提取节点
 * 琥珀色主题，表示从响应中提取变量
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Braces } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { VariableExtractorConfig } from '@/types/node';
import { useFlowStore, useDebugStore, useExecutionStore, selectIsNodeRunning, selectIsNodeDebugPaused, selectIsDebugging } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { memo } from 'react';

/** VariableExtractor 节点数据类型 */
export interface VariableExtractorNodeData {
  label?: string;
  config?: VariableExtractorConfig;
}

/**
 * VariableExtractor 节点 - 变量提取
 * 单输入端口 "in"，单输出端口 "out"
 */
export const VariableExtractorNode = memo(function VariableExtractorNode({ id, data, selected }: NodeProps) {
  const nodeData = data as VariableExtractorNodeData;
  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const isRunning = useExecutionStore(selectIsNodeRunning(id));
  const isDebugPaused = useDebugStore(selectIsNodeDebugPaused(id));
  const isDebugPausedAtThisNode = isRunning || isDebugPaused;
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
        selected && 'border-[3px] border-amber-500',
        isDebugPausedAtThisNode && 'debug-breathing-amber'
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
          '![background:radial-gradient(circle,_theme(colors.amber.500)_35%,_transparent_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150'
        )}
      />

      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-amber-500 text-white'
        )}>
          <Braces className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-amber-600 font-medium uppercase tracking-wide">
            {getNodeDisplayName('variable_extractor')}
          </span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || getNodeDisplayName('variable_extractor')}
          </span>
        </div>
      </div>

      <Handle
        type="source"
        position={Position.Right}
        id="out"
        className={cn(
          '!w-3.5 !h-3.5 !top-1/2',
          '!bg-background !border-2 !border-border',
          '![background:radial-gradient(circle,_theme(colors.amber.500)_35%,_transparent_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150'
        )}
      />
    </div>
  );
});