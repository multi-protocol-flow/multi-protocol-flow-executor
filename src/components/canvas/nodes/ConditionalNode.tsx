/**
 * Conditional 节点组件 - 条件判断节点
 * 橙色主题，表示条件分支
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { GitBranch } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ConditionalConfig } from '@/types';

/** Conditional 节点数据类型 */
export interface ConditionalNodeData {
  label?: string;
  config?: ConditionalConfig;
}

/**
 * Conditional 节点 - 条件分支
 * 输入端口 "in"，两个输出端口 "true"(上) 和 "false"(下)
 */
export function ConditionalNode({ data, selected }: NodeProps) {
  const nodeData = data as ConditionalNodeData;
  
  return (
    <div
      className={cn(
        // 基础样式
        'relative px-4 py-3 rounded-xl',
        'border-2 border-orange-500/60',
        'bg-gradient-to-br from-orange-50 to-orange-100/80',
        'shadow-lg shadow-orange-500/10',
        'transition-all duration-200 ease-out',
        'min-w-[160px]',
        // 选中状态
        selected && [
          'ring-2 ring-orange-500 ring-offset-2 ring-offset-background',
          'shadow-xl shadow-orange-500/20',
          'scale-[1.02]',
        ]
      )}
    >
      {/* 左侧装饰条 */}
      <div className="absolute left-0 top-2 bottom-2 w-1 rounded-full bg-orange-500" />
      
      {/* 输入端口 - 左侧居中 */}
      <Handle
        type="target"
        position={Position.Left}
        id="in"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-orange-500',
          '!border-2 !border-white',
          '!shadow-md !shadow-orange-500/30',
          'hover:!scale-125 hover:!bg-orange-400',
          'transition-transform duration-150'
        )}
        style={{ top: '50%' }}
      />
      
      {/* 图标和标签 */}
      <div className="flex items-center gap-2.5">
        <div className={cn(
          'p-1.5 rounded-lg',
          'bg-orange-500 text-white',
          'shadow-md shadow-orange-500/30'
        )}>
          <GitBranch className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col">
          <span className="text-xs text-orange-600 font-medium uppercase tracking-wide">
            条件
          </span>
          <span className="text-sm font-semibold text-orange-900">
            {nodeData.label || '条件判断'}
          </span>
        </div>
      </div>
      
      {/* 输出端口 - 右侧，true 在上，false 在下 */}
      {/* True 端口 */}
      <Handle
        type="source"
        position={Position.Right}
        id="true"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-emerald-500',
          '!border-2 !border-white',
          '!shadow-md !shadow-emerald-500/30',
          'hover:!scale-125 hover:!bg-emerald-400',
          'transition-transform duration-150'
        )}
        style={{ top: '30%' }}
      />
      
      {/* False 端口 */}
      <Handle
        type="source"
        position={Position.Right}
        id="false"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-rose-500',
          '!border-2 !border-white',
          '!shadow-md !shadow-rose-500/30',
          'hover:!scale-125 hover:!bg-rose-400',
          'transition-transform duration-150'
        )}
        style={{ top: '70%' }}
      />
      
      {/* 端口标签 */}
      <div className="absolute right-0 top-0 bottom-0 flex flex-col justify-center translate-x-5 pointer-events-none">
        <span className="text-[10px] font-bold text-emerald-600 leading-none" style={{ marginTop: '-8px' }}>T</span>
        <span className="text-[10px] font-bold text-rose-600 leading-none" style={{ marginTop: '16px' }}>F</span>
      </div>
    </div>
  );
}
