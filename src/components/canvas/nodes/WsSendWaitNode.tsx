/**
 * WsSendWait 节点组件 - WebSocket 发送等待节点
 * 青色主题，表示发送消息并等待响应
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Send } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { WsSendWaitConfig } from '@/types';

/** WsSendWait 节点数据类型 */
export interface WsSendWaitNodeData {
  label?: string;
  config?: WsSendWaitConfig;
}

/**
 * WsSendWait 节点 - WebSocket 发送等待
 * 输入端口 "in"，输出端口 "out"
 */
export function WsSendWaitNode({ data, selected }: NodeProps) {
  const nodeData = data as WsSendWaitNodeData;
  
  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-black',
        'bg-white',
        'transition-all duration-200 ease-out',
        'w-[180px]',
        'flex items-center',
        selected && 'border-[3px] border-teal-500'
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
          "before:content-[''] before:absolute before:inset-[3px] before:bg-teal-500 before:rounded-full"
        )}
      />
      
      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-teal-500 text-white'
        )}>
          <Send className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-teal-600 font-medium uppercase tracking-wide">
            WS
          </span>
          <span className="text-sm font-semibold text-black break-words">
            {nodeData.label || 'WS 发送等待'}
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
          "before:content-[''] before:absolute before:inset-[3px] before:bg-teal-500 before:rounded-full"
        )}
      />
    </div>
  );
}
