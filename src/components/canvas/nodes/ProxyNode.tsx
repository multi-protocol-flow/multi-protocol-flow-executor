/**
 * Proxy 节点组件 - 代理设置节点
 * 青柠色主题，表示代理配置
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Shield } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ProxyConfig } from '@/types/node';
import { useFlowStore, useDebugStore, useExecutionStore, selectIsNodeRunning, selectIsDebugging, selectIsNodeDebugPaused } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { getLL } from '@/i18n/i18n-lazy';
import { memo } from 'react';

/** Proxy 节点数据类型 */
export interface ProxyNodeData {
  label?: string;
  config?: ProxyConfig;
}

/** Proxy 节点数据类型 */
export interface ProxyNodeData {
  label?: string;
  config?: ProxyConfig;
}

/**
 * Proxy 节点 - 代理设置
 * 单输入端口 "in"，单输出端口 "out"
 */
export const ProxyNode = memo(function ProxyNode({ id, data, selected }: NodeProps) {
  const nodeData = data as ProxyNodeData;
  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const isRunning = useExecutionStore(selectIsNodeRunning(id));
  const isDebugging = useDebugStore(selectIsDebugging);
  const isDebugPaused = useDebugStore(selectIsNodeDebugPaused(id));
  const isDebugPausedAtThisNode = isRunning || isDebugPaused;

  const LL = getLL();
  const config = nodeData.config;
  const modeLabel = config?.mode === 'clear'
    ? LL.nodes.proxy_mode.clear()
    : config?.mode?.toUpperCase() ?? LL.nodes.proxy_mode.clear();

  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-border',
        'bg-background',
        'w-[180px]',
        'flex items-center',
        selected && 'border-[3px] border-lime-500',
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
          '![background:radial-gradient(circle,_theme(colors.lime.500)_35%,_transparent_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150'
        )}
      />

      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-lime-500 text-white'
        )}>
          <Shield className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-lime-600 font-medium uppercase tracking-wide">
            {getNodeDisplayName('proxy')}
          </span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || getNodeDisplayName('proxy')}
          </span>
          <div className="flex items-center gap-1.5 mt-0.5">
            <span className={cn(
              'inline-flex items-center px-1.5 py-0 rounded text-[10px] font-semibold',
              config?.mode && config.mode !== 'clear'
                ? 'bg-lime-100 text-lime-700'
                : 'bg-slate-100 text-slate-500'
            )}>
              {modeLabel}
            </span>
            {config?.proxy_url && config.mode !== 'clear' && (
              <span className="text-[10px] text-muted-foreground truncate">
                {config.proxy_url}
              </span>
            )}
          </div>
        </div>
      </div>

      <Handle
        type="source"
        position={Position.Right}
        id="out"
        className={cn(
          '!w-3.5 !h-3.5 !top-1/2',
          '!bg-background !border-2 !border-border',
          '![background:radial-gradient(circle,_theme(colors.lime.500)_35%,_transparent_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150'
        )}
      />
    </div>
  );
});