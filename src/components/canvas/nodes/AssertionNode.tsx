/**
 * Assertion 节点组件 - 断言节点
 * 紫色主题，表示数据验证/断言
 */
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { CheckCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { AssertionNodeConfig } from '@/types';
import { useFlowStore, useDebugStore, useExecutionStore, selectIsNodeRunning, selectIsDebugging, selectIsNodeDebugPaused } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { getLL } from '@/i18n/i18n-lazy';
import { memo } from 'react';
import { useCurrentLocale } from '@/hooks/useLocale';

/** Assertion 节点数据类型 */
export interface AssertionNodeData {
  label?: string;
  config?: AssertionNodeConfig;
}

/**
 * Assertion 节点 - 断言验证
 * 输入端口 "in"，两个输出端口 "true"(上，断言通过) 和 "false"(下，断言失败)
 */
export const AssertionNode = memo(function AssertionNode({ id, data, selected }: NodeProps) {
  useCurrentLocale();
  const nodeData = data as AssertionNodeData;
  const config = nodeData.config;
  const assertionCount = config?.assertions?.length ?? 0;
  const mode = config?.mode ?? 'all';
  const onFailure = config?.on_failure ?? 'route_false';
  const LL = getLL();
  
  const onError = useFlowStore(
    (state) => state.flow?.nodes.find((n) => n.uuid === id)?.on_error,
  );
  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const isRunning = useExecutionStore(selectIsNodeRunning(id));
  const isDebugging = useDebugStore(selectIsDebugging);
  const isDebugPaused = useDebugStore(selectIsNodeDebugPaused(id));
  const isDebugPausedAtThisNode = isRunning || isDebugPaused;
  
  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-border',
        'bg-background',
        'w-[180px]',
        'flex items-center',
        selected && 'border-[3px] border-purple-500',
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
          '!w-3.5 !h-3.5 !top-1/2',
          '!bg-background !border-2 !border-border',
          '![background:radial-gradient(circle,_theme(colors.purple.500)_35%,_transparent_35%)]',
          'hover:!scale-125',
          'transition-transform duration-150'
        )}
      />
      
      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div className={cn(
          'shrink-0 p-1.5 rounded-lg relative',
          'bg-purple-500 text-white'
        )}>
          <CheckCircle className="w-3.5 h-3.5" />
          {assertionCount > 0 && (
            <span className="absolute -top-1.5 -right-1.5 min-w-[14px] h-[14px] px-0.5 text-[9px] font-bold bg-purple-700 text-white rounded-full flex items-center justify-center">
              {assertionCount}
            </span>
          )}
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-xs text-purple-600 font-medium uppercase tracking-wide">
            {getNodeDisplayName('assertion')}
          </span>
          <span className="text-sm font-semibold text-foreground break-words">
            {nodeData.label || getNodeDisplayName('assertion')}
          </span>
          {/* 显示模式信息 */}
          <div className="flex items-center gap-1 mt-0.5 flex-wrap">
            <span className={cn(
              'text-[10px] px-1 py-0.5 rounded',
              mode === 'all' ? 'bg-purple-100 text-purple-700' : 'bg-purple-100 text-purple-700'
            )}>
              {mode === 'all' ? LL.nodes.assertion_mode.all_pass() : LL.nodes.assertion_mode.any_pass()}
            </span>
            <span className={cn(
              'text-[10px] px-1 py-0.5 rounded',
              onFailure === 'route_false' ? 'bg-gray-100 text-gray-600' : 'bg-red-100 text-red-600'
            )}>
              {onFailure === 'route_false' ? LL.nodes.assertion_on_failure.route() : LL.nodes.assertion_on_failure.throw()}
            </span>
            {/* 执行错误处理策略指示器 */}
            {onError && (
              <span className={cn(
                'text-[10px] px-1 py-0.5 rounded',
                onError === 'route_to_false'
                  ? 'bg-amber-100 text-amber-700'
                  : 'bg-rose-100 text-rose-600'
              )}>
                {onError === 'route_to_false'
                  ? LL.nodes.assertion_on_error.route_to_false()
                  : LL.nodes.assertion_on_error.abort_flow()}
              </span>
            )}
          </div>
        </div>
      </div>
      
      <Handle
        type="source"
        position={Position.Right}
        id="true"
        className={cn(
          '!w-3.5 !h-3.5 !top-[30%]',
          '!bg-background !border-2 !border-emerald-500 dark:!border-emerald-400',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-emerald-600 dark:before:bg-emerald-500 before:rounded-full"
        )}
      />
      
      <Handle
        type="source"
        position={Position.Right}
        id="false"
        className={cn(
          '!w-3.5 !h-3.5 !top-[70%]',
          '!bg-background !border-2 !border-rose-500 dark:!border-rose-400',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-rose-600 dark:before:bg-rose-500 before:rounded-full"
        )}
      />
    </div>
  );
});