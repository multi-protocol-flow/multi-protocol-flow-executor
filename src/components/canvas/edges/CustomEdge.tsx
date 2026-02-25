/**
 * 自定义连接线组件
 */
import {
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
  MarkerType,
  type EdgeProps,
} from '@xyflow/react';
import { cn } from '@/lib/utils';

/**
 * 自定义连接线组件
 * 提供更美观的连接线样式
 */
export function CustomEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  style = {},
  selected,
  data,
}: EdgeProps) {
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  // 根据源端口决定颜色
  const getEdgeColor = () => {
    const sourceHandle = data?.sourceHandle;
    if (sourceHandle === 'true') return '#22c55e';
    if (sourceHandle === 'false') return '#f43f5e';
    return '#94a3b8';
  };

  const edgeColor = getEdgeColor();

  return (
    <>
      <BaseEdge
        id={id}
        path={edgePath}
        markerEnd={MarkerType.ArrowClosed}
        style={{
          ...style,
          strokeWidth: selected ? 3 : 2,
          stroke: edgeColor,
        }}
        className={cn(
          'transition-all duration-150',
          selected && 'stroke-[3px]'
        )}
      />
      
      {/* 选中时显示标签 */}
      {selected && (
        <EdgeLabelRenderer>
          <div
            className={cn(
              'absolute pointer-events-none',
              'px-2 py-1 rounded-md',
              'bg-white shadow-md border',
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
}
