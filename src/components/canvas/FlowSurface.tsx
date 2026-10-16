/**
 * FlowSurface - 画布核心渲染表面
 * 承载 ReactFlow 实例、节点/边状态、视口交互及画布内部装饰层 (Background/Controls/MiniMap)。
 * 下沉状态以保证节点拖拽/画布平移帧不触发 FlowCanvasInner 重渲染。
 */
import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  ControlButton,
  MiniMap,
  useNodesState,
  useEdgesState,
  addEdge,
  useReactFlow,
  type Connection,
  type Edge,
  type Node,
  type NodeChange,
  type EdgeChange,
  type DefaultEdgeOptions,
  type FitViewOptions,
  type ProOptions,
  BackgroundVariant,
  SelectionMode,
} from '@xyflow/react';
import {
  EntryNode,
  HttpNode,
  ConditionalNode,
  SubFlowNode,
  WsConnectNode,
  WsSendCollectNode,
  WsCloseNode,
  AssertionNode,
  ScriptNode,
  TcpConnectNode,
  TcpSendNode,
  TcpReceiveNode,
  TcpCloseNode,
  UdpBindNode,
  UdpSendToNode,
  UdpRecvFromNode,
  UdpCloseNode,
  VariableExtractorNode,
  SseConnectNode,
  SseListenNode,
  SseDisconnectNode,
  GraphqlConnectNode,
  GraphqlQueryNode,
  GraphqlSubscribeNode,
  GraphqlIntrospectNode,
  GraphqlDisconnectNode,
  ProxyNode,
  EndNode,
} from './nodes';
import DynamicPluginNode from './nodes/DynamicPluginNode';
import { CustomEdge } from './edges/CustomEdge';
import { SelectionAlignmentBar } from './SelectionAlignmentBar';
import {
  useFlowStore,
  useUIStore,
  useDebugStore,
  useExecutionStore,
  selectIsExecuting,
  selectFlow,
  selectActiveStage,
  getStageGraph,
} from '@/stores';
import { useTheme } from '@/hooks/useTheme';
import { useCurrentLocale } from '@/hooks/useLocale';
import { MapIcon, ArrowRightLeft, WrapText, Search } from 'lucide-react';
import type { Flow, FlowSubGraph } from '@/types';
import type { FlowStage } from '@/stores';
import { getNodeMetadata, getCachedNodeMetadataList } from '@/lib/NodeRegistry';
import { getLayoutedPositions, getAdaptiveLayoutedPositions, type LayoutDirection } from './layout';
import {
  convertFlowToReactFlow,
  convertEdgeToConnection,
  isValidConnection,
  getConnectionErrorReason,
  findEdgeToInsert,
  insertNodeIntoConnection,
  type FlowNodeData,
} from './utils';
import { getLL } from '@/i18n/i18n-lazy';
import { toast } from 'sonner';
import { FALLBACK_MINIMAP_COLORS } from './FlowCanvas';

/** 节点类型映射 */
const nodeTypes = {
  entry: EntryNode,
  'http:request': HttpNode,
  conditional: ConditionalNode,
  'ws:connect': WsConnectNode,
  'ws:send_collect': WsSendCollectNode,
  'ws:close': WsCloseNode,
  assertion: AssertionNode,
  script: ScriptNode,
  'tcp:connect': TcpConnectNode,
  'tcp:send': TcpSendNode,
  'tcp:receive': TcpReceiveNode,
  'tcp:close': TcpCloseNode,
  'udp:bind': UdpBindNode,
  'udp:send_to': UdpSendToNode,
  'udp:recv_from': UdpRecvFromNode,
  'udp:close': UdpCloseNode,
  'variable:extractor': VariableExtractorNode,
  'sse:connect': SseConnectNode,
  'sse:listen': SseListenNode,
  'sse:disconnect': SseDisconnectNode,
  'graphql:connect': GraphqlConnectNode,
  'graphql:query': GraphqlQueryNode,
  'graphql:subscribe': GraphqlSubscribeNode,
  'graphql:introspect': GraphqlIntrospectNode,
  'graphql:disconnect': GraphqlDisconnectNode,
  proxy: ProxyNode,
  end: EndNode,
  'flow:sub': SubFlowNode,
  default: DynamicPluginNode,
};

/** 边类型映射 */
const edgeTypes = {
  custom: CustomEdge,
};

const DEFAULT_EDGE_OPTIONS: DefaultEdgeOptions = { type: 'custom', animated: false };
const FIT_VIEW_OPTIONS: FitViewOptions = { padding: 0.4 };
const PRO_OPTIONS: ProOptions = { hideAttribution: true };
const DELETE_KEY_CODES: string[] = ['Backspace', 'Delete'];

export interface FlowSurfaceProps {
  containerRef: React.RefObject<HTMLDivElement | null>;
  onDragOver: (event: React.DragEvent) => void;
  onDrop: (event: React.DragEvent) => Promise<void>;
  onNodeContextMenu: (event: React.MouseEvent, node: Node) => void;
  onSelectionContextMenu?: (event: React.MouseEvent, nodes: Node[]) => void;
  onCloseContextMenu?: () => void;
}

export const FlowSurface = memo(function FlowSurface({
  containerRef,
  onDragOver,
  onDrop,
  onNodeContextMenu,
  onSelectionContextMenu,
  onCloseContextMenu,
}: FlowSurfaceProps) {
  const flow = useFlowStore(selectFlow);
  const activeStage = useFlowStore(selectActiveStage);
  const selectedNodeId = useFlowStore((s) => s.selectedNodeId);
  const focusNodeUuid = useFlowStore((s) => s.focusNodeUuid);
  const clearFocusNode = useFlowStore((s) => s.clearFocusNode);
  const centerOnNodeUuid = useFlowStore((s) => s.centerOnNodeUuid);
  const clearCenterOnNode = useFlowStore((s) => s.clearCenterOnNode);
  const addConnection = useFlowStore((s) => s.addConnection);
  const removeConnection = useFlowStore((s) => s.removeConnection);
  const updateNode = useFlowStore((s) => s.updateNode);
  const selectNode = useFlowStore((s) => s.selectNode);
  const batchUpdateNodePositions = useFlowStore((s) => s.batchUpdateNodePositions);

  const showMinimap = useUIStore((s) => s.showMinimap);
  const toggleMinimap = useUIStore((s) => s.toggleMinimap);
  const setConfigPanelOpen = useUIStore((s) => s.setConfigPanelOpen);
  const setConfigPanelMode = useUIStore((s) => s.setConfigPanelMode);
  const setFlowListPanelOpen = useUIStore((s) => s.setFlowListPanelOpen);

  const isExecuting = useExecutionStore(selectIsExecuting);
  const { resolvedTheme } = useTheme();
  const locale = useCurrentLocale();
  const ll = getLL();

  const { fitView, setCenter, getNode, getNodes, setViewport } = useReactFlow();

  // 转换 Flow 数据为 React Flow 初始格式（按当前阶段取子图）
  const initialData = useMemo(() => {
    if (!flow) return { nodes: [], edges: [] };
    return convertFlowToReactFlow(flow, activeStage);
  }, [flow, activeStage]);

  // 节点和边状态（下沉到 FlowSurface，拖拽帧仅触发 FlowSurface 重渲染）
  const [nodes, setNodes, onNodesChange] = useNodesState(initialData.nodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialData.edges);
  const selectedNodes = useMemo(() => nodes.filter((n) => n.selected), [nodes]);


  // 动态节点元数据颜色 Map（O(1) 查找）
  const [nodeColorMap, setNodeColorMap] = useState<Map<string, string>>(() => {
    const list = getCachedNodeMetadataList();
    return list ? new Map(list.map((m) => [m.type_id, m.color])) : new Map();
  });
  useEffect(() => {
    getNodeMetadata().then((list) =>
      setNodeColorMap(new Map(list.map((m) => [m.type_id, m.color])))
    );
  }, []);

  const minimapNodeColor = useCallback(
    (node: Node) =>
      (node.type ? nodeColorMap.get(node.type) ?? FALLBACK_MINIMAP_COLORS[node.type] : undefined) ?? '#94a3b8',
    [nodeColorMap],
  );

  // 节点/边身份保持（同步 effect 重写）
  const prevFlowUuidRef = useRef<string | null>(null);
  const prevStageRef = useRef<FlowStage | null>(null);
  const prevConnectionsRef = useRef<FlowSubGraph['connections'] | null>(null);
  const nodesLengthRef = useRef(initialData.nodes.length);

  useEffect(() => {
    if (!flow) {
      if (prevFlowUuidRef.current !== null) {
        prevFlowUuidRef.current = null;
        prevStageRef.current = null;
        prevConnectionsRef.current = null;
        nodesLengthRef.current = 0;
        setNodes([]);
        setEdges([]);
      }
      return;
    }
    const { nodes: newNodes, edges: newEdges } = convertFlowToReactFlow(flow, activeStage);

    const isSwitchingFlow = flow.uuid !== prevFlowUuidRef.current || activeStage !== prevStageRef.current;
    prevFlowUuidRef.current = flow.uuid;
    prevStageRef.current = activeStage;
    const stageConnections = getStageGraph(flow, activeStage).connections;
    const connectionsChanged = prevConnectionsRef.current !== stageConnections;
    prevConnectionsRef.current = stageConnections;
    if (isSwitchingFlow) {
      finalClassNameDoneRef.current = false;
      const { nodeReportsMap } = useExecutionStore.getState();
      nodesLengthRef.current = newNodes.length;
      setNodes(
        newNodes.map((n) => {
          const report = nodeReportsMap[n.id];
          let className = n.className;
          if (report?.status === 'success') className = `${className || ''} node-succeeded`.trim();
          else if (report?.status === 'failed') className = `${className || ''} node-failed`.trim();
          return { ...n, className, selected: n.id === selectedNodeId };
        }),
      );
      setEdges(newEdges);
      return;
    }

    const newNodeById = new Map(newNodes.map((n) => [n.id, n]));
    if (newNodeById.size !== nodesLengthRef.current) {
      // 结构变化（增/删节点）→ 全量重建
      nodesLengthRef.current = newNodes.length;
      setNodes((prevNodes) =>
        prevNodes.length === newNodes.length &&
        prevNodes.every((n) => newNodeById.has(n.id))
          ? prevNodes
          : newNodes.map((n) => ({ ...n, selected: n.id === selectedNodeId })),
      );
    } else {
      setNodes((prevNodes) => {
        // 若节点数量相同但成员发生变化（同数量替换），整体重建
        if (!prevNodes.every((n) => newNodeById.has(n.id))) {
          return newNodes.map((n) => ({ ...n, selected: n.id === selectedNodeId }));
        }
        let changed = false;
        const next = prevNodes.map((prevNode) => {
          const newNode = newNodeById.get(prevNode.id);
          if (!newNode) return prevNode;
          const prevData = prevNode.data as FlowNodeData;
          const nextData = newNode.data as FlowNodeData;
          const dataUnchanged =
            prevData.label === nextData.label &&
            prevData.config === nextData.config &&
            prevData.disabled === nextData.disabled;
          const positionUnchanged =
            prevNode.position.x === newNode.position.x &&
            prevNode.position.y === newNode.position.y;
          const selected = prevNode.id === selectedNodeId;
          const classNameUnchanged = prevNode.className === newNode.className;
          if (dataUnchanged && positionUnchanged && prevNode.selected === selected && classNameUnchanged) {
            return prevNode; // 保持对象身份 → adoptUserNodes 走 checkEquality 短路
          }
          changed = true;
          return {
            ...prevNode,
            position: positionUnchanged ? prevNode.position : newNode.position,
            data: dataUnchanged ? prevNode.data : newNode.data,
            className: newNode.className,
            selected,
          };
        });
        return changed ? next : prevNodes;
      });
    }

    if (connectionsChanged) setEdges(newEdges);
  }, [flow, activeStage, selectedNodeId, setNodes, setEdges]);

  // 处理节点聚焦跳转（从调试面板/节点列表点击跳转）
  useEffect(() => {
    if (!focusNodeUuid) return;

    const node = getNode(focusNodeUuid);
    if (node) {
      setCenter(node.position.x, node.position.y, {
        zoom: 1.2,
        duration: 300,
      });
      selectNode(focusNodeUuid);
      setNodes((nds) =>
        nds.map((n) => ({ ...n, selected: n.id === focusNodeUuid }))
      );
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

  // 执行节点高亮 - 通过 Zustand subscribe 直接订阅（不触发 React 重渲染）
  const execStateRef = useRef({
    currentNodeUuid: null as string | null,
    nodeReportsMap: {} as Record<string, import('@/types/report').NodeExecutionReport>,
    isExecuting: false,
  });
  const classNameUpdateRafRef = useRef<number | null>(null);
  const finalClassNameDoneRef = useRef(false);

  useEffect(() => {
    const init = useExecutionStore.getState();
    execStateRef.current = {
      currentNodeUuid: init.currentNodeUuid,
      nodeReportsMap: init.nodeReportsMap,
      isExecuting: init.isExecuting,
    };

    const updateNodeClasses = () => {
      const { nodeReportsMap, isExecuting } = execStateRef.current;
      const nodeReportsCount = Object.keys(nodeReportsMap).length;

      if (!isExecuting && nodeReportsCount === 0) return;

      if (isExecuting) return;

      if (finalClassNameDoneRef.current) return;
      finalClassNameDoneRef.current = true;

      setNodes((nds) => {
        let changed = false;
        const updated = nds.map((node) => {
          const nodeUuid = node.id;
          const report = nodeReportsMap[nodeUuid];
          const status = report?.status;

          const baseClasses = (node.className || '')
            .replace(/\s*node-executing\s*/g, ' ')
            .replace(/\s*node-succeeded\s*/g, ' ')
            .replace(/\s*node-failed\s*/g, ' ')
            .trim();

          let newClass = baseClasses;
          if (status === 'success') newClass += ' node-succeeded';
          if (status === 'failed') newClass += ' node-failed';

          const finalClass = newClass || undefined;
          if (node.className !== finalClass) {
            changed = true;
            return { ...node, className: finalClass };
          }
          return node;
        });
        return changed ? updated : nds;
      });
    };

    const unsub = useExecutionStore.subscribe(() => {
      const s = useExecutionStore.getState();
      execStateRef.current = {
        currentNodeUuid: s.currentNodeUuid,
        nodeReportsMap: s.nodeReportsMap,
        isExecuting: s.isExecuting,
      };
      if (s.isExecuting) {
        finalClassNameDoneRef.current = false;
      }
      if (classNameUpdateRafRef.current !== null) {
        cancelAnimationFrame(classNameUpdateRafRef.current);
      }
      classNameUpdateRafRef.current = requestAnimationFrame(() => {
        classNameUpdateRafRef.current = null;
        updateNodeClasses();
      });
    });
    return () => {
      unsub();
      if (classNameUpdateRafRef.current !== null) {
        cancelAnimationFrame(classNameUpdateRafRef.current);
      }
    };
  }, [setNodes]);

  // 连接验证与创建（使用 ref 保持稳定引用）
  const activeStageConnections = flow ? getStageGraph(flow, activeStage).connections : [];
  const connectionsRef = useRef<Flow['connections']>(activeStageConnections);
  connectionsRef.current = activeStageConnections;

  const isValidConnectionProp = useCallback(
    (connection: Connection | Edge) =>
      isValidConnection(
        connection.source || '',
        connection.target || '',
        connection.sourceHandle || 'out',
        connection.targetHandle || 'in',
        connectionsRef.current,
      ),
    [],
  );

  const onConnect = useCallback(
    (connection: Connection) => {
      const reason = getConnectionErrorReason(
        connection.source || '',
        connection.target || '',
        connection.sourceHandle || 'out',
        connection.targetHandle || 'in',
        connectionsRef.current,
      );
      if (reason) {
        if (reason === 'port_used') {
          toast.warning(getLL().flow.multi_branch_warning());
        } else if (reason === 'cycle') {
          toast.warning(getLL().flow.validation_cycle_detected());
        }
        return;
      }

      setEdges((eds) =>
        addEdge(
          {
            ...connection,
            type: 'custom',
            data: { sourceHandle: connection.sourceHandle },
          },
          eds,
        ),
      );

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
    [setEdges, addConnection],
  );

  const onSelectionDragStop = useCallback(
    (_: unknown, draggedNodes: Node[]) => {
      if (!draggedNodes || draggedNodes.length === 0) return;
      const positions = new Map<string, { x: number; y: number }>();
      for (const node of draggedNodes) {
        positions.set(node.id, {
          x: Math.round(node.position.x),
          y: Math.round(node.position.y),
        });
      }
      batchUpdateNodePositions(positions);
    },
    [batchUpdateNodePositions],
  );

  const onNodeDragStop = useCallback(
    (_: unknown, node: Node) => {
      const selectedNodes = getNodes().filter((n) => n.selected);
      // 若当前有多个节点被选中且当前节点也在选区中，交由 onSelectionDragStop 统一批量处理，避免多次连续单个 updateNode 导致撤销历史雪崩
      if (selectedNodes.length > 1 && selectedNodes.some((n) => n.id === node.id)) {
        return;
      }
      const newPos = { x: Math.round(node.position.x), y: Math.round(node.position.y) };
      updateNode(node.id, { position: newPos });

      // 连线智能插入：若该节点尚无任何连线（孤立节点），且拖拽落点在某条连线上，则自动拆分插入
      const currentFlow = useFlowStore.getState().flow;
      if (currentFlow && node.type !== 'entry' && node.type !== 'end') {
        const stageGraph = getStageGraph(currentFlow, activeStage);
        const isIsolated = !stageGraph.connections.some(
          (c) => c.source_node_uuid === node.id || c.target_node_uuid === node.id
        );
        if (isIsolated) {
          const edgeToInsert = findEdgeToInsert(newPos, stageGraph.nodes, stageGraph.connections);
          if (edgeToInsert) {
            insertNodeIntoConnection(node.id, edgeToInsert, addConnection, removeConnection);
            toast.success(getLL().flow.inserted_between_nodes());
          }
        }
      }
    },
    [updateNode, getNodes, activeStage, addConnection, removeConnection],
  );

  const onNodeClick = useCallback(
    (event: React.MouseEvent, node: Node) => {
      onCloseContextMenu?.();
      // 多选操作（按住 Ctrl / Meta / Shift 点选）交给 ReactFlow 内部处理多选，不覆盖单选面板
      if (event.ctrlKey || event.metaKey || event.shiftKey) {
        return;
      }
      selectNode(node.id);
      setConfigPanelOpen(true);
      setConfigPanelMode('node');
      useUIStore.getState().closeNodePalette();
      const debugStore = useDebugStore.getState();
      if (debugStore.status !== 'idle' && debugStore.status !== 'ended') {
        setFlowListPanelOpen(true);
      }
    },
    [selectNode, setConfigPanelOpen, setConfigPanelMode, setFlowListPanelOpen, onCloseContextMenu],
  );

  const onEdgeClick = useCallback(() => {
    onCloseContextMenu?.();
    selectNode(null);
    setConfigPanelOpen(false);
  }, [selectNode, setConfigPanelOpen, onCloseContextMenu]);

  const onPaneClick = useCallback(() => {
    onCloseContextMenu?.();
    selectNode(null);
    setConfigPanelMode('node');
    useUIStore.getState().closeNodePalette();
  }, [selectNode, setConfigPanelMode, onCloseContextMenu]);
  const onMove = useCallback(() => {
    onCloseContextMenu?.();
  }, [onCloseContextMenu]);

  const onMoveEnd = useCallback(
    (event: unknown, viewport: { x: number; y: number; zoom: number }) => {
      // 用户交互停止时吸附视口坐标到物理整数像素，消除次像素抗锯齿导致的边框与字体模糊发虚
      if (event) {
        const rx = Math.round(viewport.x);
        const ry = Math.round(viewport.y);
        if (Math.abs(rx - viewport.x) > 0.01 || Math.abs(ry - viewport.y) > 0.01) {
          setViewport({ x: rx, y: ry, zoom: viewport.zoom }, { duration: 0 });
        }
      }
    },
    [setViewport],
  );

  const onPaneContextMenu = useCallback(
    (event: React.MouseEvent | MouseEvent) => {
      event.preventDefault();
      const x = 'clientX' in event ? event.clientX : 0;
      const y = 'clientY' in event ? event.clientY : 0;

      // 如果当前已有多个节点被选中，右键空白处优先弹出多选操作菜单（而非添加节点面板）
      const selectedNodes = getNodes().filter((n) => n.selected);
      if (selectedNodes.length > 1 && onSelectionContextMenu) {
        onSelectionContextMenu(event as React.MouseEvent, selectedNodes);
        return;
      }

      onCloseContextMenu?.();
      useUIStore.getState().openNodePalette({ x, y });
    },
    [onCloseContextMenu, onSelectionContextMenu, getNodes],
  );

  const handleNodesChange = useCallback(
    (changes: NodeChange[]) => {
      const nonRemoveChanges = changes.filter((c) => c.type !== 'remove');
      if (nonRemoveChanges.length > 0) {
        onNodesChange(nonRemoveChanges);
      }
    },
    [onNodesChange],
  );

  const handleEdgesChange = useCallback(
    (changes: EdgeChange[]) => {
      const removeChanges = changes.filter((c) => c.type === 'remove');
      const otherChanges = changes.filter((c) => c.type !== 'remove');

      if (otherChanges.length > 0) {
        onEdgesChange(otherChanges);
      }

      if (removeChanges.length > 0) {
        for (const change of removeChanges) {
          removeConnection(change.id);
        }
      }
    },
    [onEdgesChange, removeConnection],
  );

  const handleAutoLayout = useCallback(
    (direction: LayoutDirection) => {
      if (!flow) return;

      const stageGraph = getStageGraph(flow, activeStage);
      const positions = getLayoutedPositions(stageGraph.nodes, stageGraph.connections, direction);
      batchUpdateNodePositions(positions);

      setTimeout(() => {
        fitView({ padding: 0.4, duration: 300 });
      }, 150);
    },
    [flow, activeStage, batchUpdateNodePositions, fitView],
  );

  const handleAdaptiveLayout = useCallback(() => {
    if (!flow) return;

    const stageGraph = getStageGraph(flow, activeStage);
    const containerWidth = containerRef.current?.clientWidth ?? 1200;
    const positions = getAdaptiveLayoutedPositions(stageGraph.nodes, stageGraph.connections, {
      viewportWidth: containerWidth,
    });
    batchUpdateNodePositions(positions);

    setTimeout(() => {
      fitView({ padding: 0.2, duration: 300 });
    }, 150);
  }, [flow, activeStage, batchUpdateNodePositions, fitView, containerRef]);

  // memo 化 children，使 GraphView / FlowRenderer 在拖拽帧整体短路
  const canvasChrome = useMemo(
    () => (
      <>
        <Background
          variant={BackgroundVariant.Dots}
          gap={20}
          size={1}
          color="var(--canvas-dot)"
        />
        <Controls
          className="!bg-background !border-border"
          showZoom
          showFitView
          showInteractive
          fitViewOptions={FIT_VIEW_OPTIONS}
        >
          {/* 小地图开关 */}
          <ControlButton
            onClick={toggleMinimap}
            title={showMinimap ? ll.flow.close_minimap() : ll.flow.open_minimap()}
          >
            <MapIcon className="w-4 h-4" />
          </ControlButton>
          {/* 水平自动布局 */}
          <ControlButton
            onClick={() => handleAutoLayout('LR')}
            title={ll.flow.auto_layout_horizontal()}
          >
            <ArrowRightLeft className="w-4 h-4" />
          </ControlButton>
          {/* 可视画布自适应折行布局 */}
          <ControlButton
            onClick={handleAdaptiveLayout}
            title={ll.flow.auto_layout_adaptive()}
          >
            <WrapText className="w-4 h-4" />
          </ControlButton>
          {/* 搜索流程 */}
          <ControlButton
            onClick={() => useUIStore.getState().openSearchOverlay()}
            title={`${getLL().flow.search_flow()} (Ctrl+Shift+F)`}
          >
            <Search className="w-4 h-4" />
          </ControlButton>
        </Controls>
        {showMinimap && (
          <MiniMap
            nodeColor={minimapNodeColor}
            maskColor={resolvedTheme === 'dark' ? 'rgba(255, 255, 255, 0.12)' : 'rgba(0, 0, 0, 0.08)'}
            className="!bg-background !border-border dark:!bg-neutral-900"
            pannable
            zoomable
          />
        )}
      </>
    ),
    [
      showMinimap,
      toggleMinimap,
      handleAutoLayout,
      handleAdaptiveLayout,
      minimapNodeColor,
      resolvedTheme,
      locale,
      ll,
    ],
  );

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      onNodesChange={handleNodesChange}
      onEdgesChange={handleEdgesChange}
      onConnect={onConnect}
      onNodeDragStop={onNodeDragStop}
      onSelectionDragStop={onSelectionDragStop}
      onNodeClick={onNodeClick}
      onNodeContextMenu={onNodeContextMenu}
      onSelectionContextMenu={onSelectionContextMenu}
      onEdgeClick={onEdgeClick}
      onDragOver={onDragOver}
      onDrop={onDrop}
      onPaneClick={onPaneClick}
      onPaneContextMenu={onPaneContextMenu}
      onMove={onMove}
      onMoveEnd={onMoveEnd}
      nodeTypes={nodeTypes}
      edgeTypes={edgeTypes}
      deleteKeyCode={DELETE_KEY_CODES}
      fitViewOptions={FIT_VIEW_OPTIONS}
      minZoom={0.1}
      maxZoom={1.35}
      selectionMode={SelectionMode.Partial}
      proOptions={PRO_OPTIONS}
      colorMode={resolvedTheme === 'dark' ? 'dark' : 'light'}
      elevateNodesOnSelect={true}
      autoPanOnNodeDrag={true}
      autoPanOnConnect={true}
      connectionRadius={30}
      nodesDraggable={!isExecuting}
      nodesConnectable={!isExecuting}
      isValidConnection={isValidConnectionProp}
      defaultEdgeOptions={DEFAULT_EDGE_OPTIONS}
    >
      {canvasChrome}
      <SelectionAlignmentBar selectedNodes={selectedNodes} />
    </ReactFlow>
  );
});
