/**
 * HTTP 节点组件 - HTTP 请求节点
 * 蓝色主题，表示网络请求
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Globe } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { HttpConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectCurrentNodeUuid, selectIsPaused, selectIsDebugging } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';

/** HTTP 节点数据类型 */
export interface HttpNodeData {
  label?: string;
  config?: HttpConfig;
}

/**
 * HTTP 节点 - 网络请求
 * 输入端口 "in"，两个输出端口 "true"(上，2xx成功) 和 "false"(下，失败)
 */
export function HttpNode({ id, data, selected }: NodeProps) {
  const nodeData = data as HttpNodeData;
  const method = nodeData.config?.method?.toUpperCase() || 'GET';
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
        'border-2 border-border',
        'bg-background',
        'transition-all duration-200 ease-out',
        'w-[180px]',
        'flex items-center',
        selected && 'border-[3px] border-sky-500',
        isDebugPausedAtThisNode && 'debug-breathing-sky'
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
          '!w-3.5 !h-3.5',
          '!bg-background !border-2 !border-border',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-sky-500 before:rounded-full"
        )}
      />
      
      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-blue-500 text-white'
        )}>
          <Globe className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-blue-600 font-medium uppercase tracking-wide">
            {method}
          </span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || 'HTTP 请求'}
          </span>
        </div>
      </div>
      
      {/* 成功输出端口 */}
      <Handle
        type="source"
        position={Position.Right}
        id="true"
        className={cn(
          '!w-3.5 !h-3.5 !top-[30%]',
          '!bg-background !border-2 !border-border',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-sky-500 before:rounded-full"
        )}
      />
      
      {/* 失败输出端口 */}
      <Handle
        type="source"
        position={Position.Right}
        id="false"
        className={cn(
          '!w-3.5 !h-3.5 !top-[70%]',
          '!bg-background !border-2 !border-border',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-sky-500 before:rounded-full"
        )}
      />
      
      {/* 端口标签 */}
      <div className="absolute right-0 top-0 bottom-0 flex flex-col justify-center translate-x-5 pointer-events-none">
        <span className="text-[10px] font-bold text-emerald-600 leading-none -mt-2">✓</span>
        <span className="text-[10px] font-bold text-rose-600 leading-none mt-4">✗</span>
      </div>
    </div>
  );
}
