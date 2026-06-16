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
  const execNodeUuid = execState.currentNodeUuid;
  const sourceReport = execState.nodeReportsMap[source];
  const sourceStatus = sourceReport?.status;
  const sourceUsedPortId = sourceReport?.used_port_id;

  const debugStatus = debugState.status;
  const executionStack = debugState.executionStack;
  const debugNodeUuid = debugState.currentNodeUuid;

  // 正常执行模式下：数据是否正在通过此边流动
  // 对于已完成的节点，仅当边的端口与实际使用的端口匹配时才高亮
  // used_port_id 未设置时（legacy 报告），向后兼容显示所有出边
  const isPortMatched = sourceUsedPortId === undefined || sourceUsedPortId === sourceHandle;
  const isActiveEdge = isExecuting && (
    execNodeUuid === source ||
    (sourceStatus === 'success' && isPortMatched)
  );

  // 调试模式下：边是否在执行路径上
  const isExecutedEdge = (() => {
    if (debugStatus === 'idle' || debugStatus === 'ended') {
      return false;
    }
    const executedNodeUuids = new Set(executionStack.map((entry) => entry.node_uuid));
    if (!executedNodeUuids.has(source)) {
      return false;
    }
    return executedNodeUuids.has(target) || target === debugNodeUuid;
  })();

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

    // 单订阅即可，computeEdgeVisualState 内部通过 getState() 读取两个 store
    const unsub = useExecutionStore.subscribe(update);
    return () => {
      unsub();
    };
  }, [source, target, sourceHandle]);

  const { isActive: isActiveEdge, edgeColor } = visualState;

  return (
    <>
      <BaseEdge
        id={id}
        path={edgePath}
        style={{
          ...style,
          strokeWidth: selected ? 3 : 2,
          stroke: edgeColor,
        }}
      />

      {/* 数据流动画 - 执行中激活
          使用 CSS offset-path 动画替代 SVG animateMotion（运行在合成器线程，
          不占用主线程，不会影响画布拖拽和节点交互） */}
      {isActiveEdge && (
        <>
          <circle
            r="3"
            fill={sourceHandle === 'true' ? '#f97316' : '#3b82f6'}
            className="edge-particle"
            style={{ offsetPath: `path("${edgePath}")`, offsetDistance: '0%' } as React.CSSProperties}
          />
          <circle
            r="2"
            fill={sourceHandle === 'true' ? '#fbbf24' : '#60a5fa'}
            className="edge-particle"
            style={{ offsetPath: `path("${edgePath}")`, offsetDistance: '0%', animationDelay: '-0.33s' } as React.CSSProperties}
          />
          <circle
            r="2.5"
            fill={sourceHandle === 'true' ? '#fed7aa' : '#93c5fd'}
            className="edge-particle"
            style={{ offsetPath: `path("${edgePath}")`, offsetDistance: '0%', animationDelay: '-0.66s' } as React.CSSProperties}
          />
        </>
      )}

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
