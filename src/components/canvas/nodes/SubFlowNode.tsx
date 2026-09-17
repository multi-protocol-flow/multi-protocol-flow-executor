/**
 * SubFlow 节点组件 - 子流程嵌套节点
 * 紫色主题，表示子流程/流程嵌套调用
 */
import { memo, useMemo } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Workflow, ExternalLink, Pause } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { SubFlowConfig } from '@/types';
import {
  useFlowStore,
  useFlowListStore,
  useDebugStore,
  useExecutionStore,
  selectIsNodeRunning,
  selectSubFlowProgress,
  selectIsDebugging,
  selectIsNodeDebugPaused,
  selectIsPaused,
} from '@/stores';
import { flowStorage } from '@/services/storage';
import { useCurrentLocale } from '@/hooks/useLocale';
import { BreakpointMarker } from '../BreakpointMarker';
import { getNodeDisplayName } from '@/lib/nodeMeta';
import { getLL } from '@/i18n/i18n-lazy';
import { toast } from 'sonner';

/** SubFlow 节点数据类型 */
export interface SubFlowNodeData {
  label?: string;
  config?: SubFlowConfig;
}

/**
 * SubFlow 节点 - 嵌套子流程调用
 * 输入端口 "in"，两个输出端口 "out"(成功) 和 "false"(失败)
 */
export const SubFlowNode = memo(function SubFlowNode({ id, data, selected }: NodeProps) {
  // 订阅 locale：节点被 memo 包裹，locale 切换时需主动触发重渲染以刷新文案
  useCurrentLocale();
  const ll = getLL();
  const nodeData = data as SubFlowNodeData;
  const config = nodeData.config;

  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const setFlow = useFlowStore((state) => state.setFlow);
  const flows = useFlowListStore((state) => state.flows);

  const isRunning = useExecutionStore(selectIsNodeRunning(id));
  const subFlowProgress = useExecutionStore(selectSubFlowProgress(id));
  const isDebugging = useDebugStore(selectIsDebugging);
  const isDebugPaused = useDebugStore(selectIsNodeDebugPaused(id));
  const isPaused = useDebugStore(selectIsPaused);
  const callFrames = useDebugStore((state) => state.callFrames);
  const isDebugPausedAtThisNode = isRunning || isDebugPaused;

  const childFrame = useMemo(() => {
    return callFrames.find((f) => f.parent_subflow_node_uuid === id);
  }, [callFrames, id]);
  const isPausedInsideChild = isPaused && Boolean(childFrame);
  // 目标引用二选一：文件路径优先（与后端解析顺序一致），其次为已保存流程名称
  const targetDisplay = useMemo(() => {
    if (config?.target_flow_path) {
      const match = flows.find(
        (flow) => flow.file_path === config.target_flow_path || flow.uuid === config.target_flow_path
      );
      if (match) return match.name;
      const fileName = config.target_flow_path.split(/[/\\]/).pop()?.replace(/\.mpf$/, '');
      return fileName || config.target_flow_path;
    }
    if (config?.target_flow_id) {
      const match = flows.find(
        (flow) => flow.uuid === config.target_flow_id || flow.file_path === config.target_flow_id
      );
      return match ? match.name : config.target_flow_id;
    }
    return null;
  }, [config?.target_flow_id, config?.target_flow_path, flows]);

  /** 无论配置了 path 还是 id，只要有目标均可在标签页打开 */
  const canOpenTarget = Boolean(config?.target_flow_path || config?.target_flow_id);

  const handleOpenFlow = async (e: React.MouseEvent) => {
    e.stopPropagation();
    const target = config?.target_flow_path || config?.target_flow_id;
    if (!target) {
      toast.error(ll.panels.subflow.select_target_flow());
      return;
    }
    try {
      const flow = await flowStorage.loadFlow(target);
      if (flow) {
        setFlow(flow);
        toast.success(flow.name);
      } else {
        toast.error(ll.panels.subflow.missing_target());
      }
    } catch (err) {
      toast.error(String(err));
    }
  };

  return (
    <div
      className={cn(
        'relative px-3.5 py-2.5 rounded-xl',
        'border-2 border-violet-500/60 shadow-sm',
        'ring-2 ring-violet-500/20 ring-offset-1 ring-offset-background',
        'bg-background',
        'w-[200px]',
        'flex flex-col gap-1.5',
        selected && 'border-[3px] border-violet-500 ring-violet-500/40',
        isDebugPausedAtThisNode && 'debug-breathing-violet',
        isPausedInsideChild && 'ring-4 ring-cyan-400/60 border-violet-500 animate-pulse shadow-lg shadow-cyan-500/20'
      )}
    >
      {isDebugging && (
        <BreakpointMarker
          hasBreakpoint={hasBreakpoint}
          onToggle={() => toggleBreakpoint(id)}
        />
      )}

      {/* 输入端口 */}
      <Handle
        type="target"
        position={Position.Left}
        id="in"
        className={cn(
          '!w-3.5 !h-3.5 !top-1/2',
          '!bg-background !border-2 !border-border',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-violet-500 before:rounded-full"
        )}
      />

      <div className="flex items-center gap-2.5 w-full min-w-0">
        <div
          className={cn(
            'shrink-0 p-1.5 rounded-lg shadow-sm',
            'bg-violet-500 text-white'
          )}
        >
          <Workflow className="w-4 h-4" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-[10px] text-violet-600 font-semibold uppercase tracking-wider">
            {getNodeDisplayName('flow:sub')}
          </span>
          <span className="text-sm font-semibold text-foreground truncate" title={nodeData.label}>
            {nodeData.label || getNodeDisplayName('flow:sub')}
          </span>
        </div>
      </div>

      {/* 目标流程展示与快捷打开 */}
      <div className="flex items-center justify-between gap-1 text-xs text-muted-foreground bg-muted/50 rounded px-2 py-1 border border-border/60">
        <span className="truncate flex-1 font-mono text-[11px]" title={targetDisplay ?? undefined}>
          {targetDisplay ?? (
            <span className="italic text-muted-foreground/60">{ll.panels.notSet()}</span>
          )}
        </span>
        {canOpenTarget && (
          <button
            type="button"
            onClick={handleOpenFlow}
            title={ll.panels.subflow.open_target_flow()}
            className="shrink-0 p-0.5 text-muted-foreground hover:text-violet-600 hover:bg-violet-50 dark:hover:bg-violet-950/30 rounded transition-colors"
          >
            <ExternalLink className="w-3 h-3" />
          </button>
        )}
      </div>

      {/* 子流程实时进度：子流程在顶层流程中算一个节点，内部进度挂在本节点上。
          绝对定位在节点下方，避免执行期间撑高节点导致端口位置跳动。 */}
      {isRunning && (
        <div className="absolute left-0 right-0 top-full mt-1.5 z-40 flex flex-col gap-1 rounded border border-violet-500/40 bg-background/95 px-2 py-1 shadow-sm backdrop-blur-sm">
          <span className="text-[10px] font-medium text-violet-700 dark:text-violet-300 tabular-nums truncate">
            {subFlowProgress && subFlowProgress.total > 0
              ? ll.panels.subflow.running_progress({
                  completed: subFlowProgress.completed,
                  total: subFlowProgress.total,
                })
              : ll.panels.subflow.running_starting()}
          </span>
          {subFlowProgress && subFlowProgress.total > 0 && (
            <div className="h-1 w-full rounded-full bg-violet-500/20 overflow-hidden">
              <div
                className="h-full bg-violet-500 transition-all duration-200"
                style={{
                  width: `${Math.min(
                    100,
                    Math.round((subFlowProgress.completed / subFlowProgress.total) * 100)
                  )}%`,
                }}
              />
            </div>
          )}
          {subFlowProgress?.currentNodeName && (
            <span
              className="text-[10px] text-muted-foreground truncate"
              title={subFlowProgress.currentNodeName}
            >
              {ll.panels.subflow.running_node({ name: subFlowProgress.currentNodeName })}
            </span>
          )}
        </div>
      )}
      {isPausedInsideChild && childFrame && (
        <button
          onClick={(e) => {
            e.stopPropagation();
            const frameIndex = callFrames.findIndex((f) => f.parent_subflow_node_uuid === id);
            if (frameIndex >= 0) {
              useDebugStore.getState().setActiveFrame(frameIndex);
            }
          }}
          className="absolute left-0 right-0 top-full mt-1.5 z-40 flex items-center justify-center gap-1.5 rounded-lg border border-cyan-500/50 bg-background/95 px-2.5 py-1.5 shadow-md backdrop-blur-sm text-[11px] text-cyan-600 dark:text-cyan-400 hover:bg-cyan-50 dark:hover:bg-cyan-950/50 transition-colors cursor-pointer"
          title={ll.flow.debug_subflow_click_to_enter()}
        >
          <Pause className="w-3 h-3 fill-current text-amber-500 shrink-0" />
          <span className="truncate">
            {ll.flow.debug_subflow_paused_inside({ name: childFrame.node_name || childFrame.node_uuid })}
          </span>
        </button>
      )}


      {/* 成功端口: out */}
      <Handle
        type="source"
        position={Position.Right}
        id="out"
        className={cn(
          '!w-3.5 !h-3.5 !top-[30%]',
          '!bg-background !border-2 !border-emerald-500 dark:!border-emerald-400',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-emerald-600 dark:before:bg-emerald-500 before:rounded-full"
        )}
      />

      {/* 失败端口: false */}
      <Handle
        type="source"
        position={Position.Right}
        id="false"
        className={cn(
          '!w-3.5 !h-3.5 !top-[75%]',
          '!bg-background !border-2 !border-rose-500 dark:!border-rose-400',
          'hover:!scale-125',
          'transition-transform duration-150',
          "before:content-[''] before:absolute before:inset-[3px] before:bg-rose-600 dark:before:bg-rose-500 before:rounded-full"
        )}
      />
    </div>
  );
});
