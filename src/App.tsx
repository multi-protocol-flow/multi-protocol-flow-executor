import React, { useCallback, useMemo } from 'react';
import type { Node, Edge, Connection, NodeChange, EdgeChange } from '@xyflow/react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  applyNodeChanges,
  applyEdgeChanges,
  BackgroundVariant,
  type NodeMouseHandler,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { cn } from './lib/utils';
import { ContextMenu } from './components/ContextMenu';
import { ConfigPanel, type NodeConfigValues } from './components/ConfigPanel';

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
      data-testid={data?.dataTestId}
      className={cn(
        'px-4 py-3 rounded-full border-2 shadow-lg transition-all duration-200',
        nodeColor,
        selected && 'ring-2 ring-blue-500 ring-offset-2 ring-offset-slate-900'
      )}
    >
      <div className="text-white font-medium text-sm" data-testid="node-label">
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
 * App 主组件
 *
 * 功能：
 * - 自动创建开始节点（绿色、圆形、data-testid="start-node"）
 * - React Flow 画布正确渲染
 * - 支持节点拖拽
 * - 右键点击开始节点显示菜单（重命名、复制、查看信息）
 * - 可以通过右键菜单打开配置面板
 * - 可以编辑节点名称并保存
 * - 开始节点不能被删除
 */
function App() {
  // 状态管理
  const [nodes, setNodes] = React.useState<Node[]>([]);
  const [edges, setEdges] = React.useState<Edge[]>([]);
  const [selectedNode, setSelectedNode] = React.useState<Node | null>(null);
  const [isConfigPanelOpen, setIsConfigPanelOpen] = React.useState(false);
  const [contextMenuNode, setContextMenuNode] = React.useState<Node | null>(null);
  const [contextMenuPosition, setContextMenuPosition] = React.useState<{ x: number; y: number } | null>(null);

  // 初始化：自动创建开始节点
  React.useEffect(() => {
    const startNodeId = 'start-node-id-' + Date.now();
    const startNode: Node = {
      id: startNodeId,
      type: 'start',
      position: { x: 250, y: 100 },
      data: {
        label: '开始',
        nodeType: 'start',
        description: '工作流的起点',
        dataTestId: 'start-node',
      },
    };
    setNodes([startNode]);
  }, []);

  /**
   * 处理节点变化（拖拽等）
   */
  const handleNodesChange = useCallback(
    (changes: NodeChange[]) => {
      const updatedNodes = applyNodeChanges(changes, nodes);
      setNodes(updatedNodes);

      // Track selected node
      const selected = updatedNodes.find((n) => n.selected);
      setSelectedNode(selected || null);
    },
    [nodes]
  );

  /**
   * 处理边变化（连接删除等）
   */
  const handleEdgesChange = useCallback(
    (changes: EdgeChange[]) => {
      const updatedEdges = applyEdgeChanges(changes, edges);
      setEdges(updatedEdges);
    },
    [edges]
  );

  /**
   * 处理新连接创建
   */
  const handleConnect = useCallback(
    (connection: Connection) => {
      const newEdge: Edge = {
        id: `edge-${connection.source}-${connection.target}-${Date.now()}`,
        source: connection.source || '',
        target: connection.target || '',
        sourceHandle: connection.sourceHandle || null,
        targetHandle: connection.targetHandle || null,
        type: 'smoothstep',
        animated: true,
      };
      setEdges((eds) => [...eds, newEdge]);
    },
    []
  );

  /**
   * 处理节点删除（阻止删除开始节点）
   */
  const handleNodesDelete = useCallback(
    (nodesToDelete: Node[]) => {
      const hasStartNode = nodesToDelete.some(
        (node) => node.data?.nodeType === 'start'
      );

      if (hasStartNode) {
        console.error('不能删除开始节点');
        return;
      }

      setNodes((nds) => nds.filter((n) => !nodesToDelete.includes(n)));
    },
    []
  );

  /**
   * 处理节点右键点击
   */
  const handleNodeContextMenu: NodeMouseHandler = useCallback(
    (event, node) => {
      event.preventDefault();
      setContextMenuNode(node);
      setContextMenuPosition({ x: event.clientX, y: event.clientY });
    },
    []
  );

  /**
   * 处理画布点击：关闭右键菜单
   */
  const handlePaneClick = useCallback(() => {
    setContextMenuNode(null);
    setContextMenuPosition(null);
  }, []);

  /**
   * 处理重命名：打开配置面板
   */
  const handleRename = useCallback(() => {
    if (contextMenuNode) {
      setSelectedNode(contextMenuNode);
      setIsConfigPanelOpen(true);
      setContextMenuNode(null);
      setContextMenuPosition(null);
    }
  }, [contextMenuNode]);

  /**
   * 处理复制节点
   */
  const handleCopy = useCallback(() => {
    if (!contextMenuNode) return;

    const newNode: Node = {
      ...contextMenuNode,
      id: 'node-' + Date.now(),
      position: {
        x: contextMenuNode.position.x + 50,
        y: contextMenuNode.position.y + 50,
      },
      // Reset nodeType for copied nodes (except start nodes)
      data: {
        ...contextMenuNode.data,
        nodeType: contextMenuNode.data?.nodeType === 'start' ? undefined : contextMenuNode.data?.nodeType,
      },
    };

    setNodes((nds) => [...nds, newNode]);
    setContextMenuNode(null);
    setContextMenuPosition(null);
  }, [contextMenuNode]);

  /**
   * 处理查看信息
   */
  const handleViewInfo = useCallback(() => {
    if (!contextMenuNode) return;
    // For now, just log the node info
    console.log('Node Info:', contextMenuNode);
    // TODO: Display node info in a toast or modal
    setContextMenuNode(null);
    setContextMenuPosition(null);
  }, [contextMenuNode]);

  /**
   * 处理配置保存：更新节点名称
   */
  const handleConfigSave = useCallback((values: NodeConfigValues) => {
    if (!selectedNode) return;

    setNodes((nds) =>
      nds.map((node) =>
        node.id === selectedNode.id
          ? {
              ...node,
              data: {
                ...node.data,
                label: values.name,
                name: values.name,
              },
            }
          : node
      )
    );
  }, [selectedNode]);

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

  // Debug: log when component renders
  React.useEffect(() => {
    console.log('App component rendered, nodes:', nodes.length);
  }, [nodes]);

  return (
    <div className="w-screen h-screen bg-slate-950">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={handleNodesChange}
        onEdgesChange={handleEdgesChange}
        onConnect={handleConnect}
        onNodesDelete={handleNodesDelete}
        onNodeContextMenu={handleNodeContextMenu}
        onPaneClick={handlePaneClick}
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

      {/* 右键菜单 - 使用自定义定位 */}
      {contextMenuNode && contextMenuPosition && (
        <div
          style={{
            position: 'absolute',
            left: contextMenuPosition.x,
            top: contextMenuPosition.y,
            zIndex: 9999,
          }}
        >
          <ContextMenu
            open={!!contextMenuNode}
            onOpenChange={(isOpen) => {
              if (!isOpen) {
                setContextMenuNode(null);
                setContextMenuPosition(null);
              }
            }}
            onRename={handleRename}
            onCopy={handleCopy}
            onViewInfo={handleViewInfo}
          >
            <div className="w-0 h-0" />
          </ContextMenu>
        </div>
      )}

      <ConfigPanel
        open={isConfigPanelOpen}
        onOpenChange={setIsConfigPanelOpen}
        node={selectedNode}
        onSave={handleConfigSave}
      />
    </div>
  );
}

export default App;
