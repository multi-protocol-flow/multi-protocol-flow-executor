/**
 * FlowCanvas - 主画布组件
 * 基于 React Flow 的流程编辑器
 */
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  ReactFlow,
  ReactFlowProvider,
  Background,
  Controls,
  ControlButton,
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

import { EntryNode, HttpNode, ConditionalNode, WsConnectNode, WsSendCollectNode, WsCloseNode, AssertionNode, DbConnectNode, DbQueryNode, DbBeginNode, DbCommitNode, DbRollbackNode, DbCloseNode, ScriptNode, GrpcConnectNode, GrpcCallNode, GrpcCloseNode, TcpConnectNode, TcpSendNode, TcpReceiveNode, TcpCloseNode, UdpBindNode, UdpSendToNode, UdpRecvFromNode, UdpCloseNode, McpConnectNode, McpCallToolNode, McpReadResourceNode, McpGetPromptNode, McpCloseNode, MqttConnectNode, MqttPublishNode, MqttSubscribeNode, MqttUnsubscribeNode, MqttDisconnectNode, MqttWaitMessageNode, MqttRequestNode, VariableExtractorNode, KafkaConnectNode, KafkaProduceNode, KafkaConsumeNode, KafkaConsumeBatchNode, KafkaAdminNode, KafkaSchemaRegistryNode, KafkaCloseNode, SseConnectNode, SseListenNode, SseDisconnectNode, GraphqlConnectNode, GraphqlQueryNode, GraphqlSubscribeNode, GraphqlUnsubscribeNode, GraphqlIntrospectNode, GraphqlDisconnectNode, ImapConnectNode, ImapSelectNode, ImapSearchNode, ImapFetchNode, ImapStoreFlagsNode, ImapCopyNode, ImapMoveNode, ImapExpungeNode, ImapListNode, ImapFolderCreateNode, ImapFolderDeleteNode, ImapIdleNode, ImapCloseNode, SmtpConnectNode, SmtpSendNode, SmtpDisconnectNode, ProxyNode } from './nodes';
import DynamicPluginNode from './nodes/DynamicPluginNode';
import { CanvasToolbar } from './CanvasToolbar';
import { NodePalette } from './NodePalette';
import { NodeContextMenu } from './NodeContextMenu';
import { CustomEdge } from './edges/CustomEdge';
import { useFlowStore, useUIStore, useDebugStore } from '@/stores';
import { useTheme } from '@/hooks/useTheme';
import { MapIcon, ArrowRightLeft } from 'lucide-react';
import type { NodeType } from '@/types';
import type { NodeMetadata } from '@/types/nodeRegistry';
import { getNodeMetadata } from '@/lib/NodeRegistry';
import { getLayoutedPositions, type LayoutDirection } from './layout';
import {
  convertFlowToReactFlow,
  convertEdgeToConnection,
  isValidConnection,
  createDefaultNodeData,
  createDefaultNodeName,
} from './utils';
import { cn } from '@/lib/utils';
import { flowStorage } from '@/services/storage';

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
  ws_connect: WsConnectNode,
  ws_send_collect: WsSendCollectNode,
  ws_close: WsCloseNode,
  assertion: AssertionNode,
  db_connect: DbConnectNode,
  db_query: DbQueryNode,
  db_begin: DbBeginNode,
  db_commit: DbCommitNode,
  db_rollback: DbRollbackNode,
  db_close: DbCloseNode,
  script: ScriptNode,
  grpc_connect: GrpcConnectNode,
  grpc_call: GrpcCallNode,
  grpc_close: GrpcCloseNode,
  tcp_connect: TcpConnectNode,
  tcp_send: TcpSendNode,
  tcp_receive: TcpReceiveNode,
  tcp_close: TcpCloseNode,
  udp_bind: UdpBindNode,
  udp_send_to: UdpSendToNode,
  udp_recv_from: UdpRecvFromNode,
  udp_close: UdpCloseNode,
  mcp_connect: McpConnectNode,
  mcp_call_tool: McpCallToolNode,
  mcp_read_resource: McpReadResourceNode,
  mcp_get_prompt: McpGetPromptNode,
  mcp_close: McpCloseNode,
  mqtt_connect: MqttConnectNode,
  mqtt_publish: MqttPublishNode,
  mqtt_subscribe: MqttSubscribeNode,
  mqtt_unsubscribe: MqttUnsubscribeNode,
  mqtt_disconnect: MqttDisconnectNode,
  mqtt_wait_message: MqttWaitMessageNode,
  mqtt_request: MqttRequestNode,
  variable_extractor: VariableExtractorNode,
  kafka_connect: KafkaConnectNode,
  kafka_produce: KafkaProduceNode,
  kafka_consume: KafkaConsumeNode,
  kafka_consume_batch: KafkaConsumeBatchNode,
  kafka_admin: KafkaAdminNode,
  kafka_schema_registry: KafkaSchemaRegistryNode,
  kafka_close: KafkaCloseNode,
  sse_connect: SseConnectNode,
  sse_listen: SseListenNode,
  sse_disconnect: SseDisconnectNode,
  graphql_connect: GraphqlConnectNode,
  graphql_query: GraphqlQueryNode,
  graphql_subscribe: GraphqlSubscribeNode,
  graphql_unsubscribe: GraphqlUnsubscribeNode,
  graphql_introspect: GraphqlIntrospectNode,
  graphql_disconnect: GraphqlDisconnectNode,
  imap_connect: ImapConnectNode,
  imap_select: ImapSelectNode,
  imap_search: ImapSearchNode,
  imap_fetch: ImapFetchNode,
  imap_store_flags: ImapStoreFlagsNode,
  imap_copy: ImapCopyNode,
  imap_move: ImapMoveNode,
  imap_expunge: ImapExpungeNode,
  imap_list: ImapListNode,
  imap_folder_create: ImapFolderCreateNode,
  imap_folder_delete: ImapFolderDeleteNode,
  imap_idle: ImapIdleNode,
  imap_close: ImapCloseNode,
  smtp_connect: SmtpConnectNode,
  smtp_send: SmtpSendNode,
  smtp_disconnect: SmtpDisconnectNode,
  proxy: ProxyNode,
  default: DynamicPluginNode,
};

/** 边类型映射 */
const edgeTypes = {
  custom: CustomEdge,
};

/** MiniMap 节点颜色 — 静态回退 */
const FALLBACK_MINIMAP_COLORS: Record<string, string> = {
  entry: '#22c55e',
  http: '#3b82f6',
  conditional: '#f97316',
  ws_connect: '#f59e0b',
  ws_send_collect: '#14b8a6',
  ws_close: '#f43f5e',
  assertion: '#a855f7',
  db_connect: '#8b5cf6',
  db_query: '#8b5cf6',
  db_begin: '#8b5cf6',
  db_commit: '#8b5cf6',
  db_rollback: '#8b5cf6',
  db_close: '#8b5cf6',
  script: '#6366f1',
  grpc_connect: '#8b5cf6',
  grpc_call: '#8b5cf6',
  grpc_close: '#8b5cf6',
  tcp_connect: '#10b981',
  tcp_send: '#10b981',
  tcp_receive: '#10b981',
  tcp_close: '#10b981',
  udp_bind: '#f97316',
  udp_send_to: '#f97316',
  udp_recv_from: '#f97316',
  udp_close: '#f97316',
  mcp_connect: '#f59e0b',
  mcp_call_tool: '#f59e0b',
  mcp_read_resource: '#f59e0b',
  mcp_close: '#f59e0b',
  mqtt_connect: '#06b6d4',
  mqtt_publish: '#06b6d4',
  mqtt_subscribe: '#06b6d4',
  mqtt_unsubscribe: '#06b6d4',
  mqtt_disconnect: '#06b6d4',
  mqtt_wait_message: '#06b6d4',
  mqtt_request: '#06b6d4',
  kafka_connect: '#f43f5e',
  kafka_produce: '#f43f5e',
  kafka_consume: '#f43f5e',
  kafka_consume_batch: '#f43f5e',
  kafka_admin: '#f43f5e',
  kafka_schema_registry: '#f43f5e',
  kafka_close: '#f43f5e',
  sse_connect: '#f43f5e',
  sse_listen: '#f43f5e',
  sse_disconnect: '#f43f5e',
  graphql_connect: '#6366f1',
  graphql_query: '#6366f1',
  graphql_subscribe: '#6366f1',
  graphql_unsubscribe: '#6366f1',
  graphql_introspect: '#6366f1',
  graphql_disconnect: '#6366f1',
  proxy: '#84cc16',
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
    isDirty,
    markSaved,
    focusNodeUuid,
    clearFocusNode,
    centerOnNodeUuid,
    clearCenterOnNode,
    batchUpdateNodePositions,
  } = useFlowStore();
  const { setConfigPanelOpen, setConfigPanelMode, setFlowListPanelOpen, showMinimap, toggleMinimap } = useUIStore();
  const { resolvedTheme } = useTheme();

  // 动态节点元数据
  const [nodeMetadata, setNodeMetadata] = useState<NodeMetadata[]>([]);

  // 挂载时获取元数据
  useEffect(() => {
    getNodeMetadata().then(setNodeMetadata);
  }, []);

  // MiniMap 节点颜色：优先使用后端元数据的颜色，回退到静态映射
  const minimapNodeColor = useCallback((node: Node) => {
    if (node.type) {
      const meta = nodeMetadata.find((m) => m.type_id === node.type);
      if (meta) return meta.color;
    }
    return FALLBACK_MINIMAP_COLORS[node.type ?? ''] ?? '#94a3b8';
  }, [nodeMetadata]);

  const { screenToFlowPosition, fitView, setCenter, getNode } = useReactFlow();
  
  // 画布容器引用（用于计算视口中心）
  const canvasRef = useRef<HTMLDivElement>(null);
  
  // 跟踪已触发 fitView 的流程，避免重复触发
  const fittedFlowUuidRef = useRef<string | null>(null);

  // 右键菜单状态
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    nodeId: string;
    nodeType: string;
    nodeName: string;
  } | null>(null);

  // 重命名浮动输入框状态
  const [renamingNodeId, setRenamingNodeId] = useState<string | null>(null);
  const [renamingValue, setRenamingValue] = useState('');
  const [renamingPosition, setRenamingPosition] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const renameInputRef = useRef<HTMLInputElement>(null);

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
        fitView({ padding: 0.4, duration: 300 });
      }, 100);
    }
  }, [flow?.uuid, fitView]);

  // 应用窗口大小变化时自动调整画布视图
  useEffect(() => {
    if (!flow) return;

    let debounceTimer: ReturnType<typeof setTimeout>;

    const handleResize = () => {
      clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => {
        fitView({ padding: 0.4, duration: 200 });
      }, 200);
    };

    window.addEventListener('resize', handleResize);

    return () => {
      clearTimeout(debounceTimer);
      window.removeEventListener('resize', handleResize);
    };
  }, [flow, fitView]);

  // 自动防抖保存：当 isDirty 变为 true 时，延迟保存到文件存储
  useEffect(() => {
    if (!isDirty || !flow) return;

    const timeoutId = setTimeout(() => {
      flowStorage.saveFlow(flow).then(() => markSaved()).catch((err) => {
        console.error('[FlowCanvas] 自动保存失败:', err);
      });
    }, 500); // 500ms 防抖

    return () => clearTimeout(timeoutId);
  }, [isDirty, flow, markSaved]);

  // 处理节点聚焦跳转（从调试面板/节点列表点击跳转）
  useEffect(() => {
    if (!focusNodeUuid) return;

    const node = getNode(focusNodeUuid);
    if (node) {
      // 跳转到节点位置，并放大
      setCenter(node.position.x, node.position.y, {
        zoom: 1.2,
        duration: 300,
      });
      // 选中该节点（Zustand store）
      selectNode(focusNodeUuid);
      // 同步 React Flow 节点选中状态（驱动视觉高亮）
      setNodes((nds) =>
        nds.map((n) => ({ ...n, selected: n.id === focusNodeUuid }))
      );
      // 清除聚焦状态
      clearFocusNode();
    }
  }, [focusNodeUuid, getNode, setCenter, selectNode, clearFocusNode, setNodes]);

  // 处理节点居中定位（从节点列表点击定位，仅居中不选中）
  useEffect(() => {
    if (!centerOnNodeUuid) return;

    const node = getNode(centerOnNodeUuid);
    if (node) {
      setCenter(node.position.x, node.position.y, {
        zoom: 1.2,
        duration: 300,
      });
      clearCenterOnNode();
    }
  }, [centerOnNodeUuid, getNode, setCenter, clearCenterOnNode]);

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
      // 调试模式下自动展开左侧面板（显示节点只读配置）
      const debugStore = useDebugStore.getState();
      if (debugStore.status !== 'idle' && debugStore.status !== 'ended') {
        setFlowListPanelOpen(true);
      }
    },
    [selectNode, setConfigPanelOpen, setConfigPanelMode, setFlowListPanelOpen]
  );

  // 处理边点击 - 取消节点选中，避免 Backspace 误删节点
  const onEdgeClick = useCallback(() => {
    selectNode(null);
    setConfigPanelOpen(false);
  }, [selectNode, setConfigPanelOpen]);

  // 处理画布空白处点击 - 取消选中，保持面板打开显示节点列表
  const onPaneClick = useCallback(() => {
    selectNode(null);
    setConfigPanelOpen(true);
    setContextMenu(null);
  }, [selectNode, setConfigPanelOpen]);

  // 自动布局处理
  const handleAutoLayout = useCallback((direction: LayoutDirection) => {
    if (!flow) return;

    const positions = getLayoutedPositions(flow.nodes, flow.connections, direction);
    batchUpdateNodePositions(positions);

    // 延迟 fitView，等待 React Flow 完成位置更新
    setTimeout(() => {
      fitView({ padding: 0.4, duration: 300 });
    }, 150);
  }, [flow, batchUpdateNodePositions, fitView]);

  // 处理节点右键菜单
  const onNodeContextMenu = useCallback(
    (event: React.MouseEvent, node: Node) => {
      event.preventDefault();
      const nodeData = node.data as { label: string };
      setContextMenu({
        x: event.clientX,
        y: event.clientY,
        nodeId: node.id,
        nodeType: node.type || '',
        nodeName: nodeData.label || '',
      });
    },
    []
  );

  // 复制节点（含源端连接复制）
  const handleCopyNode = useCallback(
    (nodeId: string) => {
      const { flow: currentFlow, addNode: addNewNode } = useFlowStore.getState();
      if (!currentFlow) return;

      const sourceNode = currentFlow.nodes.find((n) => n.uuid === nodeId);
      if (!sourceNode) return;

      const newUuid = crypto.randomUUID();
      const offset = 50;

      // 深拷贝节点数据（孤立节点，不复制连接）
      addNewNode({
        uuid: newUuid,
        type: sourceNode.type,
        name: `${sourceNode.name} (副本)`,
        position: {
          x: sourceNode.position.x + offset,
          y: sourceNode.position.y + offset,
        },
        data: JSON.parse(JSON.stringify(sourceNode.data)),
      });
    },
    []
  );

  // 删除节点（通过确认弹窗）
  const handleDeleteNode = useCallback(
    (nodeId: string) => {
      const { flow: currentFlow, removeNode: removeNodeById, selectNode: deselectNode } = useFlowStore.getState();
      if (!currentFlow) return;

      const node = currentFlow.nodes.find((n) => n.uuid === nodeId);
      if (!node || node.type === 'entry') return;

      const { showConfirm } = useUIStore.getState();
      showConfirm({
        title: '删除节点',
        description: `确定要删除节点「${node.name}」吗？`,
        confirmText: '删除',
        onConfirm: () => {
          removeNodeById(node.uuid);
          deselectNode(null);
        },
      });
    },
    []
  );

  // 触发重命名浮动输入框
  const handleRenameNode = useCallback(
    (nodeId: string) => {
      const { flow: currentFlow } = useFlowStore.getState();
      if (!currentFlow) return;

      const node = currentFlow.nodes.find((n) => n.uuid === nodeId);
      if (!node) return;

      // 使用右键菜单位置作为输入框位置
      setRenamingNodeId(nodeId);
      setRenamingValue(node.name);
      setRenamingPosition(
        contextMenu
          ? { x: contextMenu.x, y: contextMenu.y }
          : { x: 0, y: 0 }
      );
    },
    [contextMenu]
  );

  // 重命名确认
  const handleRenameConfirm = useCallback(() => {
    if (renamingNodeId && renamingValue.trim()) {
      updateNode(renamingNodeId, { name: renamingValue.trim() });
    }
    setRenamingNodeId(null);
    setRenamingValue('');
  }, [renamingNodeId, renamingValue, updateNode]);

  // 重命名取消
  const handleRenameCancel = useCallback(() => {
    setRenamingNodeId(null);
    setRenamingValue('');
  }, []);

  // 处理节点变化（过滤掉删除变化，只处理位置变化等）
  // 注意：节点删除需要通过工具栏按钮确认，防止误操作
  const handleNodesChange = useCallback(
    (changes: NodeChange[]) => {
      // 过滤掉删除变化，只处理其他变化（如位置移动）
      const nonRemoveChanges = changes.filter((c) => c.type !== 'remove');
      if (nonRemoveChanges.length > 0) {
        onNodesChange(nonRemoveChanges);
      }
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
      const data = createDefaultNodeData(nodeType as NodeType);
      const name = createDefaultNodeName(nodeType as NodeType, nodeCount);
      addNode({
        uuid: newNodeUuid,
        type: nodeType as NodeType,
        name,
        position,
        data,
      });
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
    const data = createDefaultNodeData(pendingAddNodeType);
    const name = createDefaultNodeName(pendingAddNodeType, nodeCount);
    addNode({
      uuid: newNodeUuid,
      type: pendingAddNodeType,
      name,
      position,
      data,
    });

    // 清除待添加节点状态
    clearPendingAddNodeType();
  }, [pendingAddNodeType, flow, addNode, screenToFlowPosition, clearPendingAddNodeType, canvasRef]);
  // 没有流程时显示空状态
  if (!flow) {
    return (
      <div className="w-full h-full flex items-center justify-center bg-background">
        <div className="text-center">
          <div className="text-6xl mb-4">📋</div>
          <h2 className="text-xl font-semibold text-muted-foreground">
            暂无流程
          </h2>
          <p className="text-sm text-muted-foreground mt-2">
            请创建或打开一个流程文件
          </p>
        </div>
      </div>
    );
  }

  return (
    <div 
      ref={canvasRef}
      className={cn('w-full h-full', 'bg-background')}
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
        {/* Controls 按钮图标颜色 - 使用 CSS 变量跟随主题 */}
        .react-flow__controls-button svg {
          fill: var(--controls-icon) !important;
          stroke: var(--controls-icon) !important;
        }
        .react-flow__controls-button:hover svg {
          fill: var(--foreground) !important;
          stroke: var(--foreground) !important;
        }
        /* Controls 按钮背景 - 跟随主题 */
        .react-flow__controls-button {
          background-color: var(--background) !important;
          border-color: var(--border) !important;
        }
        .react-flow__controls-button:hover {
          background-color: var(--accent) !important;
        }
        /* Controls / MiniMap 边框 - 强制显示 */
        .react-flow__controls,
        .react-flow__minimap {
          border-width: 1px !important;
          border-style: solid !important;
          border-color: var(--border) !important;
        }
        /* 暗黑模式边框增强 - 浅色默认的 10% 白太淡 */
        .dark .react-flow__controls,
        .dark .react-flow__minimap {
          border-color: rgba(255, 255, 255, 0.15) !important;
        }
        .dark .react-flow__controls-button {
          border-color: rgba(255, 255, 255, 0.1) !important;
        }
        /* 拖拽时显示自定义 grabbing 光标 */
        .react-flow__node.dragging,
        .react-flow__node.dragging * {
          cursor: url("${GRABBING_CURSOR_SVG}") 8 8, grabbing !important;
        }
        /* 节点自动布局动画 */
        .react-flow__node {
          transition: transform 300ms ease-out;
        }
        /* 拖拽时禁用过渡动画，避免拖拽卡顿 */
        .react-flow__node.dragging {
          transition: none !important;
        }
      `}</style>

      {/* 画布悬浮工具栏 */}
      <CanvasToolbar />

      {/* 画布底部节点选择工具条 */}
      <NodePalette />
      
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={handleNodesChange}
        onEdgesChange={handleEdgesChange}
        onConnect={onConnect}
        onNodeDragStop={onNodeDragStop}
        onNodeClick={onNodeClick}
        onNodeContextMenu={onNodeContextMenu}
        onEdgeClick={onEdgeClick}
        onDragOver={onDragOver}
        onDrop={onDrop}
        onPaneClick={onPaneClick}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        // Backspace 和 Delete 键触发删除操作
        // 节点删除：由 useKeyboardShortcuts 在 capture 阶段拦截并弹出确认弹窗
        // 边删除：由 handleEdgesChange 处理（仅未选中节点时生效）
        deleteKeyCode={['Backspace', 'Delete']}

        fitView

        fitViewOptions={{ padding: 0.4 }}
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
          color="var(--canvas-dot)"
        />

        {/* 控制按钮 */}
        <Controls
          className="!bg-background !border-border"
          showZoom
          showFitView
          showInteractive
        >
          {/* 小地图开关 */}
          <ControlButton
            onClick={toggleMinimap}
            title={showMinimap ? '关闭小地图' : '打开小地图'}
          >
            <MapIcon className="w-4 h-4" />
          </ControlButton>
          {/* 水平自动布局 */}
          <ControlButton
            onClick={() => handleAutoLayout('LR')}
            title="水平自动布局"
          >
            <ArrowRightLeft className="w-4 h-4" />
          </ControlButton>
        </Controls>

        {/* 小地图 */}
        {showMinimap && (
          <MiniMap
            nodeColor={minimapNodeColor}
            maskColor={resolvedTheme === 'dark' ? 'rgba(0, 0, 0, 0.5)' : 'rgba(0, 0, 0, 0.1)'}
            className="!bg-background !border-border"
            pannable
            zoomable
          />
        )}
      </ReactFlow>

      {/* 节点右键菜单 */}
      {contextMenu && (
        <NodeContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          nodeId={contextMenu.nodeId}
          nodeType={contextMenu.nodeType}
          nodeName={contextMenu.nodeName}
          onClose={() => setContextMenu(null)}
          onCopy={handleCopyNode}
          onDelete={handleDeleteNode}
          onRename={handleRenameNode}
        />
      )}

      {/* 重命名浮动输入框 */}
      {renamingNodeId && (
        <div
          className={cn(
            'fixed z-50',
            'animate-in fade-in-0 zoom-in-95 duration-100',
          )}
          style={{ left: renamingPosition.x, top: renamingPosition.y }}
        >
          <input
            ref={renameInputRef}
            type="text"
            value={renamingValue}
            onChange={(e) => setRenamingValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                handleRenameConfirm();
              } else if (e.key === 'Escape') {
                handleRenameCancel();
              }
            }}
            onBlur={handleRenameConfirm}
            autoFocus
            className={cn(
              'w-48 px-2 py-1.5 text-sm',
              'bg-background border border-border rounded-md',
              'shadow-lg outline-none',
              'focus:ring-2 focus:ring-ring focus:ring-offset-1',
            )}
          />
          <div className="text-[10px] text-muted-foreground mt-0.5 px-1">
            Enter 确认 · Esc 取消
          </div>
        </div>
      )}
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
