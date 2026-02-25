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
        // 基础样式
        'relative px-4 py-3 rounded-xl',
        'border-2 border-blue-500/60',
        'bg-gradient-to-br from-blue-50 to-blue-100/80',
        'shadow-lg shadow-blue-500/10',
        'transition-all duration-200 ease-out',
        'min-w-[160px]',
        // 选中状态
        selected && [
          'ring-2 ring-blue-500 ring-offset-2 ring-offset-background',
          'shadow-xl shadow-blue-500/20',
          'scale-[1.02]',
        ]
      )}
    >
      {/* 左侧装饰条 */}
      <div className="absolute left-0 top-2 bottom-2 w-1 rounded-full bg-blue-500" />
      
      {/* 输入端口 - 左侧 */}
      <Handle
        type="target"
        position={Position.Left}
        id="in"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-blue-500',
          '!border-2 !border-white',
          '!shadow-md !shadow-blue-500/30',
          'hover:!scale-125 hover:!bg-blue-400',
          'transition-transform duration-150'
        )}
      />
      
      {/* 图标和标签 */}
      <div className="flex items-center gap-2.5">
        <div className={cn(
          'p-1.5 rounded-lg',
          'bg-blue-500 text-white',
          'shadow-md shadow-blue-500/30'
        )}>
          <Globe className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col">
          <span className="text-xs text-blue-600 font-medium uppercase tracking-wide">
            {method}
          </span>
          <span className="text-sm font-semibold text-blue-900">
            {nodeData.label || 'HTTP 请求'}
          </span>
        </div>
      </div>
      
      {/* 输出端口 - 右侧 */}
      <Handle
        type="source"
        position={Position.Right}
        id="out"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-blue-500',
          '!border-2 !border-white',
          '!shadow-md !shadow-blue-500/30',
          'hover:!scale-125 hover:!bg-blue-400',
          'transition-transform duration-150'
        )}
      />
    </div>
  );
}
