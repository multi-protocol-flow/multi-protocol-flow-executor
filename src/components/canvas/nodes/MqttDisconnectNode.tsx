/**
 * MqttDisconnect 节点组件 - MQTT 断开节点
 * 青色主题，表示断开 MQTT 连接
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Unplug } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { MqttDisconnectConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectRunningNodeUuid, selectIsExecutingFlag, selectIsDebugging, selectDebugCurrentNodeUuid } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { memo } from 'react';

/** MqttDisconnect 节点数据类型 */
export interface MqttDisconnectNodeData {
  label?: string;
  config?: MqttDisconnectConfig;
}

/**
 * MqttDisconnect 节点 - MQTT 断开
 * 输入端口 "in"，单输出端口 "out"
 */
export const MqttDisconnectNode = memo(function MqttDisconnectNode({ id, data, selected }: NodeProps) {
  const nodeData = data as MqttDisconnectNodeData;
  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const runningNodeUuid = useExecutionStore(selectRunningNodeUuid);
  const isExecuting = useExecutionStore(selectIsExecutingFlag);
  const isRunning = isExecuting && runningNodeUuid === id;
  const debugPausedNodeUuid = useDebugStore(selectDebugCurrentNodeUuid);
  const isDebuggingStatus = useDebugStore(selectIsDebugging);
  const isDebugPaused = isDebuggingStatus && debugPausedNodeUuid === id;
  const isDebugPausedAtThisNode = isRunning || isDebugPaused;
  const isDebugging = useDebugStore(selectIsDebugging);
  
  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-border',
        'bg-background',
        'transition-transform duration-200 ease-out',
        'w-[180px]',
        'flex items-center',
        selected && 'border-[3px] border-cyan-500',
        isDebugPausedAtThisNode && 'debug-breathing-purple'
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
          "before:content-[''] before:absolute before:inset-[3px] before:bg-cyan-500 before:rounded-full"
        )}
      />
      
      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          'bg-cyan-500 text-white'
        )}>
          <Unplug className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-cyan-600 font-medium tracking-wide">
            {getNodeDisplayName('mqtt_disconnect')}
          </span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || getNodeDisplayName('mqtt_disconnect')}
          </span>
        </div>
      </div>
      
      <Handle
        type="source"
        position={Position.Right}
        id="out"
        className={cn(
          '!w-3.5 !h-3.5',
          '!bg-background !border-2 !border-border',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-cyan-500 before:rounded-full"
        )}
      />
    </div>
  );
});