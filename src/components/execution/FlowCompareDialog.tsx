import { useCallback, useMemo, useState } from 'react';
import { getLL } from '@/i18n/i18n-lazy';
import {
  Plus,
  Minus,
  Pencil,
  Equal,
  GitCompare,
  RotateCcw,
  ChevronDown,
  ChevronRight,
  X,
} from 'lucide-react';
import { toast } from 'sonner';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { diffFlows } from '@/lib/flowDiff';
import { restoreFlowFromSnapshot } from '@/lib/restoreFlow';
import type { FlowDiffResult, NodeDiff, FieldChange, ConnectionDiff } from '@/lib/flowDiff';
import type { Flow } from '@/types';
import {
  ReactFlow,
  ReactFlowProvider,
  Background,
  Controls,
  BackgroundVariant,
  type Node,
  type NodeMouseHandler,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { convertFlowToReactFlow } from '@/components/canvas/utils';
import { useFlowStore } from '@/stores/flow';
import { useUIStore } from '@/stores/ui';
import { flowStorage } from '@/services/storage/flowStorage';
import {
  EntryNode,
  HttpNode,
  ConditionalNode,
  ScriptNode,
  AssertionNode,
  WsConnectNode,
  WsSendCollectNode,
  WsCloseNode,
  DbConnectNode,
  DbQueryNode,
  DbBeginNode,
  DbCommitNode,
  DbRollbackNode,
  DbCloseNode,
} from '@/components/canvas/nodes';

interface FlowCompareDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  snapshotFlow: Flow;
  currentFlow: Flow | null;
  snapshotName: string;
  currentName: string;
}

/** 格式化字段值为可读字符串 */
function formatValue(value: unknown, ll: ReturnType<typeof getLL>): string {
  if (value === undefined) return ll.execution.compare.undefined_value();
  if (value === null) return ll.execution.compare.null_value();
  if (typeof value === 'object') {
    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return String(value);
    }
  }
  return String(value);
}

/** 统计栏组件 */
function StatisticsBar({ diff, ll }: { diff: FlowDiffResult; ll: ReturnType<typeof getLL> }) {
  const added = diff.addedNodes.length + diff.addedConnections.length;
  const removed = diff.removedNodes.length + diff.removedConnections.length;
  const modified = diff.modifiedNodes.length;
  const unchanged = diff.unchangedNodes.length + diff.unchangedConnections.length;

  return (
    <div className="flex items-center gap-2 flex-wrap">
      {added > 0 && (
        <Badge variant="outline" className="bg-green-50 dark:bg-green-900/30 text-green-700 dark:text-green-400 border-green-200 dark:border-green-700">
          <Plus className="w-3 h-3 mr-1" />
          {ll.execution.compare.stat_added({ count: added })}
        </Badge>
      )}
      {removed > 0 && (
        <Badge variant="outline" className="bg-red-50 dark:bg-red-900/30 text-red-700 dark:text-red-400 border-red-200 dark:border-red-700">
          <Minus className="w-3 h-3 mr-1" />
          {ll.execution.compare.stat_removed({ count: removed })}
        </Badge>
      )}
      {modified > 0 && (
        <Badge variant="outline" className="bg-amber-50 dark:bg-amber-900/30 text-amber-700 dark:text-amber-400 border-amber-200 dark:border-amber-700">
          <Pencil className="w-3 h-3 mr-1" />
          {ll.execution.compare.stat_modified({ count: modified })}
        </Badge>
      )}
      {unchanged > 0 && (
        <Badge variant="outline" className="bg-muted text-muted-foreground border-border">
          <Equal className="w-3 h-3 mr-1" />
          {ll.execution.compare.stat_unchanged({ count: unchanged })}
        </Badge>
      )}
    </div>
  );
}

/** 字段变化行：旧值 → 新值 */
function FieldChangeRow({ change, ll }: { change: FieldChange; ll: ReturnType<typeof getLL> }) {
  return (
    <div className="flex items-start gap-2 text-sm py-1.5 px-3 border-b border-border/30 last:border-b-0">
      <span className="font-medium text-foreground min-w-[120px] shrink-0">{change.field}</span>
      <div className="flex items-center gap-2 min-w-0 flex-1">
        <span className="text-red-600 dark:text-red-400 line-through break-all">
          {formatValue(change.oldValue, ll)}
        </span>
        <span className="text-muted-foreground shrink-0">→</span>
        <span className="text-green-600 dark:text-green-400 break-all">
          {formatValue(change.newValue, ll)}
        </span>
      </div>
    </div>
  );
}

/** 流程信息对比区：名称变化 */
function FlowInfoSection({ diff, ll }: { diff: FlowDiffResult; ll: ReturnType<typeof getLL> }) {
  const nameChange = diff.flowChanges.name;
  if (!nameChange) return null;

  return (
    <div className="rounded-lg border border-amber-200 dark:border-amber-800 bg-amber-50/50 dark:bg-amber-900/20 p-3">
      <div className="flex items-center gap-2 mb-2 text-sm font-medium text-amber-700 dark:text-amber-400">
        <GitCompare className="w-4 h-4" />
        {ll.execution.compare.flow_name_changed()}
      </div>
      <div className="flex items-center gap-2 text-sm">
        <span className="text-red-600 dark:text-red-400 line-through">{String(nameChange.oldValue)}</span>
        <span className="text-muted-foreground">→</span>
        <span className="text-green-600 dark:text-green-400">{String(nameChange.newValue)}</span>
      </div>
    </div>
  );
}

/** 初始变量对比区 */
function InitialVariablesSection({ diff, ll }: { diff: FlowDiffResult; ll: ReturnType<typeof getLL> }) {
  const vars = diff.flowChanges.initialVariables;
  if (!vars) return null;

  const hasChanges = vars.added.length > 0 || vars.removed.length > 0 || vars.modified.length > 0;
  if (!hasChanges) return null;

  return (
    <div className="rounded-lg border border-border/50 p-3">
      <h4 className="text-sm font-semibold text-foreground mb-2">{ll.execution.compare.initial_vars_changed()}</h4>
      <div className="space-y-1.5">
        {vars.added.map((key) => (
          <div key={key} className="flex items-center gap-2 text-sm px-2 py-1 rounded bg-green-50 dark:bg-green-900/20">
            <Plus className="w-3.5 h-3.5 text-green-600 dark:text-green-400 shrink-0" />
            <span className="text-green-700 dark:text-green-400">{key}</span>
            <span className="text-muted-foreground">{ll.execution.compare.added_label()}</span>
          </div>
        ))}
        {vars.removed.map((key) => (
          <div key={key} className="flex items-center gap-2 text-sm px-2 py-1 rounded bg-red-50 dark:bg-red-900/20">
            <Minus className="w-3.5 h-3.5 text-red-600 dark:text-red-400 shrink-0" />
            <span className="text-red-700 dark:text-red-400">{key}</span>
            <span className="text-muted-foreground">{ll.execution.compare.removed_label()}</span>
          </div>
        ))}
        {vars.modified.map((change) => (
          <div key={change.field} className="px-2 py-1">
            <FieldChangeRow change={change} ll={ll} />
          </div>
        ))}
      </div>
    </div>
  );
}

/** 单个节点变化项 */
function NodeDiffItem({ node, status, ll }: { node: NodeDiff; status: 'added' | 'removed' | 'modified' | 'unchanged'; ll: ReturnType<typeof getLL> }) {
  const [expanded, setExpanded] = useState(status === 'modified');

  const config = {
    added: {
      icon: <Plus className="w-4 h-4" />,
      iconBg: 'bg-green-100 dark:bg-green-900/50',
      iconColor: 'text-green-600 dark:text-green-400',
      borderColor: 'border-green-300 dark:border-green-700',
      label: ll.execution.compare.node_status_added(),
      labelColor: 'text-green-600 dark:text-green-400',
      bgColor: 'bg-green-50/50 dark:bg-green-900/10',
    },
    removed: {
      icon: <Minus className="w-4 h-4" />,
      iconBg: 'bg-red-100 dark:bg-red-900/50',
      iconColor: 'text-red-600 dark:text-red-400',
      borderColor: 'border-red-300 dark:border-red-700',
      label: ll.execution.compare.node_status_removed(),
      labelColor: 'text-red-600 dark:text-red-400',
      bgColor: 'bg-red-50/50 dark:bg-red-900/10',
    },
    modified: {
      icon: <Pencil className="w-4 h-4" />,
      iconBg: 'bg-amber-100 dark:bg-amber-900/50',
      iconColor: 'text-amber-600 dark:text-amber-400',
      borderColor: 'border-amber-300 dark:border-amber-700',
      label: ll.execution.compare.node_status_modified(),
      labelColor: 'text-amber-600 dark:text-amber-400',
      bgColor: 'bg-amber-50/50 dark:bg-amber-900/10',
    },
    unchanged: {
      icon: <Equal className="w-4 h-4" />,
      iconBg: 'bg-muted',
      iconColor: 'text-muted-foreground',
      borderColor: 'border-border/50',
      label: ll.execution.compare.node_status_unchanged(),
      labelColor: 'text-muted-foreground',
      bgColor: '',
    },
  }[status];

  const hasDetails = status === 'modified' && (node.dataChanges || node.nameChange);

  return (
    <div className={cn('rounded-lg border', config.borderColor, config.bgColor)}>
      <button
        type="button"
        onClick={() => hasDetails ? setExpanded(!expanded) : undefined}
        className={cn(
          'w-full flex items-center gap-2.5 px-3 py-2 text-left',
          hasDetails && 'cursor-pointer hover:bg-muted/30',
          !hasDetails && 'cursor-default',
        )}
      >
        {hasDetails ? (
          expanded ? <ChevronDown className="w-3.5 h-3.5 text-muted-foreground shrink-0" /> : <ChevronRight className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
        ) : (
          <span className="w-3.5" />
        )}
        <span className={cn('p-1 rounded', config.iconBg, config.iconColor)}>
          {config.icon}
        </span>
        <span className="text-sm font-medium text-foreground flex-1 min-w-0 truncate">
          {node.name}
        </span>
        <span className={cn('text-xs', config.labelColor)}>{config.label}</span>
        <Badge variant="outline" className="text-xs font-normal shrink-0">{node.nodeType}</Badge>
      </button>

      {hasDetails && expanded && (
        <div className="border-t border-border/30 px-3 py-2 space-y-1">
          {node.nameChange && <FieldChangeRow change={node.nameChange} ll={ll} />}
          {node.dataChanges?.fieldChanges.map((fc) => (
            <FieldChangeRow key={fc.field} change={fc} ll={ll} />
          ))}
        </div>
      )}
    </div>
  );
}

/** 节点对比区 */
function NodesSection({ diff, ll }: { diff: FlowDiffResult; ll: ReturnType<typeof getLL> }) {
  const hasAny = diff.addedNodes.length > 0 || diff.removedNodes.length > 0 || diff.modifiedNodes.length > 0 || diff.unchangedNodes.length > 0;
  if (!hasAny) return null;

  return (
    <div className="space-y-2">
      <h4 className="text-sm font-semibold text-foreground">{ll.execution.compare.nodes_changed()}</h4>
      {diff.addedNodes.map((n) => <NodeDiffItem key={n.uuid} node={n} status="added" ll={ll} />)}
      {diff.removedNodes.map((n) => <NodeDiffItem key={n.uuid} node={n} status="removed" ll={ll} />)}
      {diff.modifiedNodes.map((n) => <NodeDiffItem key={n.uuid} node={n} status="modified" ll={ll} />)}
      {diff.unchangedNodes.map((n) => <NodeDiffItem key={n.uuid} node={n} status="unchanged" ll={ll} />)}
    </div>
  );
}

/** 连接变化行 */
function ConnectionRow({ conn, status }: { conn: ConnectionDiff; status: 'added' | 'removed' }) {
  const isAdded = status === 'added';
  return (
    <div className={cn(
      'flex items-center gap-2 text-sm px-3 py-1.5 rounded',
      isAdded ? 'bg-green-50 dark:bg-green-900/20' : 'bg-red-50 dark:bg-red-900/20',
    )}>
      {isAdded ? <Plus className="w-3.5 h-3.5 text-green-600 dark:text-green-400 shrink-0" /> : <Minus className="w-3.5 h-3.5 text-red-600 dark:text-red-400 shrink-0" />}
      <span className={cn('font-mono text-xs', isAdded ? 'text-green-700 dark:text-green-400' : 'text-red-700 dark:text-red-400')}>
        {conn.sourceNodeUuid}
      </span>
      <span className="text-muted-foreground">→</span>
      <span className={cn('font-mono text-xs', isAdded ? 'text-green-700 dark:text-green-400' : 'text-red-700 dark:text-red-400')}>
        {conn.targetNodeUuid}
      </span>
      <span className="text-muted-foreground text-xs">({conn.sourcePortId} → {conn.targetPortId})</span>
    </div>
  );
}

/** 连接对比区 */
function ConnectionsSection({ diff, ll }: { diff: FlowDiffResult; ll: ReturnType<typeof getLL> }) {
  const hasChanges = diff.addedConnections.length > 0 || diff.removedConnections.length > 0;
  if (!hasChanges) return null;

  return (
    <div className="rounded-lg border border-border/50 p-3">
      <h4 className="text-sm font-semibold text-foreground mb-2">{ll.execution.compare.connections_changed()}</h4>
      <div className="space-y-1.5">
        {diff.addedConnections.map((c) => <ConnectionRow key={c.id} conn={c} status="added" />)}
        {diff.removedConnections.map((c) => <ConnectionRow key={c.id} conn={c} status="removed" />)}
      </div>
    </div>
  );
}

/** 字段对比 Tab 内容 */
function FieldDiffContent({ diff, ll }: { diff: FlowDiffResult; ll: ReturnType<typeof getLL> }) {
  return (
    <div className="space-y-4">
      <FlowInfoSection diff={diff} ll={ll} />
      <InitialVariablesSection diff={diff} ll={ll} />
      <NodesSection diff={diff} ll={ll} />
      <ConnectionsSection diff={diff} ll={ll} />
    </div>
  );
}

/** Diff 节点状态类型 */
type DiffNodeStatus = 'added' | 'removed' | 'modified' | 'unchanged';

/** Diff 节点状态对应的样式 */
const DIFF_STATUS_STYLES: Record<DiffNodeStatus, string> = {
  added: 'ring-2 ring-green-500 bg-green-50 dark:bg-green-900/20',
  removed: 'ring-2 ring-red-500 bg-red-50 dark:bg-red-900/20 opacity-60',
  modified: 'ring-2 ring-amber-500 bg-amber-50 dark:bg-amber-900/20',
  unchanged: '',
};

/** 节点类型映射（用于画布对比的只读视图） */
const diffNodeTypes = {
  entry: EntryNode,
  http: HttpNode,
  conditional: ConditionalNode,
  script: ScriptNode,
  assertion: AssertionNode,
  ws_connect: WsConnectNode,
  ws_send_collect: WsSendCollectNode,
  ws_close: WsCloseNode,
  db_connect: DbConnectNode,
  db_query: DbQueryNode,
  db_begin: DbBeginNode,
  db_commit: DbCommitNode,
  db_rollback: DbRollbackNode,
  db_close: DbCloseNode,
};

/** 选中节点的配置 Diff 面板 */
function NodeConfigDiffPanel({
  nodeDiff,
  status,
  onClose,
  ll,
}: {
  nodeDiff: NodeDiff;
  status: DiffNodeStatus;
  onClose: () => void;
  ll: ReturnType<typeof getLL>;
}) {
  return (
    <div className="border-t border-border bg-muted/30">
      <div className="flex items-center justify-between px-4 py-2 border-b border-border/50">
        <div className="flex items-center gap-2">
          <Badge variant="outline" className="text-xs">{nodeDiff.nodeType}</Badge>
          <span className="text-sm font-semibold text-foreground">{nodeDiff.name}</span>
          <span className={cn('text-xs', {
            'text-green-600 dark:text-green-400': status === 'added',
            'text-red-600 dark:text-red-400': status === 'removed',
            'text-amber-600 dark:text-amber-400': status === 'modified',
            'text-muted-foreground': status === 'unchanged',
          })}>
            {status === 'added' ? ll.execution.compare.node_added_label() : status === 'removed' ? ll.execution.compare.node_removed_label() : status === 'modified' ? ll.execution.compare.node_modified_label() : ll.execution.compare.node_unchanged_label()}
          </span>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="p-1 rounded hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
        >
          <X className="w-4 h-4" />
        </button>
      </div>
      <ScrollArea className="max-h-[200px]">
        <div className="px-4 py-2">
          {status === 'added' && (
            <p className="text-sm text-green-600 dark:text-green-400">{ll.execution.compare.node_added_desc()}</p>
          )}
          {status === 'removed' && (
            <p className="text-sm text-red-600 dark:text-red-400">{ll.execution.compare.node_removed_desc()}</p>
          )}
          {status === 'modified' && (
            <div className="space-y-1">
              {nodeDiff.nameChange && <FieldChangeRow change={nodeDiff.nameChange} ll={ll} />}
              {nodeDiff.dataChanges?.fieldChanges.map((fc) => (
                <FieldChangeRow key={fc.field} change={fc} ll={ll} />
              ))}
              {(!nodeDiff.nameChange && !nodeDiff.dataChanges?.fieldChanges.length) && (
                <p className="text-sm text-muted-foreground">{ll.execution.compare.no_config_changes()}</p>
              )}
            </div>
          )}
          {status === 'unchanged' && (
            <p className="text-sm text-muted-foreground">{ll.execution.compare.no_changes()}</p>
          )}
        </div>
      </ScrollArea>
    </div>
  );
}

/** 画布对比 Tab 内部组件 */
function CanvasDiffTabInner({
  snapshotFlow,
  diffResult,
  ll,
}: {
  snapshotFlow: Flow;
  diffResult: FlowDiffResult;
  ll: ReturnType<typeof getLL>;
}) {
  const [selectedNodeDiff, setSelectedNodeDiff] = useState<{
    diff: NodeDiff;
    status: DiffNodeStatus;
  } | null>(null);

  // 构建节点 UUID → Diff 状态的映射
  const diffStatusMap = useMemo(() => {
    const map = new Map<string, DiffNodeStatus>();
    for (const n of diffResult.addedNodes) map.set(n.uuid, 'added');
    for (const n of diffResult.removedNodes) map.set(n.uuid, 'removed');
    for (const n of diffResult.modifiedNodes) map.set(n.uuid, 'modified');
    for (const n of diffResult.unchangedNodes) map.set(n.uuid, 'unchanged');
    return map;
  }, [diffResult]);

  // 构建节点 UUID → NodeDiff 的映射
  const nodeDiffMap = useMemo(() => {
    const map = new Map<string, { diff: NodeDiff; status: DiffNodeStatus }>();
    for (const n of diffResult.addedNodes) map.set(n.uuid, { diff: n, status: 'added' });
    for (const n of diffResult.removedNodes) map.set(n.uuid, { diff: n, status: 'removed' });
    for (const n of diffResult.modifiedNodes) map.set(n.uuid, { diff: n, status: 'modified' });
    for (const n of diffResult.unchangedNodes) map.set(n.uuid, { diff: n, status: 'unchanged' });
    return map;
  }, [diffResult]);

  // 转换 Flow → ReactFlow 格式并叠加 diff 样式
  const { nodes, edges } = useMemo(() => {
    const { nodes: rfNodes, edges: rfEdges } = convertFlowToReactFlow(snapshotFlow);

    // 为每个节点添加 diff 样式
    const styledNodes = rfNodes.map((node) => {
      const status = diffStatusMap.get(node.id) ?? 'unchanged';
      const styleClass = DIFF_STATUS_STYLES[status];
      return {
        ...node,
        className: cn(node.className, styleClass),
      };
    });

    return { nodes: styledNodes, edges: rfEdges };
  }, [snapshotFlow, diffStatusMap]);

  // 节点点击处理
  const handleNodeClick = useCallback<NodeMouseHandler>(
    (_: React.MouseEvent, node: Node) => {
      const entry = nodeDiffMap.get(node.id);
      if (entry) {
        setSelectedNodeDiff((prev) =>
          prev?.diff.uuid === entry.diff.uuid ? null : entry
        );
      }
    },
    [nodeDiffMap],
  );

  // 画布空白处点击
  const handlePaneClick = useCallback(() => {
    setSelectedNodeDiff(null);
  }, []);

  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 min-h-0 relative">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={diffNodeTypes}
          nodesDraggable={false}
          nodesConnectable={false}
          elementsSelectable={true}
          onNodeClick={handleNodeClick}
          onPaneClick={handlePaneClick}
          fitView
          fitViewOptions={{ padding: 0.2 }}
          minZoom={0.1}
          maxZoom={2}
          proOptions={{ hideAttribution: true }}
          defaultEdgeOptions={{ animated: true }}
        >
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
            showInteractive={false}
          />
        </ReactFlow>
      </div>
      {selectedNodeDiff && (
        <NodeConfigDiffPanel
          nodeDiff={selectedNodeDiff.diff}
          status={selectedNodeDiff.status}
          onClose={() => setSelectedNodeDiff(null)}
          ll={ll}
        />
      )}
    </div>
  );
}

/** 画布对比 Tab（包裹 ReactFlowProvider） */
function CanvasDiffTab({
  snapshotFlow,
  diffResult,
  ll,
}: {
  snapshotFlow: Flow;
  diffResult: FlowDiffResult;
  ll: ReturnType<typeof getLL>;
}) {
  return (
    <ReactFlowProvider>
      <CanvasDiffTabInner snapshotFlow={snapshotFlow} diffResult={diffResult} ll={ll} />
    </ReactFlowProvider>
  );
}

export function FlowCompareDialog({
  open,
  onOpenChange,
  snapshotFlow,
  currentFlow,
  snapshotName,
  currentName,
}: FlowCompareDialogProps) {
  const ll = getLL();
  const [activeTab, setActiveTab] = useState('field');

  const diff = useMemo(() => {
    if (!currentFlow) return null;
    return diffFlows(snapshotFlow, currentFlow);
  }, [snapshotFlow, currentFlow]);

  // 恢复流程快照
  const handleRestore = useCallback(async () => {
    if (!snapshotFlow) return;
    // 将 useUIStore.showConfirm (回调模式) 适配为 Promise<boolean>
    const confirmAsync = (config: { title: string; description: string; confirmText?: string; cancelText?: string }): Promise<boolean> => {
      return new Promise((resolve) => {
        useUIStore.getState().showConfirm({
          ...config,
          onConfirm: () => resolve(true),
          onCancel: () => resolve(false),
        });
      });
    };

    const result = await restoreFlowFromSnapshot(snapshotFlow, {
      isDirty: useFlowStore.getState().isDirty,
      showConfirm: confirmAsync,
      listFlows: () => flowStorage.listFlows(),
      saveFlow: (flow) => flowStorage.saveFlow(flow),
      setFlow: (flow) => useFlowStore.getState().setFlow(flow),
      generateUuid: () => crypto.randomUUID(),
      toast: { success: toast.success, error: toast.error },
    });
    if (result.success) {
      onOpenChange(false);
    }
  }, [snapshotFlow, onOpenChange]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[90vw] max-w-[1400px] h-[85vh] flex flex-col overflow-hidden p-6 gap-4">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <GitCompare className="w-5 h-5" />
            {ll.execution.compare.title({ snapshot: snapshotName, current: currentName })}
          </DialogTitle>
        </DialogHeader>

        {diff ? (
          <>
            <StatisticsBar diff={diff} ll={ll} />

            <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 min-h-0 flex flex-col">
              <TabsList className="shrink-0 self-start">
                <TabsTrigger value="canvas">{ll.execution.compare.tab_canvas()}</TabsTrigger>
                <TabsTrigger value="field">{ll.execution.compare.tab_field()}</TabsTrigger>
              </TabsList>

              <TabsContent value="canvas" className="flex-1 min-h-0">
                <CanvasDiffTab snapshotFlow={snapshotFlow} diffResult={diff} ll={ll} />
              </TabsContent>

              <TabsContent value="field" className="flex-1 min-h-0">
                <ScrollArea className="h-full">
                  <div className="pr-4 pb-4">
                    <FieldDiffContent diff={diff} ll={ll} />
                  </div>
                </ScrollArea>
              </TabsContent>
            </Tabs>
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center text-muted-foreground">
            <p>{ll.execution.compare.cannot_load()}</p>
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={handleRestore}>
            <RotateCcw className="w-4 h-4 mr-1" />
            {ll.execution.compare.restore_flow()}
          </Button>
          <Button variant="default" onClick={() => onOpenChange(false)}>
            {ll.execution.compare.close()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
