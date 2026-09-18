/**
 * 自定义连接线组件
 * 使用内置动画表示方向
 * 调试模式下，执行路径的边会变成绿色
 *
 * 性能优化：使用 ref + subscribe 模式替代直接 Zustand hook 订阅，
 * 避免全部边在每次节点状态变化时全量重渲染。
 * 只有当前边的视觉状态（颜色/动画）真正变化时才触发 React 重渲染。
 */
import { memo, useEffect, useRef, useState } from 'react';
import {
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
  type EdgeProps,
} from '@xyflow/react';
import { cn } from '@/lib/utils';
import { useDebugStore } from '@/stores/debug';
import { useExecutionStore } from '@/stores/execution';

/** 默认边颜色（跟随主题） */
const DEFAULT_EDGE_COLOR = 'var(--canvas-edge)';
/** 调试模式下执行路径的边颜色（绿色） */
const EXECUTED_EDGE_COLOR = '#22c55e';

/** 边的视觉状态 */
interface EdgeVisualState {
  /** 是否处于数据流动状态（粒子动画） */
  isActive: boolean;
  /** 是否在调试执行路径上 */
  isExecutedEdge: boolean;
  /** 边颜色 */
  edgeColor: string;
}

/**
 * 从 store 中计算当前边的视觉状态
 * 每次执行/调试事件触发时调用
 */
function computeEdgeVisualState(
  source: string,
  target: string,
  sourceHandle: string | undefined,
): EdgeVisualState {
  const execState = useExecutionStore.getState();
  const debugState = useDebugStore.getState();

  const isExecuting = execState.isExecuting;
  const sourceReport = execState.nodeReportsMap[source];
  const sourceStatus = sourceReport?.status;
  const sourceUsedPortId = sourceReport?.used_port_id;

  const debugStatus = debugState.status;
  // 是否处于调试模式
  const isDebugMode = debugStatus !== 'idle' && debugStatus !== 'ended';

  // 正常执行模式下（非调试）：数据正在通过此边流动
  // 仅在 source 节点执行成功且端口匹配时才激活
  const isPortMatched = sourceUsedPortId === undefined || sourceUsedPortId === sourceHandle;
  const isActiveEdge = !isDebugMode && isExecuting && (
    sourceStatus === 'success' && isPortMatched
  );

  // 调试模式下：边是否在执行路径上（短路求值：非调试模式直接跳过栈查找）
  const isExecutedEdge = (() => {
    if (!isDebugMode) return false;

    const executionStack = debugState.executionStack;
    const debugNodeUuid = debugState.currentNodeUuid;

    // source 必须在执行栈中
    const sourceIndex = executionStack.findIndex((e) => e.node_uuid === source);
    if (sourceIndex === -1) return false;

    // 如果 target 是当前暂停节点，说明边刚被遍历
    if (target === debugNodeUuid) return true;

    // 检查 target 是否是 source 在栈中的下一个节点（直接后继）
    const nextEntry = executionStack[sourceIndex + 1];
    return nextEntry?.node_uuid === target;
  })();

  // debug 模式下全部走绿色路径，不走蓝/橙色活动边
  const edgeColor = isActiveEdge
    ? (sourceHandle === 'true' ? '#f97316' : '#3b82f6')
    : (isExecutedEdge ? EXECUTED_EDGE_COLOR : DEFAULT_EDGE_COLOR);

  return { isActive: isActiveEdge, isExecutedEdge, edgeColor };
}

/**
 * 自定义连接线组件
 * 提供贝塞尔曲线 + 流动动画效果
 */
export const CustomEdge = memo(function CustomEdge({
  id,
  source,
  target,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition: _sourcePosition,
  targetPosition: _targetPosition,
  style = {},
  selected,
  data,
}: EdgeProps) {
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition: _sourcePosition,
    targetPosition: _targetPosition,
  });

  const sourceHandle = data?.sourceHandle as string | undefined;

  // 视觉状态 — 只在真正变化时更新，避免全部边在每步全量重渲染
  const [visualState, setVisualState] = useState<EdgeVisualState>(() =>
    computeEdgeVisualState(source, target, sourceHandle),
  );
  const visualStateRef = useRef(visualState);

  // 订阅 store 变化，只有当前边的状态变化时才触发重渲染
  useEffect(() => {
    const update = () => {
      const newState = computeEdgeVisualState(source, target, sourceHandle);
      const oldState = visualStateRef.current;

      if (
        newState.isActive !== oldState.isActive ||
        newState.isExecutedEdge !== oldState.isExecutedEdge ||
        newState.edgeColor !== oldState.edgeColor
      ) {
        visualStateRef.current = newState;
        setVisualState(newState);
      }
    };

    // 初始化
    update();

    // 同时订阅两个 store，调试模式下 debugStore 状态变化也需要触发重绘
    const unsubExec = useExecutionStore.subscribe(update);
    const unsubDebug = useDebugStore.subscribe(update);
    return () => {
      unsubExec();
      unsubDebug();
    };
  }, [source, target, sourceHandle]);

  const { isActive: isActiveEdge, edgeColor } = visualState;

  return (
    <>
      <BaseEdge
        id={id}
        path={edgePath}
        className={cn(isActiveEdge && 'edge-active-flow')}
        style={{
          ...style,
          strokeWidth: selected ? 3 : 2,
          stroke: edgeColor,
        }}
      />

      {/* 选中时显示标签 */}
      {selected && (
        <EdgeLabelRenderer>
          <div
            className={cn(
              'absolute pointer-events-none',
              'px-2 py-1 rounded-md',
              'bg-background border',
              'text-xs font-medium'
            )}
            style={{
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
            }}
          >
            {sourceHandle === 'true' && '✓ True'}
            {sourceHandle === 'false' && '✗ False'}
            {!sourceHandle && '→'}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
});
