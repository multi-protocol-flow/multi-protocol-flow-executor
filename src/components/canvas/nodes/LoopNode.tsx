/**
 * Loop 节点组件 - 循环节点
 * 天蓝主题，表示内联子图迭代；右上角按钮进入循环体画布
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { RotateCw } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { LoopConfig } from '@/types';
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
import { getLL } from '@/i18n/i18n-lazy';
import { memo } from 'react';
import { useCurrentLocale } from '@/hooks/useLocale';
import { NodeTitle } from './NodeTitle';

/** Loop 节点数据类型 */
export interface LoopNodeData {
  label?: string;
  config?: LoopConfig;
}

/**
 * Loop 节点 - 循环
 * 输入端口 "in"，输出端口 "done"（成功终止，上）与 "false"（异常终止，下）
 */
export const LoopNode = memo(function LoopNode({ id, data, selected }: NodeProps) {
  useCurrentLocale();
  const nodeData = data as LoopNodeData;
  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const isRunning = useExecutionStore(selectIsNodeRunning(id));
  const isDebugging = useDebugStore(selectIsDebugging);
  const isDebugPaused = useDebugStore(selectIsNodeDebugPaused(id));
  const isDebugPausedAtThisNode = isRunning || isDebugPaused;

  const config = nodeData.config;
  const LL = getLL();
  const summary = (() => {
    if (!config) return '';
    switch (config.source) {
      case 'items':
        return LL.panels.loop.source_items();
      case 'until':
        return LL.panels.loop.source_until();
      default:
        return `${LL.panels.loop.source_count()} ${config.count ?? 0}`;
    }
  })();
  const bodyNodeCount = config?.body?.nodes?.length ?? 0;

  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-border',
        'bg-background',
        'w-[180px]',
        'flex items-center',
        selected && 'border-sky-500 ring-2 ring-sky-500/20',
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
          '![background:radial-gradient(circle,_theme(colors.sky.500)_35%,_transparent_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150',
        )}
      />

      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn('shrink-0 p-1.5 rounded-lg', 'bg-sky-500 text-white')}>
          <RotateCw className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-sky-600 font-medium uppercase tracking-wide">
            {getNodeDisplayName('flow:loop')}
          </span>
          <NodeTitle
            nodeId={id}
            label={nodeData.label}
            defaultLabel={getNodeDisplayName('flow:loop')}
          />
          <span className="text-[10px] text-muted-foreground truncate">
            {summary}
            {bodyNodeCount > 0
              ? ` · ${LL.panels.loop.body_node_count({ count: bodyNodeCount })}`
              : ''}
          </span>
        </div>
      </div>

      <Handle
        type="source"
        position={Position.Right}
        id="done"
        className={cn(
          '!w-3.5 !h-3.5 !top-[30%]',
          '!bg-background !border-2 !border-emerald-500 dark:!border-emerald-400',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-emerald-600 dark:before:bg-emerald-500 before:rounded-full",
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
          "before:content-[''] before:absolute before:inset-[3px] before:bg-rose-600 dark:before:bg-rose-500 before:rounded-full",
        )}
      />
    </div>
  );
});
