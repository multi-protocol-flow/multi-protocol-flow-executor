import React, { useCallback, useMemo } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useReactFlow,
  addEdge,
  applyNodeChanges,
  applyEdgeChanges,
  BackgroundVariant,
} from '@xyflow/react';
import type {
  Node,
  Edge,
  Connection,
  NodeChange,
  EdgeChange,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { cn } from '../lib/utils';

export interface FlowCanvasProps {
  nodes: Node[];
  edges: Edge[];
  onNodesChange?: (changes: NodeChange[]) => void;
  onEdgesChange?: (changes: EdgeChange[]) => void;
  onConnect?: (connection: Connection) => void;
  onNodesDelete?: (nodes: Node[]) => void;
  className?: string;
}

/**
 * 基础节点组件
 */
function BaseNode({ data, selected }: { data: any; selected: boolean }) {
  const isStartNode = data?.nodeType === 'start';
  const nodeColor = isStartNode
    ? 'bg-emerald-500 border-emerald-600'
    : 'bg-slate-800 border-slate-700';

  return (
    <div
      className={cn(
        'px-4 py-3 rounded-lg border-2 shadow-lg transition-all duration-200',
        nodeColor,
        selected && 'ring-2 ring-blue-500 ring-offset-2 ring-offset-slate-900'
      )}
    >
      <div className="text-white font-medium text-sm">
        {data?.label || '节点'}
      </div>
      {data?.description && (
        <div className="text-slate-300 text-xs mt-1">
          {data.description}
        </div>
      )}
    </div>
  );
}

/**
 * React Flow 画布组件
 *
 * 功能：
 * - 渲染 React Flow 画布
 * - 支持节点拖拽
 * - 支持连接创建
 * - 阻止删除开始节点
 */
export function FlowCanvas({
  nodes: initialNodes,
  edges: initialEdges,
  onNodesChange,
  onEdgesChange,
  onConnect,
  onNodesDelete,
  className,
}: FlowCanvasProps) {
  const [nodes, setNodes] = React.useState<Node[]>(initialNodes);
  const [edges, setEdges] = React.useState<Edge[]>(initialEdges);

  // 同步外部传入的 nodes/edges
  React.useEffect(() => {
    setNodes(initialNodes);
  }, [initialNodes]);

  React.useEffect(() => {
    setEdges(initialEdges);
  }, [initialEdges]);

  /**
   * 处理节点变化（拖拽等）
   */
  const handleNodesChange = useCallback(
    (changes: NodeChange[]) => {
      const updatedNodes = applyNodeChanges(changes, nodes);
      setNodes(updatedNodes);
      onNodesChange?.(changes);
    },
    [nodes, onNodesChange]
  );

  /**
   * 处理边变化（连接删除等）
   */
  const handleEdgesChange = useCallback(
    (changes: EdgeChange[]) => {
      const updatedEdges = applyEdgeChanges(changes, edges);
      setEdges(updatedEdges);
      onEdgesChange?.(changes);
    },
    [edges, onEdgesChange]
  );

  /**
   * 处理新连接创建
   */
  const handleConnect = useCallback(
    (connection: Connection) => {
      setEdges((eds) => addEdge(connection, eds));
      onConnect?.(connection);
    },
    [onConnect]
  );

  /**
   * 处理节点删除
   * 阻止删除开始节点
   */
  const handleNodesDelete = useCallback(
    (nodesToDelete: Node[]) => {
      // 检查是否包含开始节点
      const hasStartNode = nodesToDelete.some(
        (node) => node.data?.nodeType === 'start'
      );

      if (hasStartNode) {
        console.error('不能删除开始节点');
        return;
      }

      setNodes((nds) => nds.filter((n) => !nodesToDelete.includes(n)));
      onNodesDelete?.(nodesToDelete);
    },
    [onNodesDelete]
  );

  /**
   * 节点类型配置
   */
  const nodeTypes = useMemo(
    () => ({
      start: BaseNode,
      http: BaseNode,
      conditional: BaseNode,
      transform: BaseNode,
      end: BaseNode,
    }),
    []
  );

  return (
    <div className={cn('w-full h-full bg-slate-950', className)}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={handleNodesChange}
        onEdgesChange={handleEdgesChange}
        onConnect={handleConnect}
        onNodesDelete={handleNodesDelete}
        nodeTypes={nodeTypes}
        fitView
        attributionPosition="bottom-left"
        defaultEdgeOptions={{
          type: 'smoothstep',
          animated: true,
          style: { stroke: '#64748b', strokeWidth: 2 },
        }}
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={16}
          size={1}
          color="#334155"
        />
        <Controls
          className="bg-slate-800 border-slate-700 text-slate-200"
          showZoom
          showFitView
          showInteractive
        />
        <MiniMap
          className="bg-slate-800 border-slate-700"
          nodeColor={(node) => {
            return node.data?.nodeType === 'start'
              ? '#10b981'
              : '#1e293b';
          }}
          maskColor="rgba(15, 23, 42, 0.6)"
        />
      </ReactFlow>
    </div>
  );
}

/**
 * Hook 用于获取 React Flow 实例
 */
export function useFlowCanvas() {
  const reactFlowInstance = useReactFlow();

  return {
    fitView: () => reactFlowInstance.fitView(),
    zoomIn: () => reactFlowInstance.zoomIn(),
    zoomOut: () => reactFlowInstance.zoomOut(),
    setCenter: (x: number, y: number) => reactFlowInstance.setCenter(x, y),
    getNodes: () => reactFlowInstance.getNodes(),
    getEdges: () => reactFlowInstance.getEdges(),
  };
}
