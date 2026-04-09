/**
 * Script 节点组件 - 脚本执行节点
 * 紫色主题，表示自定义脚本执行
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Code } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ScriptNodeConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectCurrentNodeUuid, selectIsPaused } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';

/** Script 节点数据类型 */
export interface ScriptNodeData {
  label?: string;
  config?: ScriptNodeConfig;
}

/**
 * Script 节点 - 脚本执行
 * 输入端口 "input"，两个输出端口 "true"(上) 和 "false"(下)
 */
export function ScriptNode({ id, data, selected }: NodeProps) {
  const nodeData = data as ScriptNodeData;
  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const currentNodeUuid = useDebugStore(selectCurrentNodeUuid);
  const isPaused = useDebugStore(selectIsPaused);
  const isDebugPausedAtThisNode = isPaused && currentNodeUuid === id;
  const getNodeExecutionStatus = useExecutionStore((state) => state.getNodeExecutionStatus);
  const executionStatus = getNodeExecutionStatus(id);
  const executionClassName = executionStatus !== 'pending' ? `execution-${executionStatus}` : '';
  
  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-black',
        'bg-white',
        'transition-all duration-200 ease-out',
        'w-[180px]',
        'flex items-center',
        selected && 'border-[3px] border-violet-500',
        isDebugPausedAtThisNode && 'debug-breathing-violet',
        executionClassName
      )}
    >
      <BreakpointMarker
        hasBreakpoint={hasBreakpoint}
        onToggle={() => toggleBreakpoint(id)}
      />
      <Handle
        type="target"
        position={Position.Left}
        id="input"
        className={cn(
          '!w-3.5 !h-3.5 !top-1/2',
          '!bg-white !border-2 !border-black',
          '![background:radial-gradient(circle,_theme(colors.violet.500)_35%,_white_35%)]',
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
            脚本
          </span>
          <span className="text-sm font-semibold text-black break-words">
            {nodeData.label || '脚本执行'}
          </span>
        </div>
      </div>
      
      <Handle
        type="source"
        position={Position.Right}
        id="true"
        className={cn(
          '!w-3.5 !h-3.5 !top-[30%]',
          '!bg-white !border-2 !border-black',
          '![background:radial-gradient(circle,_theme(colors.violet.500)_35%,_white_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150'
        )}
      />
      
      <Handle
        type="source"
        position={Position.Right}
        id="false"
        className={cn(
          '!w-3.5 !h-3.5 !top-[70%]',
          '!bg-white !border-2 !border-black',
          '![background:radial-gradient(circle,_theme(colors.violet.500)_35%,_white_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150'
        )}
      />
      
      <div className="absolute right-0 top-0 bottom-0 flex flex-col justify-center translate-x-5 pointer-events-none">
        <span className="text-[10px] font-bold text-emerald-600 leading-none -mt-2">✓</span>
        <span className="text-[10px] font-bold text-rose-600 leading-none mt-4">✗</span>
      </div>
    </div>
  );
}
