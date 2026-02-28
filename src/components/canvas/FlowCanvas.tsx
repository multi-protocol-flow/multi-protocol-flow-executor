/**
 * FlowCanvas - 主画布组件
 * 基于 React Flow 的流程编辑器
 */
import { useCallback, useEffect, useMemo } from 'react';
import {
  ReactFlow,
  ReactFlowProvider,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  addEdge,
  useReactFlow,
  type Connection,
  type Node,
  type NodeChange,
  type EdgeChange,
  BackgroundVariant,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { EntryNode, HttpNode, ConditionalNode } from './nodes';
import { CustomEdge } from './edges/CustomEdge';
import { useFlowStore } from '@/stores';
import {
  convertFlowToReactFlow,
  convertEdgeToConnection,
  isValidConnection,
} from './utils';
import { cn } from '@/lib/utils';

/**
 * 自定义十字准星光标 SVG (data URI)
 * - 深色描边 (#1e293b) 在浅色背景可见
 * - 白色填充在深色背景可见
 * - hotspot 在中心 (12, 12)
 */
const CUSTOM_CURSOR_SVG = `data:image/svg+xml,${encodeURIComponent(`
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
    <circle cx="12" cy="12" r="5" fill="white" stroke="#1e293b" stroke-width="1.5"/>
    <circle cx="12" cy="12" r="1.5" fill="#1e293b"/>
    <line x1="12" y1="3" x2="12" y2="7" stroke="#1e293b" stroke-width="1.5" stroke-linecap="round"/>
    <line x1="12" y1="17" x2="12" y2="21" stroke="#1e293b" stroke-width="1.5" stroke-linecap="round"/>
    <line x1="3" y1="12" x2="7" y2="12" stroke="#1e293b" stroke-width="1.5" stroke-linecap="round"/>
    <line x1="17" y1="12" x2="21" y2="12" stroke="#1e293b" stroke-width="1.5" stroke-linecap="round"/>
  </svg>
`)}`;

/** 画布自定义光标样式 */
const CANVAS_CURSOR_STYLE: React.CSSProperties = {
  cursor: `url("${CUSTOM_CURSOR_SVG}") 12 12, crosshair`,
};

/** 节点类型映射 */
const nodeTypes = {
  entry: EntryNode,
  http: HttpNode,
  conditional: ConditionalNode,
};

/** 边类型映射 */
const edgeTypes = {
  custom: CustomEdge,
};

/** MiniMap 节点颜色 */
const minimapNodeColor = (node: Node) => {
  switch (node.type) {
    case 'entry':
      return '#22c55e';
    case 'http':
      return '#3b82f6';
    case 'conditional':
      return '#f97316';
    default:
      return '#94a3b8';
  }
};

/**
 * 画布内部组件（在 ReactFlowProvider 内部）
 */
function FlowCanvasInner() {
  const { flow, addConnection, updateNode, selectNode, addNode, removeNode, removeConnection } = useFlowStore();
  const { screenToFlowPosition, setCenter, getViewport } = useReactFlow();

  // 转换 Flow 数据为 React Flow 格式
  const initialData = useMemo(() => {
    if (!flow) return { nodes: [], edges: [] };
    return convertFlowToReactFlow(flow);
  }, [flow]);

  // 节点和边状态
  const [nodes, setNodes, onNodesChange] = useNodesState(initialData.nodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialData.edges);

  // 当 flow 变化时，更新节点和边
  // 使用 node.uuid 和 conn.id 作为稳定标识，避免属性变化时重置选择状态
  useEffect(() => {
    if (flow) {
      const { nodes: newNodes, edges: newEdges } = convertFlowToReactFlow(flow);
      
      // 获取当前节点/边的 ID 集合
      const currentNodeIds = new Set(nodes.map(n => n.id));
      const newNodeIds = new Set(newNodes.map(n => n.id));
      const currentEdgeIds = new Set(edges.map(e => e.id));
      const newEdgeIds = new Set(newEdges.map(e => e.id));
      
      // 检查是否有节点/边的增减（结构变化）
      const hasStructureChange = 
        currentNodeIds.size !== newNodeIds.size ||
        currentEdgeIds.size !== newEdgeIds.size ||
        ![...currentNodeIds].every(id => newNodeIds.has(id)) ||
        ![...currentEdgeIds].every(id => newEdgeIds.has(id));
      
      if (hasStructureChange) {
        // 结构变化时完全重置
        setNodes(newNodes);
        setEdges(newEdges);
      } else {
        // 仅属性变化时，只更新节点数据，保留选择状态
        setNodes(prevNodes => 
          prevNodes.map(prevNode => {
            const newNode = newNodes.find(n => n.id === prevNode.id);
            if (newNode) {
              return {
                ...prevNode,
                position: newNode.position,
                data: newNode.data,
              };
            }
            return prevNode;
          })
        );
        setEdges(newEdges);
      }
    }
  // 注意：这里故意不依赖 nodes/edges，避免循环更新
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [flow, setNodes, setEdges]);

  // 处理连接创建
  const onConnect = useCallback(
    (connection: Connection) => {
      // 验证连接
      if (
        !isValidConnection(
          connection.source || '',
          connection.target || '',
          connection.sourceHandle || 'out',
          connection.targetHandle || 'in'
        )
      ) {
        return;
      }

      // 添加边到 React Flow
      setEdges((eds) =>
        addEdge(
          {
            ...connection,
            type: 'custom',
            data: { sourceHandle: connection.sourceHandle },
          },
          eds
        )
      );

      // 同步到 Zustand store
      if (connection.source && connection.target) {
        const newConnection = convertEdgeToConnection({
          id: `edge-${Date.now()}`,
          source: connection.source,
          target: connection.target,
          sourceHandle: connection.sourceHandle || 'out',
          targetHandle: connection.targetHandle || 'in',
        });
        addConnection(newConnection);
      }
    },
    [setEdges, addConnection]
  );

  // 处理节点拖拽结束
  const onNodeDragStop = useCallback(
    (_: unknown, node: Node) => {
      updateNode(node.id, {
        position: { x: node.position.x, y: node.position.y },
      });
    },
    [updateNode]
  );

  // 处理节点选择
  const onSelectionChange = useCallback(
    ({ nodes: selectedNodes }: { nodes: Node[] }) => {
      if (selectedNodes.length === 1) {
        selectNode(selectedNodes[0].id);
      } else if (selectedNodes.length === 0) {
        selectNode(null);
      }
    },
    [selectNode]
  );

  // 处理节点变化（处理位置变化和删除）
  const handleNodesChange = useCallback(
    (changes: NodeChange[]) => {
      // 分离删除变化和其他变化
      const removeChanges = changes.filter((c) => c.type === 'remove');
      const otherChanges = changes.filter((c) => c.type !== 'remove');

      // 处理其他变化（如位置变化、选择变化等）
      if (otherChanges.length > 0) {
        onNodesChange(otherChanges);
      }

      // 处理删除变化 - 入口节点不可删除
      if (removeChanges.length > 0 && flow) {
        for (const change of removeChanges) {
          const node = flow.nodes.find((n) => n.uuid === change.id);
          if (node && node.type !== 'entry') {
            removeNode(change.id);
          }
        }
      }
    },
    [onNodesChange, flow, removeNode]
  );

  // 处理边变化
  const handleEdgesChange = useCallback(
    (changes: EdgeChange[]) => {
      // 分离删除变化和其他变化
      const removeChanges = changes.filter((c) => c.type === 'remove');
      const otherChanges = changes.filter((c) => c.type !== 'remove');

      // 处理其他变化
      if (otherChanges.length > 0) {
        onEdgesChange(otherChanges);
      }

      // 处理删除变化 - 同步到 store
      if (removeChanges.length > 0) {
        for (const change of removeChanges) {
          removeConnection(change.id);
        }
      }
    },
    [onEdgesChange, removeConnection]
  );

  // 处理拖拽经过
  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  // 处理拖放
  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();

      const nodeType = event.dataTransfer.getData('application/reactflow/nodeType');
      if (!nodeType || !flow) return;

      // 获取当前视口信息
      const { zoom } = getViewport();
      
      // 计算当前可视画布中心在流程坐标系中的位置
      // 获取画布容器的尺寸
      const canvasElement = event.currentTarget as HTMLElement;
      const rect = canvasElement.getBoundingClientRect();
      const centerX = rect.width / 2;
      const centerY = rect.height / 2;
      
      // 将屏幕中心转换为流程坐标
      const position = screenToFlowPosition({
        x: rect.left + centerX,
        y: rect.top + centerY,
      });

      // 根据节点类型创建节点配置
      const nodeCount = flow.nodes.length + 1;
      const newNodeUuid = crypto.randomUUID();

      if (nodeType === 'http') {
        addNode({
          uuid: newNodeUuid,
          type: 'http',
          name: `HTTP 请求 ${nodeCount}`,
          position,
          data: { type: 'http', url: '', method: 'get', headers: {}, body: '' },
        });
      } else if (nodeType === 'conditional') {
        addNode({
          uuid: newNodeUuid,
          type: 'conditional',
          name: `条件判断 ${nodeCount}`,
          position,
          data: {
            type: 'conditional',
            condition_type: 'simple',
            field_path: '',
            operator: 'equal',
            expected_value: '',
          },
        });
      }

      // 将视角移动到新节点位置（带动画效果）
      setTimeout(() => {
        setCenter(position.x, position.y, { zoom, duration: 300 });
      }, 50);
    },
    [screenToFlowPosition, flow, addNode, getViewport, setCenter]
  );

  // 没有流程时显示空状态
  if (!flow) {
    return (
      <div className="w-full h-full flex items-center justify-center bg-slate-50">
        <div className="text-center">
          <div className="text-6xl mb-4">📋</div>
          <h2 className="text-xl font-semibold text-slate-600">
            暂无流程
          </h2>
          <p className="text-sm text-slate-400 mt-2">
            请创建或打开一个流程文件
          </p>
        </div>
      </div>
    );
  }

  return (
    <div 
      className={cn('w-full h-full', 'bg-slate-50')}
      style={CANVAS_CURSOR_STYLE}
    >
      {/* 全局样式：节点上显示 pointer */}
      <style>{`
        /* 节点和交互元素显示 pointer */
        .react-flow__node,
        .react-flow__node *,
        .react-flow__handle,
        .react-flow__edge-path,
        .react-flow__controls,
        .react-flow__controls *,
        .react-flow__minimap,
        .react-flow__minimap * {
          cursor: pointer !important;
        }
        /* 拖拽时显示 grabbing */
        .react-flow__node.dragging,
        .react-flow__node.dragging * {
          cursor: grabbing !important;
        }
      `}</style>
      
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={handleNodesChange}
        onEdgesChange={handleEdgesChange}
        onConnect={onConnect}
        onNodeDragStop={onNodeDragStop}
        onSelectionChange={onSelectionChange}
        onDragOver={onDragOver}
        onDrop={onDrop}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        minZoom={0.1}
        maxZoom={2}
        defaultEdgeOptions={{
          type: 'custom',
          animated: true,
        }}
      >
        {/* 背景网格 */}
        <Background
          variant={BackgroundVariant.Dots}
          gap={20}
          size={1}
          color="#cbd5e1"
        />

        {/* 控制按钮 */}
        <Controls
          className="!bg-white !border-slate-200"
          showZoom
          showFitView
          showInteractive
        />

        {/* 小地图 */}
        <MiniMap
          nodeColor={minimapNodeColor}
          maskColor="rgba(0, 0, 0, 0.1)"
          className="!bg-white !border-slate-200"
          pannable
          zoomable
        />
      </ReactFlow>
    </div>
  );
}

/**
 * 流程画布组件（带 ReactFlowProvider）
 */
export function FlowCanvas() {
  return (
    <ReactFlowProvider>
      <FlowCanvasInner />
    </ReactFlowProvider>
  );
}
