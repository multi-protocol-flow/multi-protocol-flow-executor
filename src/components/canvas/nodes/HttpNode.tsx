/**
 * HTTP 节点组件 - HTTP 请求节点
 * 蓝色主题，表示网络请求
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Globe } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { HttpConfig } from '@/types';

/** HTTP 节点数据类型 */
export interface HttpNodeData {
  label?: string;
  config?: HttpConfig;
}

/**
 * HTTP 节点 - 网络请求
 * 输入端口 "in"，输出端口 "out"
 */
export function HttpNode({ data, selected }: NodeProps) {
  const nodeData = data as HttpNodeData;
  const method = nodeData.config?.method?.toUpperCase() || 'GET';
  
  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-black',
        'bg-white',
        'transition-all duration-200 ease-out',
        'w-[180px]',
        'flex items-center',
        selected && 'border-[3px] border-sky-500'
      )}
    >
      <Handle
        type="target"
        position={Position.Left}
        id="in"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-white !border-2 !border-black',
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
          <span className="text-sm font-semibold text-black break-words">
            {nodeData.label || 'HTTP 请求'}
          </span>
        </div>
      </div>
      
      <Handle
        type="source"
        position={Position.Right}
        id="out"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-white !border-2 !border-black',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-sky-500 before:rounded-full"
        )}
      />
    </div>
  );
}
