/**
 * StageTabBar - 流程生命周期阶段切换栏
 *
 * 居中吸顶悬浮于画布工具栏上方，在 `Setup` / `Main` / `Teardown` 三个子图之间切换画布视图。
 * - 非活跃阶段展示节点计数 Badge
 * - 切换到 Setup/Teardown 时展示阶段说明提示条
 * - 调试进行中展示各阶段状态指示（已完成 / 暂停在断点 / 等待中），
 *   暂停节点不在当前画布时浮现快速跳回条
 */
import { useMemo } from 'react';
import { Check, Hourglass, Pause, LocateFixed, Play, AlertTriangle, Wrench, Workflow, Brush } from 'lucide-react';
import {
  useFlowStore,
  useDebugStore,
  selectActiveStage,
  selectSetupNodeCount,
  selectMainNodeCount,
  selectTeardownNodeCount,
  selectIsDebugging,
  selectIsPaused,
  getNodeStage,
  getStageGraph,
  type FlowStage,
} from '@/stores';
import { cn } from '@/lib/utils';
import { useCurrentLocale } from '@/hooks/useLocale';
import { getLL } from '@/i18n/i18n-lazy';

/** 调试状态下单个阶段的状态指示 */
type StageDebugStatus = 'done' | 'paused' | 'waiting' | 'running' | null;

const STAGE_ORDER: FlowStage[] = ['setup', 'main', 'teardown'];

const STAGE_ICONS: Record<FlowStage, typeof Wrench> = {
  setup: Wrench,
  main: Workflow,
  teardown: Brush,
};
export function StageTabBar() {
  // 订阅语言变更以随 locale 重渲染
  useCurrentLocale();
  const ll = getLL().stages;

  const activeStage = useFlowStore(selectActiveStage);
  const setActiveStage = useFlowStore((s) => s.setActiveStage);
  const setupCount = useFlowStore(selectSetupNodeCount);
  const mainCount = useFlowStore(selectMainNodeCount);
  const teardownCount = useFlowStore(selectTeardownNodeCount);

  const isDebugging = useDebugStore(selectIsDebugging);
  const isPaused = useDebugStore(selectIsPaused);
  const pausedNodeUuid = useDebugStore((s) => s.currentNodeUuid);
  const flow = useFlowStore((s) => s.flow);
  const countByStage: Record<FlowStage, number> = {
    setup: setupCount,
    main: mainCount,
    teardown: teardownCount,
  };
  const labelByStage: Record<FlowStage, string> = {
    setup: ll.tab_setup(),
    main: ll.tab_main(),
    teardown: ll.tab_teardown(),
  };
  const hintByStage: Record<FlowStage, string> = {
    setup: ll.hint_setup(),
    main: ll.hint_main(),
    teardown: ll.hint_teardown(),
  };

  // 断点节点所属阶段：暂停时据此判断是否需要"快速跳回"
  const pausedStage: FlowStage | null =
    isDebugging && pausedNodeUuid && flow ? getNodeStage(flow, pausedNodeUuid) : null;
  const pausedNodeName = pausedStage && flow && pausedNodeUuid
    ? (getStageGraph(flow, pausedStage).nodes.find((n) => n.uuid === pausedNodeUuid)?.name ?? '')
    : '';

  const statusByStage: Record<FlowStage, StageDebugStatus> = {
    setup: null,
    main: null,
    teardown: null,
  };
  if (isDebugging) {
    for (const stage of STAGE_ORDER) {
      statusByStage[stage] = pausedStage === stage ? 'paused' : 'waiting';
    }
    // 已执行完的阶段标记为已完成：执行顺序为 setup → main → teardown
    if (pausedStage === 'main') statusByStage.setup = 'done';
    if (pausedStage === 'teardown') {
      statusByStage.setup = 'done';
      statusByStage.main = 'done';
    }
  }

  const stageStatusTitle = (status: StageDebugStatus, isActive: boolean): string | undefined => {
    if (!isDebugging) return undefined;
    if (status === 'done') return ll.status_done();
    if (status === 'paused') return ll.status_paused();
    if (status === 'waiting') return ll.status_waiting();
    if (isActive) return ll.status_running();
    return undefined;
  };
  const hint = hintByStage[activeStage];
  // Setup/Teardown 多节点未连线检测：如果有多个入度为 0 的节点，提示建议连线
  const showUnlinkedWarning = useMemo(() => {
    if (activeStage === 'main' || !flow) return false;
    const stageGraph = getStageGraph(flow, activeStage);
    if (stageGraph.nodes.length <= 1) return false;
    const inDegreeMap = new Map<string, number>();
    for (const n of stageGraph.nodes) inDegreeMap.set(n.uuid, 0);
    for (const c of stageGraph.connections) {
      inDegreeMap.set(c.target_node_uuid, (inDegreeMap.get(c.target_node_uuid) ?? 0) + 1);
    }
    const rootCount = Array.from(inDegreeMap.values()).filter((deg) => deg === 0).length;
    return rootCount > 1;
  }, [activeStage, flow]);


  // 暂停节点不在当前画布时提供快速跳回
  const showJumpBack = isDebugging && isPaused && pausedStage !== null && pausedStage !== activeStage;

  return (
    <div
      data-testid="stage-tab-bar"
      className="absolute top-4 left-4 z-20 flex flex-col items-start gap-1.5 select-none"
    >
      <div className="flex flex-col gap-1 bg-background/90 backdrop-blur-md border border-border/70 rounded-lg p-1 shadow-sm">
        {STAGE_ORDER.map((stage) => {
          const isActive = stage === activeStage;
          const count = countByStage[stage];
          const status = statusByStage[stage];
          const StageIcon = STAGE_ICONS[stage];
          const StatusIcon =
            status === 'done' ? Check : status === 'paused' ? Pause : status === 'waiting' ? Hourglass : Play;
          const statusColor =
            status === 'done'
              ? 'text-emerald-500'
              : status === 'paused'
                ? 'text-amber-500 animate-pulse'
                : 'text-muted-foreground';

          return (
            <div key={stage} className="relative group">
              <button
                type="button"
                data-testid={`stage-tab-${stage}`}
                aria-label={labelByStage[stage]}
                title={stageStatusTitle(status, isActive)}
                data-active={isActive ? 'true' : 'false'}
                onClick={() => setActiveStage(stage)}
                className={cn(
                  'relative flex items-center justify-center w-8 h-8 rounded-md transition-all duration-150 cursor-pointer',
                  isActive
                    ? 'bg-primary/15 text-primary border border-primary/30 font-semibold shadow-xs'
                    : 'text-muted-foreground hover:text-foreground hover:bg-muted/60 border border-transparent'
                )}
              >
                <StageIcon className="w-4 h-4 shrink-0" />

                {/* 节点计数 Badge */}
                <span
                  data-testid={`stage-tab-count-${stage}`}
                  className={cn(
                    'absolute -top-1 -right-1 min-w-[14px] h-[14px] px-0.5 flex items-center justify-center rounded-full text-[9px] font-mono tabular-nums leading-none transition-colors border border-background shadow-2xs',
                    isActive
                      ? 'bg-primary text-primary-foreground font-bold'
                      : count > 0
                        ? 'bg-muted-foreground text-background font-medium'
                        : 'hidden'
                  )}
                >
                  {count}
                </span>

                {/* 调试状态指示 */}
                {isDebugging && status && (
                  <div className="absolute -bottom-0.5 -left-0.5 rounded-full bg-background p-0.5 shadow-2xs">
                    <StatusIcon className={cn('w-2.5 h-2.5', statusColor)} />
                  </div>
                )}
              </button>

              {/* Hover Tip 浮动卡片 */}
              <div
                className={cn(
                  'absolute left-full top-1/2 -translate-y-1/2 ml-2.5 z-50 pointer-events-none',
                  'w-60 rounded-lg border border-border/80 bg-popover/95 p-2.5 text-xs text-popover-foreground shadow-md backdrop-blur-sm',
                  'opacity-0 -translate-x-1 scale-95 invisible group-hover:opacity-100 group-hover:translate-x-0 group-hover:scale-100 group-hover:visible transition-all duration-150'
                )}
              >
                <div className="flex items-center justify-between gap-1.5 pb-1 border-b border-border/50">
                  <div className="flex items-center gap-1.5 font-semibold text-foreground">
                    <StageIcon className="w-3.5 h-3.5 text-primary" />
                    <span>{labelByStage[stage]}</span>
                  </div>
                  <span className="text-[10px] text-muted-foreground font-mono">
                    {ll.node_count({ count })}
                  </span>
                </div>
                <p className="mt-1.5 text-[11px] text-muted-foreground leading-relaxed">
                  {hintByStage[stage]}
                </p>
                {isDebugging && status && (
                  <div className="mt-2 pt-1.5 border-t border-border/40 flex items-center gap-1.5 text-[11px]">
                    <StatusIcon className={cn('w-3 h-3 shrink-0', statusColor)} />
                    <span className="text-muted-foreground">
                      {stageStatusTitle(status, isActive)}
                    </span>
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* 隐藏的辅助文本供屏幕阅读器和测试获取，免于在屏幕中央常驻遮挡画布 */}
      <span data-testid="stage-hint" className="sr-only">
        {hint}
      </span>
      {showUnlinkedWarning && (
        <div
          data-testid="stage-unlinked-warning"
          title={ll.unlinked_warning_tooltip()}
          className="flex items-center gap-1.5 rounded-md border border-amber-500/30 bg-amber-500/10 px-2 py-0.5 text-[11px] font-medium text-amber-600 dark:text-amber-400 shadow-xs cursor-help animate-in fade-in-0 duration-150"
        >
          <AlertTriangle className="w-3 h-3 shrink-0" />
          <span>{ll.unlinked_warning()}</span>
        </div>
      )}

      {showJumpBack && pausedStage && (
        <button
          type="button"
          data-testid="stage-jump-back"
          onClick={() => {
            setActiveStage(pausedStage);
            if (pausedNodeUuid) useFlowStore.getState().centerOnNode(pausedNodeUuid);
          }}
          className="flex items-center gap-1.5 rounded-md border border-amber-500/40 bg-amber-500/10 px-2.5 py-1 text-[11px] font-medium text-amber-600 shadow-sm cursor-pointer hover:bg-amber-500/20 animate-in fade-in-0 duration-150"
        >
          <Pause className="w-3 h-3 fill-current" />
          <span>
            {getLL().stages.back_to_paused({
              stage: labelByStage[pausedStage],
              node: pausedNodeName || pausedNodeUuid || '',
            })}
          </span>
          <span className="flex items-center gap-0.5 text-foreground">
            {getLL().stages.back_to_paused_action()}
            <LocateFixed className="w-3 h-3" />
          </span>
        </button>
      )}
    </div>
  );
}
