/**
 * CanvasReportViewer - 基于 React Flow 的只读画布报告查看器
 *
 * 显示 FlowExecutionReport.flow_snapshot 中的流程拓扑，
 * 已执行节点高亮，未执行节点灰显。
 */
import { useState, useRef, useMemo, useEffect, useCallback, type ComponentType } from 'react';
import { getLL } from '@/i18n/i18n-lazy';
import {
  ReactFlow,
  ReactFlowProvider,
  Background,
  type Node as FlowNode,
  type Edge as FlowEdge,
  type NodeProps,
  BackgroundVariant,
  MarkerType,
  useReactFlow,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import {
  EntryNode,
  HttpNode,
  ConditionalNode,
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
  SubFlowNode,
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
  DynamicPluginNode,
} from '@/components/canvas/nodes';
import { convertFlowToReactFlow } from '@/components/canvas/utils';
import { NodeExecutionDetails } from '@/components/execution/NodeExecutionDetails';
import { cn } from '@/lib/utils';
import { X } from 'lucide-react';
import type { ExecutionStatus, FlowExecutionReport, NodeExecutionReport } from '@/types';

// ============================================================================
// 常量
// ============================================================================

/** 执行成功节点的蓝色发光 */
const SUCCESS_BOX_SHADOW = '0 0 0 2px #3b82f6, 0 0 12px rgba(59, 130, 246, 0.3)';
/** 执行失败节点的红色边框 */
const FAILED_BOX_SHADOW = '0 0 0 2px #ef4444, 0 0 8px rgba(239, 68, 68, 0.3)';

// ============================================================================
// 原始节点类型映射（与 FlowCanvas.tsx lines 86-177 保持一致）
// ============================================================================

const originalNodeTypes: Record<string, ComponentType<NodeProps>> = {
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
  'flow:sub': SubFlowNode,
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
  default: DynamicPluginNode,
};

// ============================================================================
// 获取节点执行状态
// ============================================================================

interface ExecStatusInfo {
  /** 执行状态（unexecuted 表示不在执行路径中） */
  status: ExecutionStatus | 'unexecuted';
  /** 执行顺序序号（仅 success 时有值） */
  order?: number;
  /** 对应的节点执行报告 */
  nodeReport?: NodeExecutionReport;
}

/**
 * 根据报告获取节点的执行状态信息
 */
function getExecStatus(nodeId: string, report: FlowExecutionReport): ExecStatusInfo {
  const isInPath = report.execution_path?.includes(nodeId);
  const nodeReport = report.node_reports.find((r) => r.node_uuid === nodeId);

  if (!nodeReport || !isInPath) {
    return { status: 'unexecuted' };
  }

  const order =
    nodeReport.status === 'success'
      ? (report.execution_path?.indexOf(nodeId) ?? -1) + 1
      : undefined;

  return {
    status: nodeReport.status,
    order: order && order > 0 ? order : undefined,
    nodeReport,
  };
}

/**
 * 判断边是否在执���路径上
 * 检查源节点的 used_port_id 是否与边的 sourceHandle 匹配
 */
function isEdgeExecuted(edge: FlowEdge, report: FlowExecutionReport): boolean {
  const sourceReport = report.node_reports.find((r) => r.node_uuid === edge.source);
  if (!sourceReport) return false;
  if (sourceReport.status !== 'success' && sourceReport.status !== 'failed') return false;

  // 如果源节点有 used_port_id，则只有匹配该端口的边才是执行路径
  if (sourceReport.used_port_id) {
    return sourceReport.used_port_id === edge.sourceHandle;
  }
  // 没有 used_port_id（如早期版本报告），默认所有出边都是执行路径
  return true;
}

// ============================================================================
// 节点包裹 HOC —— 添加执行状态徽章
// ============================================================================

/**
 * 为节点组件包裹执行状态 UI
 * - success：蓝色顺序徽章（右上角）
 * - failed：无额外徽章（红色边框+动画由样式层处理）
 * - skipped：黄色警告徽章（右上角）
 * - unexecuted：虚线边框叠加层 + "未执行"标签（底部居中）
 */
function withExecBadge(Component: ComponentType<NodeProps>): ComponentType<NodeProps> {
  function WrappedNode(props: NodeProps) {
    const d = props.data as Record<string, unknown>;
    const execStatus = d.__execStatus as string | undefined;
    const execOrder = d.__execOrder as number | undefined;

    return (
      <div className="relative">
        <Component {...props} />

        {/* success：执行顺序徽章 */}
        {execStatus === 'success' && execOrder != null && (
          <div
            className={cn(
              'absolute -top-2 -right-2 z-10',
              'w-5 h-5 rounded-full',
              'bg-blue-500 text-white',
              'text-[10px] font-bold',
              'flex items-center justify-center',
              'shadow-md',
            )}
          >
            {execOrder}
          </div>
        )}

        {/* skipped：黄色警告徽章 */}
        {execStatus === 'skipped' && (
          <div
            className={cn(
              'absolute -top-2 -right-2 z-10',
              'w-5 h-5 rounded-full',
              'bg-yellow-400 text-white',
              'text-[10px] font-bold',
              'flex items-center justify-center',
              'shadow-md',
            )}
            title={getLL().execution.canvasReport.skipped_tooltip()}
          >
            ⚠
          </div>
        )}

        {/* unexecuted：虚线边框 + 标签 */}
        {execStatus === 'unexecuted' && (
          <>
            <div
              className={cn(
                'absolute inset-0',
                'border-2 border-dashed border-gray-300',
                'rounded-xl pointer-events-none',
              )}
            />
            <div
              className={cn(
                'absolute -bottom-5 left-1/2 -translate-x-1/2',
                'text-[10px] text-gray-400 whitespace-nowrap',
                'pointer-events-none',
              )}
            >
              {getLL().execution.canvasReport.unexecuted_label()}
            </div>
          </>
        )}
      </div>
    );
  }
  return WrappedNode;
}

/**
 * 构建报告查看器的 nodeTypes 映射
 * 每个原始节点类型包裹 withExecBadge 以显示执行状态
 */
function buildReportNodeTypes(): Record<string, ComponentType<NodeProps>> {
  const types: Record<string, ComponentType<NodeProps>> = {};
  for (const [key, Component] of Object.entries(originalNodeTypes)) {
    types[key] = withExecBadge(Component);
  }
  return types;
}

const reportNodeTypes = buildReportNodeTypes();

// ============================================================================
// Props 类型
// ============================================================================

export interface CanvasReportViewerProps {
  /** 流程执行报告 */
  report: FlowExecutionReport;
  /** 当前选中节点 UUID */
  selectedNodeUuid: string | null;
  /** 节点选择回调 */
  onNodeSelect: (uuid: string | null) => void;
}

// ============================================================================
// 内部组件（在 ReactFlowProvider 内部）
// ============================================================================

function CanvasReportViewerInner({
  report,
  selectedNodeUuid,
  onNodeSelect,
}: CanvasReportViewerProps) {
  const { fitView, setCenter } = useReactFlow();
  const [panelWidth, setPanelWidth] = useState(500);
  const [isDragging, setIsDragging] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
  }, []);

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (containerRef.current) {
        const containerRect = containerRef.current.getBoundingClientRect();
        const maxWidth = Math.max(400, containerRect.width - 240);
        const newWidth = Math.max(340, Math.min(maxWidth, containerRect.right - e.clientX));
        setPanelWidth(newWidth);
      }
    };

    const handleMouseUp = () => {
      setIsDragging(false);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging]);
  // 将 flow_snapshot 转换为增强的 React Flow 节点和边
  const { nodes, edges } = useMemo(() => {
    if (!report.flow_snapshot) return { nodes: [], edges: [] };

    const { nodes: rfNodes, edges: rfEdges } = convertFlowToReactFlow(report.flow_snapshot);

    // 增强节点：添加执行状态信息和样式
    const enhancedNodes: FlowNode[] = rfNodes.map((node) => {
      const { status, order } = getExecStatus(node.id, report);

      const classNames: string[] = [];
      if (status === 'unexecuted') {
        classNames.push('opacity-40', 'grayscale', 'pointer-events-none');
      }
      if (status === 'failed') {
        classNames.push('animate-pulse');
      }

      let style: React.CSSProperties | undefined;
      if (status === 'success') {
        style = { boxShadow: SUCCESS_BOX_SHADOW };
      } else if (status === 'failed') {
        style = { boxShadow: FAILED_BOX_SHADOW };
      }

      return {
        ...node,
        data: {
          ...node.data,
          __execStatus: status,
          __execOrder: order,
        },
        className: classNames.length > 0 ? classNames.join(' ') : undefined,
        style,
      };
    });

    // 增强边：执行路径蓝色实线，未执行灰色虚线
    const enhancedEdges: FlowEdge[] = rfEdges.map((edge) => {
      const executed = isEdgeExecuted(edge, report);
      return {
        ...edge,
        type: undefined,
        style: {
          stroke: executed ? '#3b82f6' : '#9ca3af',
          strokeWidth: executed ? 2 : 1.5,
          ...(executed ? {} : { strokeDasharray: '5,5' }),
        },
        markerEnd: {
          type: MarkerType.ArrowClosed,
          color: executed ? '#3b82f6' : '#9ca3af',
        },
      };
    });

    return { nodes: enhancedNodes, edges: enhancedEdges };
  }, [report]);

  // 组件挂载时 fitView
  useEffect(() => {
    if (nodes.length > 0) {
      const timer = setTimeout(() => {
        fitView({ padding: 0.3, duration: 200 });
      }, 100);
      return () => clearTimeout(timer);
    }
  }, [fitView, nodes.length]);
  // 节点点击处理：选中并平滑平移可视区域居中聚焦
  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: FlowNode) => {
      const d = node.data as Record<string, unknown>;
      const status = d.__execStatus as string;
      // 未执行节点不可选择
      if (status === 'unexecuted') return;
      const nextSelected = node.id === selectedNodeUuid ? null : node.id;
      onNodeSelect(nextSelected);
      if (nextSelected) {
        const x = node.position.x + (node.width ?? 180) / 2;
        const y = node.position.y + (node.height ?? 40) / 2;
        setCenter(x, y, { duration: 300 });
      }
    },
    [selectedNodeUuid, onNodeSelect, setCenter],
  );

  // 画布空白处点击取消选择
  const onPaneClick = useCallback(() => {
    onNodeSelect(null);
  }, [onNodeSelect]);

  // 查找选中节点的执行报告
  const selectedReport = useMemo(() => {
    if (!selectedNodeUuid) return undefined;
    return report.node_reports.find((r) => r.node_uuid === selectedNodeUuid);
  }, [selectedNodeUuid, report.node_reports]);

  return (
    <div
      ref={containerRef}
      className={cn('flex w-full h-full relative overflow-hidden', isDragging && 'select-none')}
    >
      {/* 左侧画布：自适应占据剩余可视区域 */}
      <div className="flex-1 h-full min-w-0 relative">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={reportNodeTypes}
          nodesDraggable={false}
          nodesConnectable={false}
          elementsSelectable={true}
          onNodeClick={onNodeClick}
          onPaneClick={onPaneClick}
          minZoom={0.1}
          maxZoom={2}
          proOptions={{ hideAttribution: true }}
          fitView={false}
        >
          <Background variant={BackgroundVariant.Dots} gap={20} size={1} />
        </ReactFlow>
      </div>

      {/* 可拖拽分隔手柄 */}
      <div
        onMouseDown={handleMouseDown}
        onDoubleClick={() => setPanelWidth(500)}
        title="拖动调整宽度，双击恢复默认"
        className="w-3 flex items-center justify-center flex-shrink-0 relative group cursor-col-resize z-10"
      >
        <div className="absolute inset-y-0 left-1/2 -translate-x-1/2 w-px bg-border group-hover:bg-blue-400 group-active:bg-blue-500 transition-colors" />
        <div className="relative z-10 flex flex-col items-center gap-0.5 py-2 px-0.5 rounded group-hover:bg-blue-100/50 dark:group-hover:bg-blue-900/40 transition-colors">
          <div className="w-1 h-1 rounded-full bg-muted-foreground/40 group-hover:bg-blue-500 transition-colors" />
          <div className="w-1 h-1 rounded-full bg-muted-foreground/40 group-hover:bg-blue-500 transition-colors" />
          <div className="w-1 h-1 rounded-full bg-muted-foreground/40 group-hover:bg-blue-500 transition-colors" />
        </div>
      </div>

      {/* 右侧详情面板（支持自由拖拽调节宽度） */}
      <div
        style={{ width: panelWidth }}
        className={cn(
          'shrink-0 border-l border-border',
          'flex flex-col bg-background',
          'overflow-hidden shadow-xs',
        )}
      >
        {/* 面板头 */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-border">
          <h3 className="text-sm font-semibold text-foreground">{getLL().execution.report.node_details()}</h3>
          <button
            type="button"
            onClick={() => onNodeSelect(null)}
            className={cn(
              'p-1 rounded-md',
              'text-muted-foreground hover:text-foreground',
              'hover:bg-muted/50 transition-colors',
            )}
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* 面板内容 */}
        <div className="flex-1 overflow-y-auto p-2.5 sm:p-3">
          {selectedNodeUuid && selectedReport ? (
            <NodeExecutionDetails report={selectedReport} />
          ) : (
            <div className="flex items-center justify-center h-full text-muted-foreground text-sm">
              {getLL().execution.canvasReport.select_node_hint()}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// ============================================================================
// 公开组件
// ============================================================================

/**
 * CanvasReportViewer - 基于 React Flow 的只读画布报告查看器
 *
 * 使用方式：
 * ```tsx
 * <CanvasReportViewer
 *   report={report}
 *   selectedNodeUuid={selectedUuid}
 *   onNodeSelect={setSelectedUuid}
 * />
 * ```
 */
export function CanvasReportViewer(props: CanvasReportViewerProps) {
  const { report } = props;

  // flow_snapshot 为空时显示提示
  if (!report.flow_snapshot) {
    return (
      <div className="w-full h-full flex items-center justify-center bg-background">
        <div className="text-center">
          <p className="text-sm text-muted-foreground">
            {getLL().execution.canvasReport.no_topology_data()}
          </p>
        </div>
      </div>
    );
  }

  return (
    <ReactFlowProvider>
      <CanvasReportViewerInner {...props} />
    </ReactFlowProvider>
  );
}
