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
        'relative px-4 h-16 rounded-xl',
        'border-2 border-black',
        'bg-white',
        'transition-all duration-200 ease-out',
        'min-w-[160px]',
        'flex items-center',
        selected && 'border-[3px] border-amber-500'
      )}
    >
      <Handle
        type="target"
        position={Position.Left}
        id="in"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-white !border-2 !border-black',
          '![background:radial-gradient(circle,_theme(colors.amber.500)_35%,_white_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150'
        )}
        style={{ top: '50%' }}
      />
      
      <div className="flex items-center gap-2.5">
        <div className={cn(
          'p-1.5 rounded-lg',
          'bg-orange-500 text-white'
        )}>
          <GitBranch className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col">
          <span className="text-xs text-orange-600 font-medium uppercase tracking-wide">
            条件
          </span>
          <span className="text-sm font-semibold text-black">
            {nodeData.label || '条件判断'}
          </span>
        </div>
      </div>
      
      <Handle
        type="source"
        position={Position.Right}
        id="true"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-white !border-2 !border-black',
          '![background:radial-gradient(circle,_theme(colors.amber.500)_35%,_white_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150'
        )}
        style={{ top: '30%' }}
      />
      
      <Handle
        type="source"
        position={Position.Right}
        id="false"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-white !border-2 !border-black',
          '![background:radial-gradient(circle,_theme(colors.amber.500)_35%,_white_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150'
        )}
        style={{ top: '70%' }}
      />
      
      <div className="absolute right-0 top-0 bottom-0 flex flex-col justify-center translate-x-5 pointer-events-none">
        <span className="text-[10px] font-bold text-emerald-600 leading-none" style={{ marginTop: '-8px' }}>T</span>
        <span className="text-[10px] font-bold text-rose-600 leading-none" style={{ marginTop: '16px' }}>F</span>
      </div>
    </div>
  );
}
