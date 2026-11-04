/**
 * Delay 节点组件 - 延时节点
 * 石板灰主题，表示可中断等待
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Timer } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { DelayConfig } from '@/types';
import {
  useFlowStore,
  useDebugStore,
  useExecutionStore,
  selectIsNodeRunning,
  selectIsDebugging,
  selectIsNodeDebugPaused,
} from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { memo } from 'react';
import { useCurrentLocale } from '@/hooks/useLocale';
import { NodeTitle } from './NodeTitle';

/** Delay 节点数据类型 */
export interface DelayNodeData {
  label?: string;
  config?: DelayConfig;
}

/**
 * Delay 节点 - 延时
 * 单输入端口 "in"，单输出端口 "out"
 */
export const DelayNode = memo(function DelayNode({ id, data, selected }: NodeProps) {
  useCurrentLocale();
  const nodeData = data as DelayNodeData;
  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const isRunning = useExecutionStore(selectIsNodeRunning(id));
  const isDebugging = useDebugStore(selectIsDebugging);
  const isDebugPaused = useDebugStore(selectIsNodeDebugPaused(id));
  const isDebugPausedAtThisNode = isRunning || isDebugPaused;

  const config = nodeData.config;

  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-border',
        'bg-background',
        'w-[180px]',
        'flex items-center',
        selected && 'border-slate-500 ring-2 ring-slate-500/20',
        isDebugPausedAtThisNode && 'debug-breathing-amber',
      )}
    >
      {isDebugging && (
        <BreakpointMarker hasBreakpoint={hasBreakpoint} onToggle={() => toggleBreakpoint(id)} />
      )}
      <Handle
        type="target"
        position={Position.Left}
        id="in"
        className={cn(
          '!w-3.5 !h-3.5 !top-1/2',
          '!bg-background !border-2 !border-border',
          '![background:radial-gradient(circle,_theme(colors.slate.500)_35%,_transparent_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150',
        )}
      />

      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn('shrink-0 p-1.5 rounded-lg', 'bg-slate-500 text-white')}>
          <Timer className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-slate-600 font-medium uppercase tracking-wide">
            {getNodeDisplayName('flow:delay')}
          </span>
          <NodeTitle
            nodeId={id}
            label={nodeData.label}
            defaultLabel={getNodeDisplayName('flow:delay')}
          />
          <span className="text-[10px] text-muted-foreground truncate">
            {config?.duration_ms ?? '1000'} ms
            {config && config.jitter_ms > 0 ? ` (+${config.jitter_ms})` : ''}
          </span>
        </div>
      </div>

      <Handle
        type="source"
        position={Position.Right}
        id="out"
        className={cn(
          '!w-3.5 !h-3.5 !top-1/2',
          '!bg-background !border-2 !border-slate-500 dark:!border-slate-400',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-slate-600 dark:before:bg-slate-500 before:rounded-full",
        )}
      />
    </div>
  );
});
