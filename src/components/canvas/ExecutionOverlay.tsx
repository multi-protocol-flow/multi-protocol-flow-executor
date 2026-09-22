/**
 * ExecutionOverlay - 执行状态叠加层
 *
 * 在 React Flow 画布上方独立渲染执行状态指示器，
 * 不依赖 setNodes 修改节点 className，从而避免执行期间触发 React Flow 全量 reconcile。
 *
 * 性能原理：
 * - 使用 Zustand subscribe 而非 hook，避免组件因 store 变化而重渲染
 * - 仅在状态实际变化时调用 setState（比对 badge 列表）
 * - 叠加层容器通过 onMove 直接写入 DOM style.transform，不触发 React 重渲染
 */
import { memo, useEffect, useState, useRef } from 'react';
import { useReactFlow } from '@xyflow/react';
import { useExecutionStore, useFlowStore, selectActiveStage } from '@/stores';
import { cn } from '@/lib/utils';

// ============================================================================
// 类型
// ============================================================================

interface StatusBadge {
  uuid: string;
  status: 'success' | 'failed';
  /** 流程坐标系 X */
  x: number;
  /** 流程坐标系 Y */
  y: number;
}

// ============================================================================
// 常量
// ============================================================================

/** 节点宽度（与 CSS w-[180px] 一致） */
const NODE_WIDTH = 180;

/** 徽章偏移：节点右上角 */
const BADGE_OFFSET_X = NODE_WIDTH - 8;
const BADGE_OFFSET_Y = -8;

// ============================================================================
// 组件
// ============================================================================

export const ExecutionOverlay = memo(function ExecutionOverlay() {
  const [badges, setBadges] = useState<StatusBadge[]>([]);
  const badgesRef = useRef<StatusBadge[]>([]);
  const { getNode } = useReactFlow();
  const activeStage = useFlowStore(selectActiveStage);

  useEffect(() => {
    const update = () => {
      const { nodeReportsMap, isExecuting } = useExecutionStore.getState();

      // 跳过：没有执行中的节点也没有已完成的节点
      if (!isExecuting && Object.keys(nodeReportsMap).length === 0) {
        if (badgesRef.current.length > 0) {
          badgesRef.current = [];
          setBadges([]);
        }
        return;
      }

      const newBadges: StatusBadge[] = [];

      for (const [uuid, report] of Object.entries(nodeReportsMap)) {
        if (report.status === 'success' || report.status === 'failed') {
          const node = getNode(uuid);
          if (node) {
            newBadges.push({
              uuid,
              status: report.status,
              // 徽章定位在节点右上角，使用流程坐标系
              x: node.position.x + BADGE_OFFSET_X,
              y: node.position.y + BADGE_OFFSET_Y,
            });
          }
        }
      }

      // 浅比较新旧列表，只有真正变化时才触发 React 重渲染
      const old = badgesRef.current;
      if (old.length !== newBadges.length) {
        badgesRef.current = newBadges;
        setBadges(newBadges);
        return;
      }
      for (let i = 0; i < newBadges.length; i++) {
        if (
          newBadges[i].uuid !== old[i]?.uuid ||
          newBadges[i].status !== old[i]?.status
        ) {
          badgesRef.current = newBadges;
          setBadges(newBadges);
          return;
        }
      }
    };

    // 初始检查
    update();

    const unsubExec = useExecutionStore.subscribe(update);
    // 画布阶段切换或流程变更时刷新徽章
    const unsubFlow = useFlowStore.subscribe((state, prevState) => {
      if (state.activeStage !== prevState.activeStage || state.flow !== prevState.flow) {
        requestAnimationFrame(update);
      }
    });
    return () => {
      unsubExec();
      unsubFlow();
    };
  }, [getNode, activeStage]);

  if (badges.length === 0) return null;

  return (
    <>
      {badges.map((badge) => (
        <div
          key={badge.uuid}
          className={cn(
            'absolute z-50 w-5 h-5 rounded-full flex items-center justify-center',
            'text-white text-[11px] font-bold leading-none',
            'shadow-sm border border-white/30',
            'select-none pointer-events-none',
            'animate-in fade-in zoom-in-50 duration-100',
            badge.status === 'success' && 'bg-emerald-500',
            badge.status === 'failed' && 'bg-red-500',
          )}
          style={{
            left: badge.x,
            top: badge.y,
            transform: 'translate(-50%, -50%)',
          }}
        >
          {badge.status === 'success' ? '✓' : '✗'}
        </div>
      ))}
    </>
  );
});
