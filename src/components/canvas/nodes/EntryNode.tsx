/**
 * Entry 节点组件 - 流程入口点
 * 绿色主题，表示流程的开始
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Play } from 'lucide-react';
import { cn } from '@/lib/utils';

/** Entry 节点数据类型 */
export interface EntryNodeData {
  label?: string;
  config?: unknown;
}

/**
 * Entry 节点 - 流程入口
 * 无输入端口，单个输出端口 "out"
 */
export function EntryNode({ data, selected }: NodeProps) {
  const nodeData = data as EntryNodeData;
  
  return (
    <div
      className={cn(
        // 基础样式
        'relative px-4 py-3 rounded-xl',
        'border-2 border-emerald-500/60',
        'bg-gradient-to-br from-emerald-50 to-emerald-100/80',
        'shadow-lg shadow-emerald-500/10',
        'transition-all duration-200 ease-out',
        'min-w-[140px]',
        // 选中状态
        selected && [
          'ring-2 ring-emerald-500 ring-offset-2 ring-offset-background',
          'shadow-xl shadow-emerald-500/20',
          'scale-[1.02]',
        ]
      )}
    >
      {/* 左侧装饰条 */}
      <div className="absolute left-0 top-2 bottom-2 w-1 rounded-full bg-emerald-500" />
      
      {/* 图标和标签 */}
      <div className="flex items-center gap-2.5">
        <div className={cn(
          'p-1.5 rounded-lg',
          'bg-emerald-500 text-white',
          'shadow-md shadow-emerald-500/30'
        )}>
          <Play className="w-3.5 h-3.5 fill-current" />
        </div>
        <div className="flex flex-col">
          <span className="text-xs text-emerald-600 font-medium uppercase tracking-wide">
            入口
          </span>
          <span className="text-sm font-semibold text-emerald-900">
            {nodeData.label || '流程入口'}
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
          '!bg-emerald-500',
          '!border-2 !border-white',
          '!shadow-md !shadow-emerald-500/30',
          'hover:!scale-125 hover:!bg-emerald-400',
          'transition-transform duration-150'
        )}
      />
    </div>
  );
}
