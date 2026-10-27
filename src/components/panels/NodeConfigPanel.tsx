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

const SINGLE_PORT_TYPES = new Set<NodeType>([
  'entry', 'end', 'ws:close',
  'tcp:close', 'udp:close', 'sse:disconnect',
  'graphql:disconnect',
  'proxy',
  'variable:extractor',
]);

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

/**
 * 插件 iframe 面板专用合并：与 mergeNodeData 相同（保留面板未发送的顶层字段、
 * 显式 undefined 删除），但嵌套普通对象**整体替换**而非深合并。
 *
 * 插件面板以 init 的完整 config 为种子且全量回传，`{}` 就是"清空"语义；
 * 深合并会让删除对象字段（headers/env）永远失效。参见 handlePluginSave。
 */
function mergeNodeDataReplace<T extends Record<string, unknown>>(original: T, updates: Partial<T>): T {
  const merged = { ...original } as T;
  for (const key of Object.keys(updates) as Array<keyof T>) {
    const val = updates[key];
    if (val === undefined) {
      delete merged[key];
    } else {
      merged[key] = val as T[keyof T];
    }
  }
  return merged;
}
import { useFlowStore, useDebugStore, selectHistoryVersion, selectNodes } from '@/stores';
import { DebugNodeConfigDialog } from '@/components/dialogs/DebugNodeConfigDialog';
import { FlowConfigPanel } from './FlowConfigPanel';
import { getPanelLoader, isSchemaDriven } from '@/lib/configPanelRegistry';
import { SchemaConfigPanel } from '@/components/panels/SchemaConfigPanel';
import { Button } from '@/components/ui/button';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { NodeData, Node, NodeType } from '@/types';
import { MousePointerClick, PencilLine, CircleHelp, Play, Loader2, Ban } from 'lucide-react';
import { startInteractiveSliceExecution } from '@/services/tauri';
import { useExecutionStore, selectIsExecuting } from '@/stores/execution';
import { HelpDrawer } from './HelpDrawer';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import { useSettingsStore } from '@/stores/settings';
import { toast } from 'sonner';
import { OnErrorSection } from '@/components/panels/shared/OnErrorSection';
import { getCachedNodeMeta } from '@/lib/NodeRegistry';
import { PluginIframe } from '@/components/plugin/PluginIframe';
import { useFlowVariables } from '@/hooks/useFlowVariables';
import { useTheme } from '@/hooks/useTheme';
import { PLUGIN_HOST_API, createPluginRequestHandler } from '@/lib/pluginRequestActions';

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
  // 订阅当前选中的节点对象（selectedNodeId 变化或当前节点数据变化时触发更新）
  const storeSelectedNode = useFlowStore((s) =>
    typeof s.getSelectedNode === 'function' ? s.getSelectedNode() : null
  );
  const modifyNodeConfig = useDebugStore((state) => state.modifyNodeConfig);
  const historyVersion = useFlowStore(selectHistoryVersion);
  const ll = getLL().panels.nodeConfig;

  // 插件 iframe 配置面板的变量快照（与现有面板同源：useFlowVariables）
  // 注意：必须在所有条件 return 之前调用（hook 规则）
  const flowVariables = useFlowVariables();
  const pluginVariables = useMemo(
    () => Object.fromEntries(flowVariables.variables.map((v) => [v.name, v.value])),
    [flowVariables.variables]
  );

  // 同插件节点快照的数据源（iframe 面板 connection 下拉等跨节点引用用）
  // 必须在所有条件 return 之前调用（hook 规则）
  const allFlowNodes = useFlowStore(selectNodes);

  // 宿主当前 UI 语言（BCP-47，透传 iframe init 载荷；设置 store 是唯一事实源）
  // 必须在所有条件 return 之前调用（hook 规则）
  const locale = useSettingsStore((s) => s.locale ?? 'en-US');

  // 宿主已解析主题（light/dark，透传 iframe init 载荷并在变化时推 themeChanged）
  // 必须在所有条件 return 之前调用（hook 规则）
  const { resolvedTheme } = useTheme();

  // 优先使用外部传入的节点，否则回退到 store 中的选中节点
  const selectedNode = externalNode ?? storeSelectedNode;

  // 当前渲染周期的节点 UUID（在渲染阶段同步捕获，用于 handleSave 闭包）
  // 子组件的 cleanup effect 中持有的 onSave 闭包捕获的是创建时的 uuid，
  // 因此即使节点切换后 cleanup 执行，保存的目标仍是正确的旧节点。
  const renderNodeUuid = selectedNode?.uuid ?? null;
  // 当前渲染周期的流程 UUID（在渲染阶段同步捕获，防止切标签时卸载保存串扰其他流程）
  const currentFlow = useFlowStore((s) => s.flow);
  const renderFlowUuid = currentFlow?.uuid ?? null;

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
  const isExecuting = useExecutionStore(selectIsExecuting);

  const handleTestCurrentNode = useCallback(async () => {
    if (!selectedNode || isExecuting) return;
    const currentFlow = useFlowStore.getState().flow;
    if (!currentFlow) return;
    await startInteractiveSliceExecution(currentFlow, [selectedNode.uuid]);
  }, [selectedNode, isExecuting]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
        const panelElement = panelRef.current;
        if (panelElement && panelElement.contains(document.activeElement)) {
          e.preventDefault();
          e.stopPropagation();
          handleTestCurrentNode();
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown, true);
    return () => window.removeEventListener('keydown', handleKeyDown, true);
  }, [handleTestCurrentNode]);


  // 未选中节点时显示流程属性面板
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
    return <FlowConfigPanel />;
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
  const saveWith = (
    data: NodeData,
    merge: (o: NodeData & Record<string, unknown>, u: NodeData & Record<string, unknown>) => NodeData & Record<string, unknown>,
  ) => {
    if (!renderNodeUuid) return;
    const { flow: activeFlow, activeTabUuid, tabSessions, updateNode: doUpdate } = useFlowStore.getState();

    // 1. 若当前活跃流程依然是本面板渲染时的流程：正常更新活跃 flow
    if (activeFlow && activeFlow.uuid === renderFlowUuid && activeTabUuid === renderFlowUuid) {
      const node = activeFlow.nodes.find((n) => n.uuid === renderNodeUuid);
      const merged = node
        ? merge(node.data as NodeData & Record<string, unknown>, data as NodeData & Record<string, unknown>)
        : data;
      if (node && deepEqual(node.data, merged as NodeData)) {
        return;
      }
      doUpdate(renderNodeUuid, { data: merged as NodeData });
      return;
    }

    // 2. 关键防串写隔离屏障：如果 activeFlow 已经不是 renderFlowUuid（即发生了标签页切换），
    // 严禁调用当前 activeFlow 的 doUpdate，防止同 UUID 节点被串写！
    // 此时定向写回其源流程 session：tabSessions[renderFlowUuid]
    if (renderFlowUuid && tabSessions[renderFlowUuid]) {
      const targetSession = tabSessions[renderFlowUuid];
      const targetFlow = targetSession.flow;
      const node = targetFlow.nodes.find((n) => n.uuid === renderNodeUuid);
      if (!node) return;
      const merged = merge(
        node.data as NodeData & Record<string, unknown>,
        data as NodeData & Record<string, unknown>,
      );
      if (deepEqual(node.data, merged as NodeData)) {
        return;
      }
      const updatedNodes = targetFlow.nodes.map((n) =>
        n.uuid === renderNodeUuid ? { ...n, data: merged as NodeData } : n,
      );
      const updatedFlow = { ...targetFlow, nodes: updatedNodes };
      useFlowStore.setState((state) => ({
        tabSessions: {
          ...state.tabSessions,
          [renderFlowUuid]: {
            ...state.tabSessions[renderFlowUuid],
            flow: updatedFlow,
            isDirty: true,
          },
        },
        tabs: state.tabs.map((t) =>
          t.uuid === renderFlowUuid ? { ...t, isDirty: true } : t,
        ),
      }));
    }
  };
  const handleSave = (data: NodeData) => saveWith(data, mergeNodeData);
  /**
   * 插件 iframe 面板的保存：面板以 init 携带的完整 config 为种子，configChanged
   * 总是回传完整配置（postMessage 结构化克隆会丢弃 undefined 值，面板无法用
   * undefined 表达"删除"）。因此嵌套对象字段（headers/env 等）必须**整体替换**
   * 而非深合并 —— 深合并会把面板发出的 `{}`（清空）与原始键合并回去，导致
   * 删除 header/env 永远不生效，且 deepEqual 判等使 store 完全不更新。
   */
  const handlePluginSave = (data: NodeData) => saveWith(data, mergeNodeDataReplace);

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
            {(selectedNode?.disabled === true || selectedNode?.on_error === 'bypass') && (
              <span
                onClick={() =>
                  useFlowStore.getState().updateNode(selectedNode.uuid, {
                    disabled: false,
                    on_error: selectedNode.on_error === 'bypass' ? 'route_to_false' : selectedNode.on_error,
                  })
                }
                className="ml-2 inline-flex items-center gap-1 rounded-md border border-amber-300 dark:border-amber-700 bg-amber-100/70 dark:bg-amber-950/60 px-2 py-0.5 text-xs font-medium text-amber-800 dark:text-amber-200 cursor-pointer hover:bg-amber-200/70 shrink-0"
                title={getLL().flow.enable_node()}
              >
                <Ban className="h-3 w-3 text-amber-600 dark:text-amber-400" />
                {getLL().flow.node_bypassed_tag()}
              </span>
            )}
          </div>
          <div className="flex items-center gap-2 shrink-0">
            {selectedNode && !readOnly && (
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="h-6 gap-1 px-2 text-xs font-medium border-emerald-500/40 text-emerald-600 hover:bg-emerald-500/10 dark:text-emerald-400 dark:border-emerald-500/30 cursor-pointer"
                onClick={handleTestCurrentNode}
                disabled={isExecuting}
                title={`${getLL().flow.context_test_node()} (Ctrl+Enter)`}
              >
                {isExecuting ? (
                  <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />
                ) : (
                  <Play className="h-3 w-3 fill-current text-emerald-500" />
                )}
                <span>{getLL().flow.context_test_node()}</span>
              </Button>
            )}
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
                // 插件声明了自定义前端 iframe（meta.frontend.url）→ 渲染 PluginIframe 桥
                const frontendUrl = meta?.frontend?.url;
                if (frontendUrl) {
                  // 流程节点快照（供 connection 下拉、跨节点引用等使用；插件按需自行过滤）
                  const nodes = allFlowNodes.map((n) => ({
                    uuid: n.uuid,
                    label: n.name,
                    type: n.type,
                    config: n.data as unknown as Record<string, unknown>,
                  }));
                  return (
                    <PluginIframe
                      // 按节点 uuid 键控：同一插件下所有节点共享同一 iframe URL
                      // （mpe-plugin://<id>.localhost/config-panel），src 不变不会
                      // 触发 iframe 重载；不加 key 切换节点时 usePluginIframe 只发
                      // configUpdated（不含 nodeType），面板停留在旧节点的表单。
                      // 键控重挂载 → 重新 load → init 携带新节点完整上下文。
                      key={selectedNode.uuid}
                      src={frontendUrl}
                      origin={new URL(frontendUrl).origin}
                      config={nodeData}
                      variables={pluginVariables}
                      // hostApi 能力声明与 requestAction 白名单同源（PR-5A 安全门）：
                      // 未知动作被 createPluginRequestHandler 立即拒绝 → usePluginIframe
                      // 经 actionResult 回传错误，插件不会挂起
                      hostApi={PLUGIN_HOST_API}
                      // 节点上下文：nodeType 供面板按类型渲染表单；locale 供面板取内联文案；
                      // theme 供面板跟随宿主明暗模式；nodes 快照供 connection 下拉等跨节点引用
                      nodeType={nodeType}
                      locale={locale}
                      theme={resolvedTheme}
                      nodes={nodes}
                      onConfigChange={(config) => handlePluginSave(config as NodeData)}
                      onRequestAction={createPluginRequestHandler(nodeData, nodeType)}
                    />
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
                  value={
                    selectedNode.on_error === 'bypass'
                      ? 'route_to_false'
                      : selectedNode.on_error
                  }
                  onChange={(v) => {
                    useFlowStore.getState().updateNode(selectedNode.uuid, {
                      on_error: v,
                      disabled: false,
                    });
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
