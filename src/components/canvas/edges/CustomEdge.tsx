/**
 * 自定义连接线组件
 * 使用内置动画表示方向
 * 调试模式下，执行路径的边会变成绿色
 */
import { memo, useMemo } from 'react';
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

  // 获取调试状态
  const executionStack = useDebugStore((state) => state.executionStack);
  const debugStatus = useDebugStore((state) => state.status);
  const currentNodeUuid = useDebugStore((state) => state.currentNodeUuid);

  // 获取正常执行状态
  const isExecuting = useExecutionStore((state) => state.isExecuting);
  const execCurrentNodeUuid = useExecutionStore((state) => state.currentNodeUuid);
  const nodeReportsMap = useExecutionStore((state) => state.nodeReportsMap);

  // 判断数据是否正在通过此边流动（正常执行模式）
  const isActiveEdge = isExecuting && (
    execCurrentNodeUuid === source || nodeReportsMap[source]?.status === 'success'
  );

  // 判断边是否在执行路径上
  const isExecutedEdge = useMemo(() => {
    // 非调试模式下，不改变边颜色
    if (debugStatus === 'idle' || debugStatus === 'ended') {
      return false;
    }

    // 获取已执行节点的 UUID 集合
    const executedNodeUuids = new Set(executionStack.map((entry) => entry.node_uuid));

    // 源节点必须已执行
    if (!executedNodeUuids.has(source)) {
      return false;
    }

    // 目标节点满足以下任一条件：
    // 1. 目标节点也已执行完成（在 executionStack 中）
    // 2. 目标节点是当前暂停的节点（正在走这条路）
    return executedNodeUuids.has(target) || target === currentNodeUuid;
  }, [executionStack, debugStatus, source, target, currentNodeUuid]);

  // 连接线颜色：数据流动中 → 协议主题色，调试执行路径 → 绿色，否则默认
  const edgeColor = isActiveEdge
    ? (data?.sourceHandle === 'true' ? '#f97316' : '#3b82f6')
    : (isExecutedEdge ? EXECUTED_EDGE_COLOR : DEFAULT_EDGE_COLOR);

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

      {/* 数据流动画 - 执行中激活 */}
      {isActiveEdge && (
        <>
          {/* 主粒子 */}
          <circle r="3" fill={data?.sourceHandle === 'true' ? '#f97316' : '#3b82f6'}>
            <animateMotion
              dur="1s"
              repeatCount="indefinite"
              path={edgePath}
            />
          </circle>
          {/* 偏移粒子 1 */}
          <circle r="2" fill={data?.sourceHandle === 'true' ? '#fbbf24' : '#60a5fa'}>
            <animateMotion
              dur="1s"
              repeatCount="indefinite"
              path={edgePath}
              begin="0.33s"
            />
          </circle>
          {/* 偏移粒子 2 */}
          <circle r="2.5" fill={data?.sourceHandle === 'true' ? '#fed7aa' : '#93c5fd'}>
            <animateMotion
              dur="1s"
              repeatCount="indefinite"
              path={edgePath}
              begin="0.66s"
            />
          </circle>
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
            {data?.sourceHandle === 'true' && '✓ True'}
            {data?.sourceHandle === 'false' && '✗ False'}
            {!data?.sourceHandle && '→'}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
});