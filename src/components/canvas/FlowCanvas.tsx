/**
 * FlowCanvas - 主画布组件
 * 基于 React Flow 的流程编辑器
 */
import { useCallback, useEffect, useMemo, useRef } from 'react';
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
import { CanvasToolbar } from './CanvasToolbar';
import { CustomEdge } from './edges/CustomEdge';
import { useFlowStore, useUIStore } from '@/stores';
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

/**
 * 自定义抓手光标 SVG (data URI)
 * - 深色描边确保在浅色背景可见
 * - 浅色填充确保在深色背景可见
 * - hotspot 在左上角 (8, 8)
 */
const GRABBING_CURSOR_SVG = `data:image/svg+xml,${encodeURIComponent(`
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
    <path d="M8 8V16M8 8L5 11M8 8L11 11" stroke="#1e293b" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="M16 16V8M16 16L19 13M16 16L13 13" stroke="#1e293b" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    <circle cx="12" cy="12" r="3" fill="white" stroke="#1e293b" stroke-width="1.5"/>
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
  const { 
    flow, 
    addConnection, 
    updateNode, 
    selectNode, 
    addNode, 
    removeConnection,
    pendingAddNodeType,
    clearPendingAddNodeType,
  } = useFlowStore();
  const { setConfigPanelOpen, setConfigPanelMode } = useUIStore();

  const { screenToFlowPosition, fitView } = useReactFlow();
  
  // 画布容器引用（用于计算视口中心）
  const canvasRef = useRef<HTMLDivElement>(null);
  
  // 跟踪已触发 fitView 的流程，避免重复触发
  const fittedFlowUuidRef = useRef<string | null>(null);
  // 转换 Flow 数据为 React Flow 格式
  const initialData = useMemo(() => {
    if (!flow) return { nodes: [], edges: [] };
    return convertFlowToReactFlow(flow);
  }, [flow]);

  // 节点和边状态
  const [nodes, setNodes, onNodesChange] = useNodesState(initialData.nodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialData.edges);
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

  // 流程首次加载或切换时触发 fitView
  useEffect(() => {
    if (flow && flow.uuid !== fittedFlowUuidRef.current) {
      fittedFlowUuidRef.current = flow.uuid;
      // 使用 setTimeout 确保 ReactFlow 已完成节点渲染
      setTimeout(() => {
        fitView({ padding: 0.2, duration: 300 });
      }, 100);
    }
  }, [flow?.uuid, fitView]);

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

  // 处理节点点击 - 选中节点并展开面板
  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      selectNode(node.id);
      setConfigPanelOpen(true);
      setConfigPanelMode('node');
    },
    [selectNode, setConfigPanelOpen, setConfigPanelMode]
  );

  // 处理画布空白处点击 - 取消选中并收起面板
  const onPaneClick = useCallback(() => {
    selectNode(null);
    setConfigPanelOpen(false);
  }, [selectNode, setConfigPanelOpen]);

  // 处理节点变化（处理位置变化等）
  const handleNodesChange = useCallback(
    (changes: NodeChange[]) => {
      onNodesChange(changes);
    },
    [onNodesChange]
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

  // 处理拖放 - 使用鼠标实际位置
  // 处理拖放 - 使用鼠标实际位置
  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();

      const nodeType = event.dataTransfer.getData('application/reactflow/nodeType');
      if (!nodeType || !flow) return;

      // 使用鼠标实际位置转换为流程坐标
      const position = screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
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
    },
    [screenToFlowPosition, flow, addNode]
  );

  // 处理点击添加节点请求 - 在当前视口中心添加节点
  useEffect(() => {
    if (!pendingAddNodeType || !flow) return;

    // 使用 canvasRef 获取画布容器
    const canvasElement = canvasRef.current;
    if (!canvasElement) {
      clearPendingAddNodeType();
      return;
    }

    const rect = canvasElement.getBoundingClientRect();
    const centerX = rect.left + rect.width / 2;
    const centerY = rect.top + rect.height / 2;

    // 将视口中心转换为流程坐标
    const position = screenToFlowPosition({ x: centerX, y: centerY });

    // 根据节点类型创建节点
    const nodeCount = flow.nodes.length + 1;
    const newNodeUuid = crypto.randomUUID();

    if (pendingAddNodeType === 'http') {
      addNode({
        uuid: newNodeUuid,
        type: 'http',
        name: `HTTP 请求 ${nodeCount}`,
        position,
        data: { type: 'http', url: '', method: 'get', headers: {}, body: '' },
      });
    } else if (pendingAddNodeType === 'conditional') {
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

    // 清除待添加节点状态
    clearPendingAddNodeType();
  }, [pendingAddNodeType, flow, addNode, screenToFlowPosition, clearPendingAddNodeType, canvasRef]);
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
      ref={canvasRef}
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
        /* Controls 按钮图标颜色 - 深色确保在白色背景可见 */
        .react-flow__controls-button svg {
          fill: #1e293b !important;
          stroke: #1e293b !important;
        }
        .react-flow__controls-button:hover svg {
          fill: #0f172a !important;
          stroke: #0f172a !important;
        }
        /* 拖拽时显示自定义 grabbing 光标 */
        .react-flow__node.dragging,
        .react-flow__node.dragging * {
          cursor: url("${GRABBING_CURSOR_SVG}") 8 8, grabbing !important;
        }
      `}</style>

      {/* 画布悬浮工具栏 */}
      <CanvasToolbar />
      
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={handleNodesChange}
        onEdgesChange={handleEdgesChange}
        onConnect={onConnect}
        onNodeDragStop={onNodeDragStop}
        onNodeClick={onNodeClick}
        onDragOver={onDragOver}
        onDrop={onDrop}
        onPaneClick={onPaneClick}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        deleteKeyCode={[]}
        // 禁用 Backspace/Delete 键删除节点/边
        onKeyDown={(e) => {
          if (e.key === 'Backspace' || e.key === 'Delete') {
            e.preventDefault();
            e.stopPropagation();
          }
        }}

        fitView
        fitViewOptions={{ padding: 0.2 }}
        minZoom={0.1}
        maxZoom={2}
        proOptions={{ hideAttribution: true }}
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
