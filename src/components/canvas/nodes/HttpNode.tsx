/**
 * HTTP 节点组件 - HTTP 请求节点
 * 蓝色主题，表示网络请求
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Globe } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { HttpConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectIsNodeRunning, selectIsDebugging, selectIsNodeDebugPaused } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { memo } from 'react';

// T18: @xyflow/react useNodesData 实验性评估
// useNodesData 可用于在 node 组件中直接订阅特定节点的数据变化，
// 替代手动从 store 读取 node.data。示例用法：
//   import { useNodesData } from '@xyflow/react';
//   const { nodeData } = useNodesData(id) ?? {};
//   const method = (nodeData?.config as HttpConfig)?.method?.toUpperCase() || 'GET';
// 优点：减少 store 订阅范围，避免不必要的重渲染
// 当前未启用：HttpNode 中 nodeData 来自 data prop（NodeProps），
// useNodesData 对非受控场景的收益有限，但可作为备选方案保留。

/** HTTP 节点数据类型 */
export interface HttpNodeData {
  label?: string;
  config?: HttpConfig;
}

/**
 * HTTP 节点 - 网络请求
 * 输入端口 "in"，两个输出端口 "true"(上，2xx成功) 和 "false"(下，失败)
 */
export const HttpNode = memo(function HttpNode({ id, data, selected }: NodeProps) {
  const nodeData = data as HttpNodeData;
  const method = nodeData.config?.method?.toUpperCase() || 'GET';
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
        'transition-transform duration-200 ease-out',
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
            {nodeData.label || getNodeDisplayName('http')}
          </span>
        </div>
      </div>
      
      {/* 成功输出端口 - 绿色 */}
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
      
      {/* 失败输出端口 - 红色 */}
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