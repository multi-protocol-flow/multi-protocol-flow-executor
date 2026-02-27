/**
 * 自定义连接线组件
 */
import {
  BaseEdge,
  EdgeLabelRenderer,
  getStraightPath,
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
  sourcePosition: _sourcePosition,
  targetPosition: _targetPosition,
  style = {},
  selected,
  data,
}: EdgeProps) {
  const [edgePath, labelX, labelY] = getStraightPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
  });

  // 连接线统一使用纯黑色
  const edgeColor = '#000000';

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
              'bg-white border',
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
