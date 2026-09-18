/**
 * FlowSurface - 画布核心渲染表面
 * 承载 ReactFlow 实例、节点/边状态、视口交互及画布内部装饰层 (Background/Controls/MiniMap/ExecutionOverlay)。
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
import { ExecutionOverlay } from './ExecutionOverlay';
import {
  useFlowStore,
  useUIStore,
  useDebugStore,
  useExecutionStore,
  selectIsExecuting,
  selectFlow,
} from '@/stores';
import { useTheme } from '@/hooks/useTheme';
import { useCurrentLocale } from '@/hooks/useLocale';
import { MapIcon, ArrowRightLeft, WrapText, Search } from 'lucide-react';
import type { Flow } from '@/types';
import { getNodeMetadata } from '@/lib/NodeRegistry';
import { getLayoutedPositions, getAdaptiveLayoutedPositions, type LayoutDirection } from './layout';
import {
  convertFlowToReactFlow,
  convertEdgeToConnection,
  isValidConnection,
  type FlowNodeData,
} from './utils';
import { getLL } from '@/i18n/i18n-lazy';
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
}

export const FlowSurface = memo(function FlowSurface({
  containerRef,
  onDragOver,
  onDrop,
  onNodeContextMenu,
}: FlowSurfaceProps) {
  const flow = useFlowStore(selectFlow);
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

  const { fitView, setCenter, getNode } = useReactFlow();

  // 转换 Flow 数据为 React Flow 初始格式
  const initialData = useMemo(() => {
    if (!flow) return { nodes: [], edges: [] };
    return convertFlowToReactFlow(flow);
  }, [flow]);

  // 节点和边状态（下沉到 FlowSurface，拖拽帧仅触发 FlowSurface 重渲染）
  const [nodes, setNodes, onNodesChange] = useNodesState(initialData.nodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialData.edges);

  // 动态节点元数据颜色 Map（O(1) 查找）
  const [nodeColorMap, setNodeColorMap] = useState<Map<string, string>>(() => new Map());

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
  const prevConnectionsRef = useRef<Flow['connections'] | null>(null);
  const nodesLengthRef = useRef(initialData.nodes.length);

  useEffect(() => {
    if (!flow) return;
    const { nodes: newNodes, edges: newEdges } = convertFlowToReactFlow(flow);

    const isSwitchingFlow = flow.uuid !== prevFlowUuidRef.current;
    prevFlowUuidRef.current = flow.uuid;
    const connectionsChanged = prevConnectionsRef.current !== flow.connections;
    prevConnectionsRef.current = flow.connections;

    if (isSwitchingFlow) {
      nodesLengthRef.current = newNodes.length;
      setNodes(newNodes.map((n) => ({ ...n, selected: n.id === selectedNodeId })));
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
            prevData.label === nextData.label && prevData.config === nextData.config;
          const positionUnchanged =
            prevNode.position.x === newNode.position.x &&
            prevNode.position.y === newNode.position.y;
          const selected = prevNode.id === selectedNodeId;
          if (dataUnchanged && positionUnchanged && prevNode.selected === selected) {
            return prevNode; // 保持对象身份 → adoptUserNodes 走 checkEquality 短路
          }
          changed = true;
          return {
            ...prevNode,
            position: positionUnchanged ? prevNode.position : newNode.position,
            data: dataUnchanged ? prevNode.data : newNode.data,
            selected,
          };
        });
        return changed ? next : prevNodes;
      });
    }

    if (connectionsChanged) setEdges(newEdges);
  }, [flow, selectedNodeId, setNodes, setEdges]);

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
  const overlayInnerRef = useRef<HTMLDivElement>(null);

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
  const connectionsRef = useRef<Flow['connections']>(flow?.connections ?? []);
  connectionsRef.current = flow?.connections ?? [];

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
      if (
        !isValidConnection(
          connection.source || '',
          connection.target || '',
          connection.sourceHandle || 'out',
          connection.targetHandle || 'in',
          connectionsRef.current,
        )
      ) {
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

  const onNodeDragStop = useCallback(
    (_: unknown, node: Node) => {
      updateNode(node.id, {
        position: { x: node.position.x, y: node.position.y },
      });
    },
    [updateNode],
  );

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      selectNode(node.id);
      setConfigPanelOpen(true);
      setConfigPanelMode('node');
      useUIStore.getState().closeNodePalette();
      const debugStore = useDebugStore.getState();
      if (debugStore.status !== 'idle' && debugStore.status !== 'ended') {
        setFlowListPanelOpen(true);
      }
    },
    [selectNode, setConfigPanelOpen, setConfigPanelMode, setFlowListPanelOpen],
  );

  const onEdgeClick = useCallback(() => {
    selectNode(null);
    setConfigPanelOpen(false);
  }, [selectNode, setConfigPanelOpen]);

  const onPaneClick = useCallback(() => {
    selectNode(null);
    setConfigPanelMode('node');
    useUIStore.getState().closeNodePalette();
  }, [selectNode, setConfigPanelMode]);

  const onMove = useCallback((_event: unknown, viewport: { x: number; y: number; zoom: number }) => {
    const el = overlayInnerRef.current;
    if (el) {
      el.style.transform = `translate(${viewport.x}px, ${viewport.y}px) scale(${viewport.zoom})`;
    }
  }, []);

  const onPaneContextMenu = useCallback(
    (event: React.MouseEvent | MouseEvent) => {
      event.preventDefault();
      const x = 'clientX' in event ? event.clientX : 0;
      const y = 'clientY' in event ? event.clientY : 0;
      useUIStore.getState().openNodePalette({ x, y });
    },
    [],
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

      const positions = getLayoutedPositions(flow.nodes, flow.connections, direction);
      batchUpdateNodePositions(positions);

      setTimeout(() => {
        fitView({ padding: 0.4, duration: 300 });
      }, 150);
    },
    [flow, batchUpdateNodePositions, fitView],
  );

  const handleAdaptiveLayout = useCallback(() => {
    if (!flow) return;

    const containerWidth = containerRef.current?.clientWidth ?? 1200;
    const positions = getAdaptiveLayoutedPositions(flow.nodes, flow.connections, {
      viewportWidth: containerWidth,
    });
    batchUpdateNodePositions(positions);

    setTimeout(() => {
      fitView({ padding: 0.2, duration: 300 });
    }, 150);
  }, [flow, batchUpdateNodePositions, fitView, containerRef]);

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
        {/* 执行状态叠加层 — 独立渲染，执行期间不触发 React Flow reconcile */}
        <div className="absolute inset-0 pointer-events-none overflow-hidden" style={{ zIndex: 5 }}>
          <div
            ref={overlayInnerRef}
            className="absolute left-0 top-0"
            style={{
              transformOrigin: '0 0',
              willChange: 'transform',
            }}
          >
            <ExecutionOverlay />
          </div>
        </div>
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
      onNodeClick={onNodeClick}
      onNodeContextMenu={onNodeContextMenu}
      onEdgeClick={onEdgeClick}
      onDragOver={onDragOver}
      onDrop={onDrop}
      onPaneClick={onPaneClick}
      onPaneContextMenu={onPaneContextMenu}
      onMove={onMove}
      nodeTypes={nodeTypes}
      edgeTypes={edgeTypes}
      deleteKeyCode={DELETE_KEY_CODES}
      fitViewOptions={FIT_VIEW_OPTIONS}
      minZoom={0.1}
      maxZoom={2}
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
    </ReactFlow>
  );
});
