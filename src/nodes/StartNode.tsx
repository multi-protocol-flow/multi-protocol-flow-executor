import React, { memo } from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import { Card } from '@/components/ui/card';
import { cn } from '@/lib/utils';

/**
 * StartNode 组件的 data 接口
 */
export interface StartNodeData {
  label?: string;
  description?: string;
  nodeType?: string;
  error?: boolean;
}

/**
 * 开始节点组件
 *
 * 功能：
 * - 使用 shadcn/ui Card 组件作为基础容器
 * - 绿色背景 (#22c55e，Tailwind emerald-500)
 * - 圆形/圆角矩形形状 (rounded-full)
 * - 输出端口位于右侧（Handle, Position.Right）
 * - 选中状态显示蓝色边框 (#3b82f6，Tailwind blue-500)
 * - 错误状态显示红色边框 (#ef4444，Tailwind red-500)
 * - 使用 React.memo() 优化性能
 *
 * 设计决策：
 * - 不添加输入端口（开始节点无输入）
 * - 仅实现基础渲染（不添加复杂交互）
 * - 使用 rounded-full 实现圆形形状
 * - 使用 Tailwind 的 ring 类实现边框效果
 */
export const StartNode = memo(({ data, selected }: NodeProps<StartNodeData>) => {
  const isError = data?.error === true;

  return (
    <Card
      className={cn(
        // 基础样式：绿色背景、圆形、内边距
        'bg-emerald-500 rounded-full px-6 py-3',
        // 边框和阴影
        'border-2 border-emerald-600 shadow-lg',
        // 选中状态：蓝色边框
        selected && 'ring-2 ring-blue-500 ring-offset-2 ring-offset-slate-900',
        // 错误状态：红色边框
        isError && 'ring-2 ring-red-500 ring-offset-2 ring-offset-slate-900',
        // 文字样式
        'text-white',
        // 过渡动画
        'transition-all duration-200',
        // 确保内容居中
        'flex items-center justify-center',
        // 最小宽度确保文字显示
        'min-w-[120px]'
      )}
      data-testid="start-node-card"
    >
      {/* 节点标签 */}
      <div className="font-medium text-sm text-center" data-testid="node-label">
        {data?.label || '开始'}
      </div>

      {/* 输出端口 - 位于右侧 */}
      <Handle
        type="source"
        position={Position.Right}
        className={cn(
          'w-3 h-3',
          'bg-emerald-300',
          'border-2 border-emerald-600',
          'transition-colors duration-200'
        )}
      />
    </Card>
  );
});

StartNode.displayName = 'StartNode';
