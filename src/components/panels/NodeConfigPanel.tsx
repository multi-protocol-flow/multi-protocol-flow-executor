/**
 * 节点配置面板主组件
 * 根据节点类型显示不同的配置表单
 */
import { useState, useRef, useMemo, useEffect, useCallback } from 'react';

/** 深度比较两个值是否相等（属性顺序无关，undefined 值视为缺失） */
function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (a == null || b == null) return a === b;
  if (typeof a !== typeof b) return false;
  if (Array.isArray(a)) {
    if (!Array.isArray(b)) return false;
    if (a.length !== b.length) return false;
    return a.every((item, i) => deepEqual(item, b[i]));
  }
  if (typeof a === 'object') {
    const aObj = a as Record<string, unknown>;
    const bObj = b as Record<string, unknown>;
    const aKeys = Object.keys(aObj).filter(k => aObj[k] !== undefined);
    const bKeys = Object.keys(bObj).filter(k => bObj[k] !== undefined);
    if (aKeys.length !== bKeys.length) return false;
    const bKeySet = new Set(bKeys);
    return aKeys.every(k => bKeySet.has(k) && deepEqual(aObj[k], bObj[k]));
  }
  return false;
}

/** 检查是否为普通对象（非数组、非 null） */
function isPlainObject(obj: unknown): obj is Record<string, unknown> {
  return typeof obj === 'object' && obj !== null && !Array.isArray(obj);
}

/**
 * 合并面板输出与原始节点数据。
 * `updates` 中的字段覆盖 `original`。
 * `original` 中有但 `updates` 中没有的字段被保留。
 * `updates` 中显式设为 `undefined` 的字段从结果中移除。
 */
function mergeNodeData<T extends Record<string, unknown>>(original: T, updates: Partial<T>): T {
  const merged = { ...original } as T;
  for (const key of Object.keys(updates) as Array<keyof T>) {
    const val = updates[key];
    if (val === undefined) {
      // 显式删除: 从结果中移除该 key
      delete merged[key];
    } else if (isPlainObject(val) && isPlainObject(original[key])) {
      // 深度合并嵌套的普通对象（不合并数组）
      merged[key] = { ...(original[key] as object), ...(val as object) } as T[keyof T];
    } else {
      merged[key] = val as T[keyof T];
    }
  }
  return merged;
}
import { useFlowStore, useDebugStore, selectHistoryVersion } from '@/stores';
import { DebugNodeConfigDialog } from '@/components/dialogs/DebugNodeConfigDialog';
import { FlowNodeList } from './FlowNodeList';
import { getPanelLoader, isSchemaDriven } from '@/lib/configPanelRegistry';
import { SchemaConfigPanel } from '@/components/panels/SchemaConfigPanel';
import { Button } from '@/components/ui/button';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { NodeData, Node, NodeType } from '@/types';
import { MousePointerClick, PencilLine, CircleHelp } from 'lucide-react';
import { HelpDrawer } from './HelpDrawer';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import { toast } from 'sonner';
import { OnErrorSection } from '@/components/panels/shared/OnErrorSection';
import { getCachedNodeMeta } from '@/lib/NodeRegistry';

interface NodeConfigPanelProps {
  /** 只读模式（调试时查看节点配置） */
  readOnly?: boolean;
  /** 指定要显示的节点（用于只读模式，不使用 store 中的选中节点） */
  node?: Node | null;
}

/** 懒加载配置面板包装组件 — 手动加载，避免 React.lazy 动态创建的身份丢失问题 */
function LazyConfigPanel({ nodeType, data, onSave, nodeUuid }: { nodeType: string; data: NodeData; onSave: (data: NodeData) => void; nodeUuid: string | null }) {
  const [Component, setComponent] = useState<React.ComponentType<any> | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const loader = getPanelLoader(nodeType);
    if (!loader) {
      setComponent(null);
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    loader().then(comp => {
      if (!cancelled) {
        setComponent(() => comp);
        setIsLoading(false);
      }
    });

    return () => { cancelled = true; };
  }, [nodeType]);

  if (isLoading || !Component) {
    return (
      <div className="space-y-4 p-4 animate-pulse" aria-label={getLL().panels.loading()}>
        <div className="h-8 bg-muted rounded-md w-3/4" />
        <div className="h-24 bg-muted rounded-md" />
        <div className="h-8 bg-muted rounded-md w-1/2" />
        <div className="h-8 bg-muted rounded-md" />
      </div>
    );
  }

  return <Component key={nodeUuid} data={data} onSave={onSave} />;
}

export function NodeConfigPanel({ readOnly = false, node: externalNode }: NodeConfigPanelProps) {
  const { getSelectedNode } = useFlowStore();
  const modifyNodeConfig = useDebugStore((state) => state.modifyNodeConfig);
  const historyVersion = useFlowStore(selectHistoryVersion);
  const ll = getLL().panels.nodeConfig;

  // 优先使用外部传入的节点，否则回退到 store 中的选中节点
  const selectedNode = externalNode ?? getSelectedNode();

  // 当前渲染周期的节点 UUID（在渲染阶段同步捕获，用于 handleSave 闭包）
  // 子组件的 cleanup effect 中持有的 onSave 闭包捕获的是创建时的 uuid，
  // 因此即使节点切换后 cleanup 执行，保存的目标仍是正确的旧节点。
  const renderNodeUuid = selectedNode?.uuid ?? null;

  // 稳定数据引用：仅在节点 UUID 变化或 undo/redo 时更新
  // 避免自动保存后 store 更新 → data prop 变化 → 子组件 reset() → 输入框失焦
  // historyVersion 作为额外依赖，确保 undo/redo 能正确同步表单状态
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const stableData = useMemo(() => selectedNode?.data ?? null, [selectedNode?.uuid, historyVersion]);

  // 调试模式下编辑配置弹窗状态
  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false);

  // 帮助抽屉状态
  const [isHelpDrawerOpen, setIsHelpDrawerOpen] = useState(false);

  // 面板边界（用于帮助面板定位）— 必须在所有条件 return 之前声明
  const panelRef = useRef<HTMLDivElement>(null);
  const [panelBounds, setPanelBounds] = useState<{ top: number; left: number; width: number; height: number } | null>(null);

  useEffect(() => {
    if (isHelpDrawerOpen && panelRef.current) {
      const rect = panelRef.current.getBoundingClientRect();
      setPanelBounds({ top: rect.top, left: rect.left, width: rect.width, height: rect.height });
    }
  }, [isHelpDrawerOpen]);

  // 单端口节点类型（无 false 输出端口）— 必须在所有条件 return 之前声明（hook 规则）
  const SINGLE_PORT_TYPES = new Set<NodeType>([
    'entry', 'end', 'db_close', 'ws_close', 'grpc_close', 'mcp_close',
    'tcp_close', 'udp_close', 'kafka_close', 'sse_disconnect',
    'graphql_disconnect', 'amqp_close', 'amqp10_disconnect',
    'smtp_disconnect', 'imap_close', 'redis_close', 'proxy',
    'variable_extractor',
  ]);

  // 判断节点是否有 false 输出端口 — 必须在所有条件 return 之前声明（hook 规则）
  const hasFalsePort = useMemo(() => {
    if (!selectedNode) return true; // 没有选中节点时默认显示（选中后有值后重新计算）
    // 优先检查节点自带的 ports 数组
    if (selectedNode.ports && selectedNode.ports.length > 0) {
      return selectedNode.ports.some((p) => p.id === 'false');
    }
    // 其次从缓存的后端元数据中查找
    const meta = getCachedNodeMeta(selectedNode.type);
    if (meta?.ports && meta.ports.length > 0) {
      return meta.ports.some((p) => p.id === 'false');
    }
    // 最终回退到已知的单端口类型列表
    return !SINGLE_PORT_TYPES.has(selectedNode.type);
  }, [selectedNode?.ports, selectedNode?.type]);

  // 右键复制标题
  const handleCopyTitle = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const name = selectedNode?.name;
    if (!name) return;
    navigator.clipboard.writeText(name).then(() => {
      toast.success(`${getLL().execution.report.copied()}: ${name}`, { duration: 1500 });
    });
  }, [selectedNode?.name]);

  // 未选中节点时显示流程节点列表
  if (!selectedNode) {
    if (readOnly) {
      return (
        <div className="flex h-full flex-col items-center justify-center p-6 text-center">
          <div className="rounded-full bg-muted p-4 mb-4">
            <MousePointerClick className="h-8 w-8 text-muted-foreground" />
          </div>
          <h3 className="text-lg font-medium text-foreground">
            {ll.debugMode()}
          </h3>
          <p className="mt-2 text-sm text-muted-foreground">
            {ll.debugHint()}
          </p>
        </div>
      );
    }
    return <FlowNodeList />;
  }

  // 冻结节点 UUID，防止快速切换节点时闭包捕获到过时的节点
  // 仅用于调试模式下 DebugNodeConfigDialog 的回调
  const frozenNodeUuid = selectedNode.uuid;

  // stableData 在节点 UUID 变化后才更新，此處 selectedNode 必定非空
  const nodeData = stableData as NodeData;

  /**
   * 处理配置保存
   * 通过闭包捕获当前渲染时刻的节点 UUID（renderNodeUuid），
   * 确保子组件卸载时 cleanup 中调用的 onSave 仍能正确保存到原节点。
   *
   * 合并策略：面板输出的 data 只包含该面板已知的字段，
   * 通过 mergeNodeData 与原始节点数据合并，确保未知字段不被丢弃。
   */
  const handleSave = (data: NodeData) => {
    if (!renderNodeUuid) return;
    // 使用 getState() 获取调用时刻的最新 store 状态，避免闭包捕获过期 flow
    const { flow: latestFlow, updateNode: doUpdate } = useFlowStore.getState();
    if (!latestFlow) return;
    const node = latestFlow.nodes.find((n) => n.uuid === renderNodeUuid);
    // 合并：面板输出覆盖原始值，但面板不认识的字段从原始值保留
    const merged = node ? mergeNodeData(node.data as NodeData & Record<string, unknown>, data as NodeData & Record<string, unknown>) : data;
    if (node && deepEqual(node.data, merged as NodeData)) {
      return;
    }
    doUpdate(renderNodeUuid, { data: merged as NodeData });
  };

  return (
    <Card ref={panelRef} className="flex h-full flex-col overflow-hidden border-0 shadow-none w-full min-w-0">
      <CardHeader className="shrink-0 border-b border-muted bg-background pb-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center flex-1 min-w-0">
            <CardTitle
              className="text-base font-semibold text-foreground truncate cursor-default"
              title={selectedNode.name}
              onContextMenu={handleCopyTitle}
            >
              {selectedNode.name}
            </CardTitle>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6"
              onClick={(e) => {
                e.stopPropagation();
                setIsHelpDrawerOpen(true);
              }}
              title={getLL().panels.nodeConfig.help()}
            >
              <CircleHelp className="h-4 w-4 text-muted-foreground hover:text-primary" />
            </Button>
            {readOnly && (
              <>
                <span className="inline-flex items-center rounded-md border border-amber-200 dark:border-amber-800 bg-amber-50 dark:bg-amber-950/50 px-2 py-0.5 text-xs font-medium text-amber-700 dark:text-amber-300">
                  {ll.readOnly()}
                </span>
                {selectedNode && (
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-6 gap-1.5 text-xs"
                    onClick={() => setIsEditDialogOpen(true)}
                  >
                    <PencilLine className="h-3 w-3" />
                    {ll.editConfig()}
                  </Button>
                )}
              </>
            )}
          </div>
        </div>
      </CardHeader>

      <CardContent className="flex-1 overflow-hidden p-0 relative">
        <ScrollArea className="h-full">
          <div className={cn(
            "w-full min-w-0 max-w-full overflow-hidden px-6 py-4",
            readOnly && "pointer-events-none select-none opacity-80"
          )}>
            {/* 根据节点类型注册表渲染对应的配置组件（懒加载） */}
            {(() => {
              const nodeType = selectedNode.type;
              // 注册表元数据（插件节点携带 config_schema / frontend）
              const meta = getCachedNodeMeta(nodeType);
              // 无硬编码面板的类型（含插件节点）→ Schema 驱动渲染
              if (isSchemaDriven(nodeType)) {
                // 插件声明了自定义前端 → P1 提示（P0 仅支持 inline，此处渲染惰性占位）
                if (meta?.frontend) {
                  return (
                    <div className="w-full rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
                      <p className="text-xs text-muted-foreground">
                        {ll.pluginFrontendNotSupported()}
                      </p>
                    </div>
                  );
                }
                // 插件提供了 config_schema → 动态 JSON-Schema 表单；否则保持空白 fallback
                const rawSchema: unknown = meta?.config_schema;
                const pluginSchema: Record<string, unknown> =
                  isPlainObject(rawSchema) ? rawSchema : {};
                return (
                  <SchemaConfigPanel
                    key={selectedNode.uuid}
                    schema={pluginSchema}
                    data={nodeData as NodeData & Record<string, unknown>}
                    onSave={(data: Record<string, unknown>) => handleSave(data as NodeData & Record<string, unknown>)}
                  />
                );
              }
              // 已知节点类型 → 懒加载对应的配置面板
              return (
                <LazyConfigPanel
                  nodeType={nodeType}
                  data={nodeData}
                  onSave={handleSave}
                  nodeUuid={selectedNode.uuid}
                />
              );
            })()}

            {/* 节点执行错误处理策略（仅双端口节点显示，Assertion 节点自身已包含 OnErrorSection） */}
            {hasFalsePort && (
              <div className="mt-4">
                <OnErrorSection
                  value={selectedNode.on_error}
                  onChange={(v) => {
                    useFlowStore.getState().updateNode(selectedNode.uuid, { on_error: v });
                  }}
                />
              </div>
            )}
          </div>
        </ScrollArea>
      </CardContent>

      {/* 调试模式下编辑配置弹窗（使用 selectedNode.data 以获取最新数据） */}
      {readOnly && selectedNode && (
        <DebugNodeConfigDialog
          open={isEditDialogOpen}
          nodeData={selectedNode.data}
          nodeName={selectedNode.name}
          onSave={async (data) => {
            await modifyNodeConfig(frozenNodeUuid, data);
          }}
          onCancel={() => setIsEditDialogOpen(false)}
        />
      )}

      <HelpDrawer
        nodeTypeId={selectedNode.type}
        open={isHelpDrawerOpen}
        onClose={() => setIsHelpDrawerOpen(false)}
        panelBounds={panelBounds}
      />
    </Card>
  );
}
