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

import { EntryNode, HttpNode, ConditionalNode, SubFlowNode, WsConnectNode, WsSendCollectNode, WsCloseNode, AssertionNode, ScriptNode, TcpConnectNode, TcpSendNode, TcpReceiveNode, TcpCloseNode, UdpBindNode, UdpSendToNode, UdpRecvFromNode, UdpCloseNode, VariableExtractorNode, SseConnectNode, SseListenNode, SseDisconnectNode, GraphqlConnectNode, GraphqlQueryNode, GraphqlSubscribeNode, GraphqlIntrospectNode, GraphqlDisconnectNode, ProxyNode, EndNode } from './nodes';
import DynamicPluginNode from './nodes/DynamicPluginNode';
import { ExecutionOverlay } from './ExecutionOverlay';
import { CanvasToolbar } from './CanvasToolbar';
import { DebugBreadcrumbs } from './DebugBreadcrumbs';
import { NodePalette } from './NodePalette';
import { NodeContextMenu } from './NodeContextMenu';
import { CustomEdge } from './edges/CustomEdge';
import { useFlowStore, useUIStore, useDebugStore, useExecutionStore, useFlowListStore, selectIsExecuting, selectFlow, selectIsDirty, connectionFor } from '@/stores';
import { useTheme } from '@/hooks/useTheme';
import { MapIcon, ArrowRightLeft, WrapText, Search } from 'lucide-react';
import type { NodeData, NodeType } from '@/types';
import type { NodeMetadata } from '@/types/nodeRegistry';
import { getNodeMetadata } from '@/lib/NodeRegistry';
import { datasourceGetTemplates } from '@/services/tauri/datasource';
import { getLayoutedPositions, getAdaptiveLayoutedPositions, type LayoutDirection } from './layout';
import {
  convertFlowToReactFlow,
  convertEdgeToConnection,
  isValidConnection,
  createDefaultNodeData,
  createDefaultNodeName,
} from './utils';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import { flowStorage } from '@/services/storage';
import { inferDefaults } from '@/lib/nodeDefaults';
import { toast } from 'sonner';

/** 节点配置剪贴板 key */
const NODE_CONFIG_CLIPBOARD_KEY = 'node-config-clipboard';

/** 节点配置剪贴板数据类型 */
interface NodeConfigClipboard {
  nodeType: string;
  data: Record<string, unknown>;
  copiedAt: number;
  sourceNodeName: string;
}

/** 敏感字段列表，复制节点配置时需要脱敏 */
const SENSITIVE_FIELDS: readonly string[] = [
  'password', 'token', 'authorization', 'secret', 'api_key',
  'private_key', 'certificate', 'jwt',
];

/**
 * 脱敏节点数据：将敏感字段的值替换为占位符
 */
function sanitizeNodeData(data: Record<string, unknown>): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(data)) {
    if (SENSITIVE_FIELDS.includes(key)) {
      result[key] = '***redacted***';
    } else if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
      result[key] = sanitizeNodeData(value as Record<string, unknown>);
    } else {
      result[key] = value;
    }
  }
  return result;
}

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

/** 画布全局样式字符串（静态，避免每次渲染重新创建 <style> 标签） */
const CANVAS_STYLE_STRING = `
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
  .react-flow__controls-button svg {
    fill: var(--controls-icon) !important;
    stroke: var(--controls-icon) !important;
  }
  .react-flow__controls-button:hover svg {
    fill: var(--foreground) !important;
    stroke: var(--foreground) !important;
  }
  .react-flow__controls-button {
    background-color: var(--background) !important;
    border-color: var(--border) !important;
  }
  .react-flow__controls-button:hover {
    background-color: var(--accent) !important;
  }
  .react-flow__controls,
  .react-flow__minimap {
    border-width: 1px !important;
    border-style: solid !important;
    border-color: var(--border) !important;
  }
  .dark .react-flow__controls,
  .dark .react-flow__minimap {
    border-color: rgba(255, 255, 255, 0.15) !important;
  }
  .dark .react-flow__controls-button {
    border-color: rgba(255, 255, 255, 0.1) !important;
  }
  .react-flow__node.dragging,
  .react-flow__node.dragging * {
    cursor: url("${GRABBING_CURSOR_SVG}") 8 8, grabbing !important;
  }
  .react-flow__node {
    /* contain: paint 会裁剪溢出内容（包括节点出入口圆圈），所以只保留 layout style */
    contain: layout style;
  }
  /* 插件节点走 React Flow 的 default fallback，wrapper 会套上库的默认节点样式
     （白底/边框/150px 宽/padding），与 DynamicPluginNode 自绘外观冲突产生双重边框。
     清零 wrapper 样式，让节点完全由自定义组件呈现。 */
  .react-flow__node-default {
    background: transparent !important;
    border: none !important;
    border-radius: 0 !important;
    box-shadow: none !important;
    padding: 0 !important;
    width: auto !important;
    font-size: inherit !important;
    text-align: inherit !important;
  }
`;

/** 节点类型映射 */
const nodeTypes = {
  entry: EntryNode,
  http: HttpNode,
  conditional: ConditionalNode,
  ws_connect: WsConnectNode,
  ws_send_collect: WsSendCollectNode,
  ws_close: WsCloseNode,
  assertion: AssertionNode,
  script: ScriptNode,
  tcp_connect: TcpConnectNode,
  tcp_send: TcpSendNode,
  tcp_receive: TcpReceiveNode,
  tcp_close: TcpCloseNode,
  udp_bind: UdpBindNode,
  udp_send_to: UdpSendToNode,
  udp_recv_from: UdpRecvFromNode,
  udp_close: UdpCloseNode,
  variable_extractor: VariableExtractorNode,
  sse_connect: SseConnectNode,
  sse_listen: SseListenNode,
  sse_disconnect: SseDisconnectNode,
  graphql_connect: GraphqlConnectNode,
  graphql_query: GraphqlQueryNode,
  graphql_subscribe: GraphqlSubscribeNode,
  graphql_introspect: GraphqlIntrospectNode,
  graphql_disconnect: GraphqlDisconnectNode,
  proxy: ProxyNode,
  end: EndNode,
  sub_flow: SubFlowNode,
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
  script: '#6366f1',
  sub_flow: '#8b5cf6',
  tcp_connect: '#10b981',
  tcp_send: '#10b981',
  tcp_receive: '#10b981',
  tcp_close: '#10b981',
  udp_bind: '#f97316',
  udp_send_to: '#f97316',
  udp_recv_from: '#f97316',
  udp_close: '#f97316',
  sse_connect: '#f43f5e',
  sse_listen: '#f43f5e',
  sse_disconnect: '#f43f5e',
  graphql_connect: '#6366f1',
  graphql_query: '#6366f1',
  graphql_subscribe: '#6366f1',
  graphql_introspect: '#6366f1',
  graphql_disconnect: '#6366f1',
  proxy: '#84cc16',
  end: '#8B5CF6',
};

/**
 * ImportFlowCard - 导入流程文件卡片（拖放 + 点击选择文件）
 */
function ImportFlowCard({ wc, onImport }: { wc: ReturnType<typeof getLL>['welcome']; onImport: (flow: import('@/types/flow').Flow) => void }) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [isDragOver, setIsDragOver] = useState(false);

  const handleFile = useCallback((file: File) => {
    if (!file.name.endsWith('.mpf') && !file.name.endsWith('.json')) return;
    const reader = new FileReader();
    reader.onload = (e) => {
      try {
        const text = e.target?.result as string;
        const data = JSON.parse(text);
        // 支持两种格式：直接 Flow 对象和 FlowFile 包装格式
        const flowData = data.flow ?? data;
        if (flowData && flowData.uuid && flowData.nodes) {
          onImport(flowData as import('@/types/flow').Flow);
        }
      } catch (err) {
        console.error('[ImportFlow] 解析流程文件失败:', err);
      }
    };
    reader.readAsText(file);
  }, [onImport]);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);
    const file = e.dataTransfer.files[0];
    if (file) handleFile(file);
  }, [handleFile]);

  return (
    <>
      <input
        ref={fileInputRef}
        type="file"
        accept=".mpf,.json"
        className="hidden"
        onChange={(e) => {
          const file = e.target.files?.[0];
          if (file) handleFile(file);
          e.target.value = '';
        }}
      />
      <div
        onClick={() => fileInputRef.current?.click()}
        onDragOver={(e) => { e.preventDefault(); setIsDragOver(true); }}
        onDragLeave={() => setIsDragOver(false)}
        onDrop={handleDrop}
        className={cn(
          'flex flex-col items-center gap-2 p-5 rounded-xl border transition-all cursor-pointer w-36 group',
          isDragOver
            ? 'border-primary bg-primary/5'
            : 'border-dashed border-border hover:border-primary/50 hover:bg-accent/50'
        )}
      >
        <div className="w-10 h-10 rounded-full bg-primary/10 flex items-center justify-center group-hover:bg-primary/20 transition-colors">
          <svg className="w-5 h-5 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
          </svg>
        </div>
        <span className="text-sm font-medium text-foreground">{wc.empty_import_flow()}</span>
        <span className="text-xs text-muted-foreground">{wc.empty_import_flow_desc()}</span>
      </div>
    </>
  );
}

/**
 * 画布内部组件（在 ReactFlowProvider 内部）
 */
function FlowCanvasInner() {
  const flow = useFlowStore(selectFlow);
  const addConnection = useFlowStore((s) => s.addConnection);
  const updateNode = useFlowStore((s) => s.updateNode);
  const selectNode = useFlowStore((s) => s.selectNode);
  const addNode = useFlowStore((s) => s.addNode);
  const removeConnection = useFlowStore((s) => s.removeConnection);
  const pendingAddNodeType = useFlowStore((s) => s.pendingAddNodeType);
  const clearPendingAddNodeType = useFlowStore((s) => s.clearPendingAddNodeType);
  const isDirty = useFlowStore(selectIsDirty);
  const markSaved = useFlowStore((s) => s.markSaved);
  const focusNodeUuid = useFlowStore((s) => s.focusNodeUuid);
  const clearFocusNode = useFlowStore((s) => s.clearFocusNode);
  const centerOnNodeUuid = useFlowStore((s) => s.centerOnNodeUuid);
  const clearCenterOnNode = useFlowStore((s) => s.clearCenterOnNode);
  const batchUpdateNodePositions = useFlowStore((s) => s.batchUpdateNodePositions);
  const setConfigPanelOpen = useUIStore((s) => s.setConfigPanelOpen);
  const setConfigPanelMode = useUIStore((s) => s.setConfigPanelMode);
  const setFlowListPanelOpen = useUIStore((s) => s.setFlowListPanelOpen);
  const showMinimap = useUIStore((s) => s.showMinimap);
  const toggleMinimap = useUIStore((s) => s.toggleMinimap);
  const { resolvedTheme } = useTheme();
  const ll = getLL();
  const wc = ll.welcome;

  // 执行状态（用于控制画布交互）
  const isExecuting = useExecutionStore(selectIsExecuting);

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

  // ===== 执行状态跟踪（非响应式，通过 ref + subscribe 避免画布整体重渲染） =====
  const execStateRef = useRef({ currentNodeUuid: null as string | null, nodeReportsMap: {} as Record<string, import('@/types/report').NodeExecutionReport>, isExecuting: false });
  const classNameUpdateRafRef = useRef<number | null>(null);
  // 执行结束后只做一次 className 回写，避免执行期间反复 setNodes
  const finalClassNameDoneRef = useRef(false);
  // 叠加层视口变换 ref — 通过 onMove 直接写 DOM，不触发 React 渲染
  const overlayInnerRef = useRef<HTMLDivElement>(null);

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
  const isRenameCancellingRef = useRef(false);

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

  // 执行节点高亮 - 通过 Zustand subscribe 直接订阅（不触发 React 重渲染）
  // 避免每次 nodeStarted/nodeCompleted 时 FlowCanvasInner 整体重渲染
  useEffect(() => {
    // 初始化 ref
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

      // === 极致性能优化 ===
      // 执行期间：不调用 setNodes，节点视觉状态由 ExecutionOverlay 叠加层提供
      // 执行结束后：一次 setNodes 写回最终 className，使节点呈现绿/红边框
      if (isExecuting) return;

      // 执行已结束，仅做一次 className 写回
      if (finalClassNameDoneRef.current) return;
      finalClassNameDoneRef.current = true;

      setNodes(nds => {
        let changed = false;
        const updated = nds.map(node => {
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
      // 新执行开始时重置「最终 className 写回」标记
      if (s.isExecuting) {
        finalClassNameDoneRef.current = false;
      }
      // RAF 节流：合并多次连续事件的 setNodes 调用到下一帧，
      // 避免每次 nodeStarted/nodeCompleted 都触发全量 reconcile
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
  }, []);

  // 获取当前流程连接列表（用于连接验证）
  const currentConnections = flow?.connections ?? [];

  // 处理连接创建
  const onConnect = useCallback(
    (connection: Connection) => {
      // 验证连接（传入当前连接列表进行完整校验）
      if (
        !isValidConnection(
          connection.source || '',
          connection.target || '',
          connection.sourceHandle || 'out',
          connection.targetHandle || 'in',
          currentConnections
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
    [setEdges, addConnection, currentConnections]
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

  // 处理节点点击 - 选中节点并展开面板，同时关闭节点选择器
  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      selectNode(node.id);
      setConfigPanelOpen(true);
      setConfigPanelMode('node');
      useUIStore.getState().closeNodePalette();
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

  // 处理画布空白处点击 - 取消选中，切回节点列表面板
  // 不自动展开配置面板：如果面板已打开，取消选中后会显示节点列表；
  // 如果面板已关闭，保持关闭。用户可点击右侧抽屉图标手动打开。
  const onPaneClick = useCallback(() => {
    selectNode(null);
    setContextMenu(null);
    setConfigPanelMode('node');
    useUIStore.getState().closeNodePalette();
  }, [selectNode, setConfigPanelMode]);

  // 视口变换回调 — 同步到叠加层容器，直接写 DOM style.transform，不触发 React 渲染
  const onMove = useCallback((_event: unknown, viewport: { x: number; y: number; zoom: number }) => {
    const el = overlayInnerRef.current;
    if (el) {
      el.style.transform = `translate(${viewport.x}px, ${viewport.y}px) scale(${viewport.zoom})`;
    }
  }, []);

  // 处理画布右键 - 弹出节点选择器
  const onPaneContextMenu = useCallback(
    (event: React.MouseEvent | MouseEvent) => {
      event.preventDefault();
      // 使用 clientX/Y 获取相对于视口的坐标用于 fixed 定位
      const x = 'clientX' in event ? event.clientX : 0;
      const y = 'clientY' in event ? event.clientY : 0;
      useUIStore.getState().openNodePalette({ x, y });
    },
    []
  );

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

  // 可视画布自适应折行自动布局处理
  const handleAdaptiveLayout = useCallback(() => {
    if (!flow) return;

    const containerWidth = canvasRef.current?.clientWidth ?? 1200;
    const positions = getAdaptiveLayoutedPositions(flow.nodes, flow.connections, {
      viewportWidth: containerWidth,
    });
    batchUpdateNodePositions(positions);

    // 延迟 fitView，自适应折行布局整体宽高比均衡，padding 设为 0.2 更充盈饱满
    setTimeout(() => {
      fitView({ padding: 0.2, duration: 300 });
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
        name: `${sourceNode.name} (${getLL().flow.duplicate_suffix()})`,
        position: {
          x: sourceNode.position.x + offset,
          y: sourceNode.position.y + offset,
        },
        data: JSON.parse(JSON.stringify(sourceNode.data)),
      });
    },
    []
  );

  // 复制节点配置到剪贴板
  const handleCopyNodeConfig = useCallback(
    (nodeId: string) => {
      const { flow: currentFlow } = useFlowStore.getState();
      if (!currentFlow) return;

      const sourceNode = currentFlow.nodes.find((n) => n.uuid === nodeId);
      if (!sourceNode) return;

      const clipData: NodeConfigClipboard = {
        nodeType: sourceNode.type,
        data: sanitizeNodeData(JSON.parse(JSON.stringify(sourceNode.data))),
        copiedAt: Date.now(),
        sourceNodeName: sourceNode.name,
      };

      try {
        localStorage.setItem(NODE_CONFIG_CLIPBOARD_KEY, JSON.stringify(clipData));
      } catch {
        console.warn('Failed to save node config to localStorage');
        toast.error(getLL().errors.clipboard_save_failed());
      }
    },
    []
  );

  // 从剪贴板粘贴节点配置
  const handlePasteNodeConfig = useCallback(
    (nodeId: string) => {
      const { flow: currentFlow, updateNode: modifyNode } = useFlowStore.getState();
      if (!currentFlow) return;

      const targetNode = currentFlow.nodes.find((n) => n.uuid === nodeId);
      if (!targetNode) return;

      let clipData: NodeConfigClipboard | null = null;
      try {
        const raw = localStorage.getItem(NODE_CONFIG_CLIPBOARD_KEY);
        if (raw) clipData = JSON.parse(raw);
      } catch {
        return;
      }

      if (!clipData) return;
      if (clipData.nodeType !== targetNode.type) return;

      modifyNode(nodeId, { data: JSON.parse(JSON.stringify(clipData.data)) });
    },
    []
  );

  /** 检查是否有可粘贴的兼容配置 */
  const hasClipboardConfigFor = useCallback((nodeType: string): boolean => {
    try {
      const raw = localStorage.getItem(NODE_CONFIG_CLIPBOARD_KEY);
      if (!raw) return false;
      const clipData: NodeConfigClipboard = JSON.parse(raw);
      return clipData.nodeType === nodeType && Date.now() - clipData.copiedAt < 86400000; // 24h 过期
    } catch {
      return false;
    }
  }, []);

  // 删除节点（通过确认弹窗）
  const handleDeleteNode = useCallback(
    (nodeId: string) => {
      const { flow: currentFlow, removeNode: removeNodeById, selectNode: deselectNode } = useFlowStore.getState();
      if (!currentFlow) return;

      const node = currentFlow.nodes.find((n) => n.uuid === nodeId);
      if (!node || node.type === 'entry') return;

      const LL = getLL();
      const { showConfirm } = useUIStore.getState();
      showConfirm({
        title: LL.flow.delete_node_title(),
        description: LL.flow.delete_node_desc({ name: node.name }),
        confirmText: LL.flow.delete_node_confirm(),
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

      isRenameCancellingRef.current = false;
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
    if (isRenameCancellingRef.current) {
      isRenameCancellingRef.current = false;
      return;
    }
    if (renamingNodeId && renamingValue.trim()) {
      updateNode(renamingNodeId, { name: renamingValue.trim() });
    }
    setRenamingNodeId(null);
    setRenamingValue('');
  }, [renamingNodeId, renamingValue, updateNode]);

  // 重命名取消
  const handleRenameCancel = useCallback(() => {
    isRenameCancellingRef.current = true;
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
  const onDrop = useCallback(
    async (event: React.DragEvent) => {
      event.preventDefault();

      // 数据源条目拖拽：经 get_templates 转成节点模板落画布
      const dsPayload = event.dataTransfer.getData('application/reactflow/datasource');
      if (dsPayload) {
        if (!flow) return;
        let parsed: { datasource_id: string; connection_id?: string; item_ids: string[] };
        try {
          parsed = JSON.parse(dsPayload);
        } catch {
          toast.error(ll.datasource.drag_payload_invalid());
          return;
        }
        const conn = parsed.connection_id
          ? connectionFor(parsed.connection_id)
          : connectionFor(parsed.datasource_id);
        if (!conn) {
          toast.error(ll.datasource.auth_failed());
          return;
        }
        try {
          const templates = await datasourceGetTemplates(
            parsed.datasource_id,
            parsed.item_ids,
            conn.config
          );
          const position = screenToFlowPosition({
            x: event.clientX,
            y: event.clientY,
          });
          templates.forEach((tpl, i) => {
            const defaultData = createDefaultNodeData(tpl.type_id as NodeType);
            const nodeData = {
              ...defaultData,
              ...tpl.data,
              type: tpl.type_id,
            };
            const nodeCount = flow.nodes.length + i + 1;
            const name = tpl.name || createDefaultNodeName(tpl.type_id as NodeType, nodeCount);
            addNode({
              uuid: crypto.randomUUID(),
              type: tpl.type_id as NodeType,
              name,
              position: { x: position.x + i * 40, y: position.y + i * 40 },
              data: nodeData as unknown as NodeData,
            });
          });
        } catch (e) {
          toast.error(e instanceof Error ? e.message : String(e));
        }
        return;
      }

      const nodeType = event.dataTransfer.getData('application/reactflow/nodeType');
      if (!nodeType || !flow) return;

      // 使用鼠标实际位置转换为流程坐标
      const position = screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });

      // 获取上游节点（当前选中的节点作为上游推断上下文）
      const { getSelectedNode } = useFlowStore.getState();
      const selectedNode = getSelectedNode();
      const inferred = selectedNode
        ? inferDefaults(
            { id: selectedNode.uuid, type: selectedNode.type, data: selectedNode.data as NodeData & Record<string, unknown> },
            nodeType as NodeType
          )
        : {};

      // 根据节点类型创建节点配置
      const nodeCount = flow.nodes.length + 1;
      const newNodeUuid = crypto.randomUUID();
      const defaultData = createDefaultNodeData(nodeType);
      const data = { ...defaultData, ...inferred };
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

    // 若由右键在画布某处唤出，则放置在鼠标右键位置；否则放置在视口中心
    const palettePos = useUIStore.getState().nodePalettePosition;
    const targetScreenX = palettePos ? palettePos.x : centerX;
    const targetScreenY = palettePos ? palettePos.y : centerY;
    const position = screenToFlowPosition({ x: targetScreenX, y: targetScreenY });
    // 获取上游节点（当前选中的节点作为上游推断上下文）
    const { getSelectedNode } = useFlowStore.getState();
    const selectedNode = getSelectedNode();
    const inferred = selectedNode
      ? inferDefaults(
          { id: selectedNode.uuid, type: selectedNode.type, data: selectedNode.data as NodeData & Record<string, unknown> },
          pendingAddNodeType as NodeType
        )
      : {};

    // 根据节点类型创建节点
    const nodeCount = flow.nodes.length + 1;
    const newNodeUuid = crypto.randomUUID();
    const defaultData = createDefaultNodeData(pendingAddNodeType);
    const data = { ...defaultData, ...inferred };
    const name = createDefaultNodeName(pendingAddNodeType as NodeType, nodeCount);
    addNode({
      uuid: newNodeUuid,
      type: pendingAddNodeType as NodeType,
      name,
      position,
      data,
    });

    // 清除待添加节点状态，并关闭节点选择器
    clearPendingAddNodeType();
    useUIStore.getState().closeNodePalette();
  }, [pendingAddNodeType, flow, addNode, screenToFlowPosition, clearPendingAddNodeType, canvasRef]);

  // 监听 F 键适应画布
  useEffect(() => {
    const handler = () => {
      if (flow) {
        fitView({ padding: 0.4, duration: 300 });
      }
    };
    window.addEventListener('trigger-fit-view', handler);
    return () => window.removeEventListener('trigger-fit-view', handler);
  }, [flow, fitView]);

  // 监听 Ctrl+Enter 执行流程
  useEffect(() => {
    const handler = () => {
      // 触发主工具栏的执行按钮点击
      const executeBtn = document.querySelector('[data-execute-btn]') as HTMLButtonElement;
      executeBtn?.click();
    };
    window.addEventListener('trigger-execute-flow', handler);
    return () => window.removeEventListener('trigger-execute-flow', handler);
  }, []);

  // 没有流程时显示空状态
  if (!flow) {
    return (
      <div className="w-full h-full flex items-center justify-center bg-background" data-guide="canvas-area">
        <div className="text-center max-w-lg">
          <h2 className="text-xl font-semibold text-muted-foreground mb-6">
            {ll.flow.no_flow_title()}
          </h2>

          {/* Three action cards */}
          <div className="flex gap-4 justify-center mb-8">
            {/* New blank flow */}
            <button
              onClick={() => useUIStore.getState().openNewFlowDialog()}
              className="flex flex-col items-center gap-2 p-5 rounded-xl border border-border hover:border-primary/50 hover:bg-accent/50 transition-all cursor-pointer w-36 group"
            >
              <div className="w-10 h-10 rounded-full bg-primary/10 flex items-center justify-center group-hover:bg-primary/20 transition-colors">
                <svg className="w-5 h-5 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" />
                </svg>
              </div>
              <span className="text-sm font-medium text-foreground">{ll.flow.new_flow()}</span>
              <span className="text-xs text-muted-foreground">{wc.empty_new_flow_desc()}</span>
            </button>

            {/* From template */}
            <button
              onClick={() => useUIStore.getState().openNewFlowDialog('', true)}
              className="flex flex-col items-center gap-2 p-5 rounded-xl border border-border hover:border-primary/50 hover:bg-accent/50 transition-all cursor-pointer w-36 group"
            >
              <div className="w-10 h-10 rounded-full bg-primary/10 flex items-center justify-center group-hover:bg-primary/20 transition-colors">
                <svg className="w-5 h-5 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                </svg>
              </div>
              <span className="text-sm font-medium text-foreground">{wc.empty_from_template()}</span>
              <span className="text-xs text-muted-foreground">{wc.empty_from_template_desc()}</span>
            </button>

            {/* Import flow file - drag & drop and click to open */}
            <ImportFlowCard
              wc={wc}
              onImport={(importedFlow) => {
                // 设置导入的流程并持久化
                flowStorage.saveFlow(importedFlow).then(() => {
                  const { addFlow: addToList } = useFlowListStore.getState();
                  addToList({
                    uuid: importedFlow.uuid,
                    name: importedFlow.name,
                    updated_at: Date.now(),
                    folder_path: '',
                  });
                  useFlowStore.getState().setFlow(importedFlow);
                }).catch(console.error);
              }}
            />
          </div>

          {/* Keyboard shortcuts hint */}
          <div className="text-xs text-muted-foreground/60">
            <span className="inline-flex items-center gap-1">
              <kbd className="px-1.5 py-0.5 rounded border border-border bg-muted text-[10px] font-mono">{wc.empty_right_click_key()}</kbd>
              <span>{wc.empty_right_click_desc()}</span>
            </span>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div 
      ref={canvasRef}
      className="w-full h-full"
      style={{ backgroundColor: 'var(--canvas-bg)', ...CANVAS_CURSOR_STYLE }}
      data-guide="canvas-area"
    >
      {/* 全局样式（静态字符串，避免每次渲染重新创建 <style> 标签） */}
      <style>{CANVAS_STYLE_STRING}</style>

      {/* 画布悬浮工具栏 */}
      <CanvasToolbar />

      {/* 调试面包屑导航 */}
      <DebugBreadcrumbs />

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
        onPaneContextMenu={onPaneContextMenu}
        onMove={onMove}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        // Backspace 和 Delete 键触发删除操作
        // 节点删除：由 useKeyboardShortcuts 在 capture 阶段拦截并弹出确认弹窗
        // 边删除：由 handleEdgesChange 处理（仅未选中节点时生效）
        deleteKeyCode={['Backspace', 'Delete']}

        fitViewOptions={{ padding: 0.4 }}
        minZoom={0.1}
        maxZoom={2}
        onlyRenderVisibleElements={true}
        proOptions={{ hideAttribution: true }}
        // → React Flow v12 性能/交互 props
        // panOnDrag 默认左键(0)可拖拽平移，如需中/右键平移可改为 {[1, 2]}
        // selectionOnDrag 启用左键框选（与左键平移互斥，默认关闭）
        colorMode={resolvedTheme === 'dark' ? 'dark' : 'light'}
        elevateNodesOnSelect={true}
        autoPanOnNodeDrag={true}
        autoPanOnConnect={true}
        connectionRadius={30}
        // 执行期间禁止拖拽和连线，避免干扰
        nodesDraggable={!isExecuting}
        nodesConnectable={!isExecuting}
        // 拖拽连线时的实时校验：端口唯一性 + 无循环
        isValidConnection={(connection) =>
          isValidConnection(
            connection.source || '',
            connection.target || '',
            connection.sourceHandle || 'out',
            connection.targetHandle || 'in',
            currentConnections
          )
        }
        defaultEdgeOptions={{
          type: 'custom',
          // 自定义边已自带粒子动画（<animateMotion>），不需要 React Flow 内置动画
          animated: false,
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
          fitViewOptions={{ padding: 0.4 }}
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

        {/* 小地图 */}
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
          onCopyConfig={handleCopyNodeConfig}
          onPasteConfig={handlePasteNodeConfig}
          hasClipboardConfig={hasClipboardConfigFor(contextMenu.nodeType)}
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
            {getLL().flow.rename_hint()}
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
