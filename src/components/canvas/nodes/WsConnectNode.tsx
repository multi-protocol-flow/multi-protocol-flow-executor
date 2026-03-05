/**
 * WsConnect 节点组件 - WebSocket 连接节点
 * 琥珀色主题，表示建立 WebSocket 连接
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Wifi } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { WsConnectConfig } from '@/types';

/** WsConnect 节点数据类型 */
export interface WsConnectNodeData {
  label?: string;
  config?: WsConnectConfig;
}

/**
 * WsConnect 节点 - WebSocket 连接
 * 输入端口 "in"，输出端口 "out"
 */
export function WsConnectNode({ data, selected }: NodeProps) {
  const nodeData = data as WsConnectNodeData;
  
  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-black',
        'bg-white',
        'transition-all duration-200 ease-out',
        'w-[180px]',
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
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-amber-500 before:rounded-full"
        )}
      />
      
      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-amber-500 text-white'
        )}>
          <Wifi className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-amber-600 font-medium uppercase tracking-wide">
            WS
          </span>
          <span className="text-sm font-semibold text-black break-words">
            {nodeData.label || 'WS 连接'}
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
          "before:content-[''] before:absolute before:inset-[3px] before:bg-amber-500 before:rounded-full"
        )}
      />
    </div>
  );
}
