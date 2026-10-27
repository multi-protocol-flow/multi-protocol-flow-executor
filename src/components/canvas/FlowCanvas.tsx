/**
 * FlowCanvas - 主画布组件
 * 基于 React Flow 的流程编辑器
 */
import { memo, useCallback, useEffect, useRef, useState } from 'react';
import YAML from 'yaml';
import {
  ReactFlowProvider,
  useReactFlow,
  type Node,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { FlowSurface } from './FlowSurface';
import { CanvasToolbar } from './CanvasToolbar';
import { DebugBreadcrumbs } from './DebugBreadcrumbs';
import { StageTabBar } from './StageTabBar';
import { NodePalette } from './NodePalette';
import { NodeContextMenu } from './NodeContextMenu';
import { EmptyCanvasTips } from './EmptyCanvasTips';
import {
  useFlowStore,
  useUIStore,
  useFlowListStore,
  selectFlow,
  selectIsDirty,
  connectionFor,
  getStageGraph,
} from '@/stores';
import type { NodeData, NodeType } from '@/types';
import { isEntryNodeType } from '@/types';
import { datasourceGetTemplates } from '@/services/tauri/datasource';
import {
  createDefaultNodeData,
  createDefaultNodeName,
  findEdgeToInsert,
  insertNodeIntoConnection,
} from './utils';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import { flowStorage } from '@/services/storage';
import { inferDefaults } from '@/lib/nodeDefaults';
import { toast } from 'sonner';
import { parseCurlToHttpNodeConfig } from '@/lib/curlParser';
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
    text-rendering: geometricPrecision;
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
  /* 被旁路禁用的节点视觉指示（半透明弱化、灰度、虚线边框；严禁任何 :hover 样式与过渡动画，避免在非整数变换视口中触发图层重绘导致全画布节点抖动） */
  .react-flow__node.node-bypassed {
    opacity: 0.45 !important;
    filter: grayscale(85%) !important;
    transition: none !important;
  }
  .react-flow__node.node-bypassed * {
    transition: none !important;
  }
  .react-flow__node.node-bypassed > div {
    border-style: dashed !important;
    border-color: rgba(148, 163, 184, 0.6) !important;
  }
  .react-flow__node.node-bypassed .react-flow__handle:hover {
    transform: none !important;
  }
  .react-flow__node.node-bypassed:hover {
    opacity: 0.45 !important;
  }
`;

/** MiniMap 节点颜色 — 静态回退 */
export const FALLBACK_MINIMAP_COLORS: Record<string, string> = {
  entry: '#22c55e',
  'http:request': '#3b82f6',
  conditional: '#f97316',
  'ws:connect': '#f59e0b',
  'ws:send_collect': '#14b8a6',
  'ws:close': '#f43f5e',
  assertion: '#a855f7',
  script: '#6366f1',
  'flow:sub': '#8b5cf6',
  'tcp:connect': '#10b981',
  'tcp:send': '#10b981',
  'tcp:receive': '#10b981',
  'tcp:close': '#10b981',
  'udp:bind': '#f97316',
  'udp:send_to': '#f97316',
  'udp:recv_from': '#f97316',
  'udp:close': '#f97316',
  'sse:connect': '#f43f5e',
  'sse:listen': '#f43f5e',
  'sse:disconnect': '#f43f5e',
  'graphql:connect': '#6366f1',
  'graphql:query': '#6366f1',
  'graphql:subscribe': '#6366f1',
  'graphql:introspect': '#6366f1',
  'graphql:disconnect': '#6366f1',
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
    const lowerName = file.name.toLowerCase();
    if (
      !lowerName.endsWith('.mpf') &&
      !lowerName.endsWith('.json') &&
      !lowerName.endsWith('.yaml') &&
      !lowerName.endsWith('.yml')
    ) {
      return;
    }
    const reader = new FileReader();
    reader.onload = (e) => {
      try {
        const text = e.target?.result as string;
        let data: Record<string, unknown>;
        try {
          data = JSON.parse(text);
        } catch {
          data = YAML.parse(text) as Record<string, unknown>;
        }
        // 支持两种格式：直接 Flow 对象和 FlowFile 包装格式
        const flowData = (data?.flow ?? data) as Record<string, unknown> | undefined;
        if (flowData && flowData.uuid && flowData.nodes) {
          onImport(flowData as unknown as import('@/types/flow').Flow);
        }
      } catch (err) {
        console.error('[ImportFlow] Failed to parse flow file:', err);
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
        accept=".mpf,.json,.yaml,.yml"
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
  const addNode = useFlowStore((s) => s.addNode);
  const pendingAddNodeType = useFlowStore((s) => s.pendingAddNodeType);
  const clearPendingAddNodeType = useFlowStore((s) => s.clearPendingAddNodeType);
  const isDirty = useFlowStore(selectIsDirty);
  const markSaved = useFlowStore((s) => s.markSaved);
  const ll = getLL();
  const wc = ll.welcome;

  const { screenToFlowPosition, fitView } = useReactFlow();

  // 画布容器引用（用于计算视口中心及传递给 FlowSurface）
  const canvasRef = useRef<HTMLDivElement>(null);
  const mousePosRef = useRef<{ x: number; y: number } | null>(null);


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

  // 流程首次加载或切换时触发 fitView
  useEffect(() => {
    if (flow && flow.uuid !== fittedFlowUuidRef.current) {
      fittedFlowUuidRef.current = flow.uuid;
      // 首次加载或切换流程时即时居中适配，无须等待 300ms 缩放动画
      setTimeout(() => {
        fitView({ padding: 0.4, duration: 0 });
      }, 50);
    }
  }, [flow?.uuid, fitView]);

  // 空闲时预加载画布核心模块与常用组件，消除首次点击流程时的 ESM 动态解析延迟
  useEffect(() => {
    const preload = () => {
      import('./FlowSurface');
      import('./CanvasToolbar');
      import('./DebugBreadcrumbs');
      import('./NodePalette');
      import('@/components/panels/FlowConfigPanel');
    };
    if (typeof window !== 'undefined') {
      if ('requestIdleCallback' in window) {
        window.requestIdleCallback(preload);
      } else {
        setTimeout(preload, 100);
      }
    }
  }, []);

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
        console.error('[FlowCanvas] Auto-save failed:', err);
      });
    }, 500); // 500ms 防抖

    return () => clearTimeout(timeoutId);
  }, [isDirty, flow, markSaved]);

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

  // 处理多选区域右键菜单（Shift 框选区域内右键）
  const onSelectionContextMenu = useCallback(
    (event: React.MouseEvent, selectedNodes: Node[]) => {
      event.preventDefault();
      if (!selectedNodes || selectedNodes.length === 0) return;
      const firstNode = selectedNodes[0];
      const nodeData = firstNode.data as { label?: string };
      setContextMenu({
        x: event.clientX,
        y: event.clientY,
        nodeId: firstNode.id,
        nodeType: firstNode.type || '',
        nodeName:
          selectedNodes.length > 1
            ? getLL().flow.selected_nodes_count({ count: selectedNodes.length })
            : nodeData?.label || '',
      });
    },
    []
  );
  // 复制节点（含源端连接复制）
  const handleCopyNode = useCallback(
    (nodeId: string) => {
      const { flow: currentFlow, activeStage, addNode: addNewNode } = useFlowStore.getState();
      if (!currentFlow) return;

      const sourceNode = getStageGraph(currentFlow, activeStage).nodes.find((n) => n.uuid === nodeId);
      if (!sourceNode || sourceNode.type === 'entry' || sourceNode.type === 'http:ingress') return;

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
      const { flow: currentFlow, activeStage } = useFlowStore.getState();
      if (!currentFlow) return;

      const sourceNode = getStageGraph(currentFlow, activeStage).nodes.find((n) => n.uuid === nodeId);
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
      const { flow: currentFlow, activeStage, updateNode: modifyNode } = useFlowStore.getState();
      if (!currentFlow) return;

      const targetNode = getStageGraph(currentFlow, activeStage).nodes.find((n) => n.uuid === nodeId);
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
      const { flow: currentFlow, activeStage, removeNode: removeNodeById, selectNode: deselectNode } = useFlowStore.getState();
      if (!currentFlow) return;

      const stageNodes = getStageGraph(currentFlow, activeStage).nodes;
      const node = stageNodes.find((n) => n.uuid === nodeId);
      if (!node) return;
      const hasAlternativeEntry = stageNodes.some(
        (n) => n.uuid !== nodeId && isEntryNodeType(n.type)
      );
      if (isEntryNodeType(node.type) && !hasAlternativeEntry) return;
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

  // 触发节点就地内联重命名
  const handleRenameNode = useCallback((nodeId: string) => {
    useUIStore.getState().setRenamingNodeId(nodeId);
  }, []);

  // 处理拖拽经过
  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  // 处理拖放 - 使用鼠标实际位置
  const onDrop = useCallback(
    async (event: React.DragEvent) => {
      event.preventDefault();
      event.stopPropagation();
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
      // 子流程拖拽：从流程列表拖入画布生成 flow:sub 节点（或空画布时直接打开）
      const subflowPayload =
        event.dataTransfer.getData('application/reactflow/subflow') ||
        event.dataTransfer.getData('text/plain');
      if (subflowPayload) {
        let subflowData: { uuid?: string; name?: string; filePath?: string } | null = null;
        try {
          const parsed = JSON.parse(subflowPayload);
          if (parsed && typeof parsed === 'object' && typeof parsed.uuid === 'string') {
            subflowData = parsed;
          }
        } catch {
          subflowData = null;
        }

        if (subflowData?.uuid) {
          if (!flow) {
            flowStorage
              .loadFlow(subflowData.uuid)
              .then((loadedFlow) => {
                if (loadedFlow) {
                  useFlowStore.getState().openTab(loadedFlow);
                }
              })
              .catch(console.error);
            return;
          }

          // 禁止循环引用当前流程自身
          if (flow.uuid === subflowData.uuid) {
            toast.error(ll.flow.cannot_reference_self_subflow());
            return;
          }

          const position = screenToFlowPosition({
            x: event.clientX,
            y: event.clientY,
          });

          const defaultData = createDefaultNodeData('flow:sub');
          const nodeData = {
            ...defaultData,
            type: 'flow:sub' as const,
            target_flow_id: subflowData.uuid,
            target_flow_path: subflowData.filePath || subflowData.uuid,
            inherit_all_variables: true,
            export_all_variables: true,
            variable_mappings: [],
            output_mappings: [],
          };

          const nodeCount = flow.nodes.length + 1;
          const name = subflowData.name || createDefaultNodeName('flow:sub', nodeCount);
          addNode({
            uuid: crypto.randomUUID(),
            type: 'flow:sub',
            name,
            position,
            data: nodeData as unknown as NodeData,
          });
          return;
        }
      }


      const nodeType = event.dataTransfer.getData('application/reactflow/nodeType');
      if (!nodeType || !flow) return;

      // 使用鼠标实际位置转换为流程坐标
      const position = screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });

      if (nodeType === 'entry') {
        const { activeStage } = useFlowStore.getState();
        const stageGraph = getStageGraph(flow, activeStage);
        if (stageGraph.nodes.some((n) => n.type === 'entry')) {
          toast.warning(getLL().flow.entry_node_already_exists());
          return;
        }
      }
      if (nodeType === 'http:ingress') {
        const { activeStage } = useFlowStore.getState();
        const stageGraph = getStageGraph(flow, activeStage);
        if (stageGraph.nodes.some((n) => n.type === 'http:ingress')) {
          toast.warning(getLL().flow.ingress_node_already_exists());
          return;
        }
      }

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

    if (pendingAddNodeType === 'entry') {
      const { activeStage } = useFlowStore.getState();
      const stageGraph = getStageGraph(flow, activeStage);
      if (stageGraph.nodes.some((n) => n.type === 'entry')) {
        toast.warning(getLL().flow.entry_node_already_exists());
        clearPendingAddNodeType();
        useUIStore.getState().closeNodePalette();
        return;
      }
    }
    if (pendingAddNodeType === 'http:ingress') {
      const { activeStage } = useFlowStore.getState();
      const stageGraph = getStageGraph(flow, activeStage);
      if (stageGraph.nodes.some((n) => n.type === 'http:ingress')) {
        toast.warning(getLL().flow.ingress_node_already_exists());
        clearPendingAddNodeType();
        useUIStore.getState().closeNodePalette();
        return;
      }
    }

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
    if (pendingAddNodeType !== 'entry' && pendingAddNodeType !== 'end') {
      const { activeStage, addConnection, removeConnection } = useFlowStore.getState();
      const stageGraph = getStageGraph(flow, activeStage);
      const edgeToInsert = findEdgeToInsert(position, stageGraph.nodes, stageGraph.connections);
      if (edgeToInsert) {
        insertNodeIntoConnection(newNodeUuid, edgeToInsert, addConnection, removeConnection);
        toast.success(getLL().flow.inserted_between_nodes());
      }
    }

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

  // 1. 阻止触摸板双指捏合（Pinch-to-zoom）放大到极限时穿透触发浏览器原生窗口缩放与失焦
  // 2. 窗口切换回来时自动恢复画布焦点，避免 Alt+Tab 切回后快捷键失焦
  useEffect(() => {
    const handleWheel = (e: WheelEvent) => {
      // 触摸板 pinch 缩放及 Ctrl+滚轮缩放均带有 ctrlKey
      // 当 React Flow 达到 maxZoom (1.35) 时内部 d3-zoom 会放弃调用 preventDefault，
      // 此处兜底阻止浏览器原生触发 Viewport Pinch Zoom，防止整个 Webview 窗口被拉伸失焦
      if (e.ctrlKey) {
        e.preventDefault();
      }
    };

    const handleGesture = (e: Event) => {
      // Safari / WebKit 独有的触摸板手势拦截
      e.preventDefault();
    };

    const handleWindowFocus = () => {
      // 切换窗口切回时，若焦点停留在 body 外壳，自动恢复画布 Pane 的交互焦点
      const pane = document.querySelector('.react-flow__pane') as HTMLElement | null;
      if (pane && (document.activeElement === document.body || !document.activeElement)) {
        pane.focus({ preventScroll: true });
      }
    };

    window.addEventListener('wheel', handleWheel, { passive: false });
    window.addEventListener('gesturestart', handleGesture);
    window.addEventListener('gesturechange', handleGesture);
    window.addEventListener('focus', handleWindowFocus);

    return () => {
      window.removeEventListener('wheel', handleWheel);
      window.removeEventListener('gesturestart', handleGesture);
      window.removeEventListener('gesturechange', handleGesture);
      window.removeEventListener('focus', handleWindowFocus);
    };
  }, []);
  // 监听全局 Ctrl+V 粘贴事件：
  // 1. 若剪贴板内容为 cURL 命令，在鼠标位置或视口中心自动解析生成配置好的 HTTP 节点
  // 2. 若剪贴板内容为已复制的节点配置且当前处于空白处，直接粘贴为新节点
  useEffect(() => {
    const handlePaste = (e: ClipboardEvent) => {
      const target = e.target as HTMLElement | null;
      const isInputting =
        target &&
        (target.tagName === 'INPUT' ||
          target.tagName === 'TEXTAREA' ||
          target.isContentEditable ||
          Boolean(target.closest('input, textarea, [contenteditable="true"], .cm-editor, .monaco-editor')));
      if (isInputting) return;

      const text = e.clipboardData?.getData('text');
      if (!text) return;
      const trimmed = text.trim();

      // 1. 尝试解析 cURL
      if (/^curl\s/i.test(trimmed)) {
        const httpConfig = parseCurlToHttpNodeConfig(trimmed);
        if (httpConfig) {
          e.preventDefault();
          const { flow: currentFlow, addNode: addNewNode, selectNode: setSelectNode } = useFlowStore.getState();
          if (!currentFlow) return;

          let position: { x: number; y: number };
          if (mousePosRef.current) {
            position = screenToFlowPosition(mousePosRef.current);
          } else if (canvasRef.current) {
            const rect = canvasRef.current.getBoundingClientRect();
            position = screenToFlowPosition({
              x: rect.left + rect.width / 2,
              y: rect.top + rect.height / 2,
            });
          } else {
            position = { x: 100, y: 100 };
          }

          const newUuid = crypto.randomUUID();
          const nodeCount = currentFlow.nodes.length + 1;
          const defaultName = createDefaultNodeName('http:request', nodeCount);
          const rawUrl = httpConfig.url ? String(httpConfig.url) : '';
          let name = defaultName;
          if (rawUrl) {
            try {
              const u = new URL(rawUrl);
              const method = (httpConfig.method as string || 'GET').toUpperCase();
              name = `${method} ${u.pathname === '/' ? u.hostname : u.pathname}`;
            } catch {
              name = defaultName;
            }
          }

          addNewNode({
            uuid: newUuid,
            type: 'http:request',
            name,
            position,
            data: httpConfig as unknown as NodeData,
          });

          setSelectNode(newUuid);
          useUIStore.getState().setConfigPanelOpen(true);
          useUIStore.getState().setConfigPanelMode('node');

          toast.success(getLL().flow.curl_pasted_node_created(), {
            description: rawUrl || name,
          });
          return;
        }
      }

      // 2. 检查是否有复制的节点配置
      try {
        const clipRaw = localStorage.getItem(NODE_CONFIG_CLIPBOARD_KEY);
        if (clipRaw) {
          const clipData = JSON.parse(clipRaw) as NodeConfigClipboard;
          if (clipData && clipData.nodeType && Date.now() - clipData.copiedAt < 86400000) {
            const selectedNodeId = useFlowStore.getState().selectedNodeId;
            if (!selectedNodeId) {
              e.preventDefault();
              const { flow: currentFlow, addNode: addNewNode, selectNode: setSelectNode } = useFlowStore.getState();
              if (!currentFlow) return;

              let position: { x: number; y: number };
              if (mousePosRef.current) {
                position = screenToFlowPosition(mousePosRef.current);
              } else if (canvasRef.current) {
                const rect = canvasRef.current.getBoundingClientRect();
                position = screenToFlowPosition({
                  x: rect.left + rect.width / 2,
                  y: rect.top + rect.height / 2,
                });
              } else {
                position = { x: 100, y: 100 };
              }

              const newUuid = crypto.randomUUID();
              const name = `${clipData.sourceNodeName || clipData.nodeType} (${getLL().flow.duplicate_suffix()})`;

              addNewNode({
                uuid: newUuid,
                type: clipData.nodeType as NodeType,
                name,
                position,
                data: JSON.parse(JSON.stringify(clipData.data)),
              });

              setSelectNode(newUuid);
              useUIStore.getState().setConfigPanelOpen(true);
              useUIStore.getState().setConfigPanelMode('node');
              toast.success(getLL().flow.node_pasted_created());
            }
          }
        }
      } catch {
        // ignore
      }
    };

    const handleMouseMove = (e: MouseEvent) => {
      mousePosRef.current = { x: e.clientX, y: e.clientY };
    };

    window.addEventListener('paste', handlePaste);
    window.addEventListener('mousemove', handleMouseMove);
    return () => {
      window.removeEventListener('paste', handlePaste);
      window.removeEventListener('mousemove', handleMouseMove);
    };
  }, [screenToFlowPosition]);


  // 没有流程时显示空状态
  if (!flow) {
    return (
      <div
        className="w-full h-full flex items-center justify-center bg-background"
        data-guide="canvas-area"
        onDragOver={onDragOver}
        onDrop={onDrop}
      >
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

          {/* Keyboard shortcuts & usage tips hint */}
          <EmptyCanvasTips />
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

      {/* 生命周期阶段切换栏 */}
      <StageTabBar />

      {/* 调试面包屑导航 */}
      <DebugBreadcrumbs />

      {/* 画布底部节点选择工具条 */}
      <NodePalette />
      
      <FlowSurface
        containerRef={canvasRef}
        onDragOver={onDragOver}
        onDrop={onDrop}
        onNodeContextMenu={onNodeContextMenu}
        onSelectionContextMenu={onSelectionContextMenu}
        onCloseContextMenu={() => setContextMenu(null)}
      />
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

    </div>
  );
}

/**
 * 流程画布组件（带 ReactFlowProvider）
 */
export const FlowCanvas = memo(function FlowCanvas() {
  return (
    <ReactFlowProvider>
      <FlowCanvasInner />
    </ReactFlowProvider>
  );
});
