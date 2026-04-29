/**
 * 动态插件节点组件 - 未知节点类型的通用回退渲染
 * 当流程中包含未在 nodeTypes 注册的节点类型时，
 * React Flow 会自动使用 'default' key 指向此组件
 */
import { memo } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Box } from 'lucide-react';
import { cn } from '@/lib/utils';
import { ICON_MAP, getNodeDisplayName } from '@/lib/nodeMeta';
import { useFlowStore, useDebugStore, useExecutionStore, selectCurrentNodeUuid, selectIsPaused, selectIsDebugging } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';

/** 动态节点数据类型 */
interface DynamicNodeData {
  label?: string;
  config?: {
    type?: string;
    [key: string]: unknown;
  };
  [key: string]: unknown;
}

/**
 * 动态插件节点 - 未知节点类型的通用渲染
 * 从 data.config.type 或 React Flow node type 获取类型标识，
 * 通过 nodeMeta 查找图标/颜色，找不到时使用 Box 图标 + 灰色主题
 */
function DynamicPluginNode({ id, type, data, selected }: NodeProps) {
  const nodeData = data as DynamicNodeData;
  const nodeType = nodeData.config?.type || type || 'unknown';
  const iconData = ICON_MAP[nodeType];
  const displayName = nodeData.label || getNodeDisplayName(nodeType);
  const IconComponent = iconData?.icon || Box;
  const iconBg = iconData?.iconBg || 'bg-slate-500';
  const labelColor = iconData?.labelColor || 'text-slate-600';

  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const currentNodeUuid = useDebugStore(selectCurrentNodeUuid);
  const isPaused = useDebugStore(selectIsPaused);
  const isExecuting = useExecutionStore(s => s.isExecuting);
  const isDebugPausedAtThisNode = (isPaused || isExecuting) && currentNodeUuid === id;
  const isDebugging = useDebugStore(selectIsDebugging);

  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-border',
        'bg-background',
        'transition-all duration-200 ease-out',
        'w-[180px]',
        'flex items-center',
        selected && 'border-[3px] border-slate-500',
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
          "before:content-[''] before:absolute before:inset-[3px] before:bg-slate-500 before:rounded-full"
        )}
      />

      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg',
          `${iconBg} text-white`
        )}>
          <IconComponent className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className={cn('text-xs font-medium uppercase tracking-wide', labelColor)}>
            {nodeType}
          </span>
          <span className="text-sm font-semibold text-foreground break-words">
            {displayName}
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
          '![background:radial-gradient(circle,_theme(colors.slate.500)_35%,_white_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150'
        )}
      />
    </div>
  );
}

export default memo(DynamicPluginNode);
