/**
 * NodeContextMenu - 节点右键上下文菜单
 * 在鼠标右键点击节点时显示，支持复制、重命名、删除、配置复制/粘贴操作
 */
import { useEffect, useRef } from 'react';
import { useReactFlow } from '@xyflow/react';
import { Copy, Trash2, Pencil, ClipboardCopy, ClipboardPaste, Play, RotateCcw, Ban, CheckCircle, Terminal } from 'lucide-react';
import { generateCurlCommand, type CurlExportOptions } from '@/lib/curlExporter';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import { startInteractiveSliceExecution } from '@/services/tauri';
import { useFlowStore, useDebugStore, getStageGraph } from '@/stores';
import { isEntryNodeType } from '@/types';
export interface NodeContextMenuProps {
  /** 菜单显示位置 (屏幕坐标) */
  x: number;
  y: number;
  /** 节点 ID */
  nodeId: string;
  /** 节点类型 (用于判断 entry 节点) */
  nodeType: string;
  /** 节点名称 (用于显示) */
  nodeName: string;
  /** 关闭菜单回调 */
  onClose: () => void;
  /** 复制回调 */
  onCopy: (nodeId: string) => void;
  /** 删除回调 */
  onDelete: (nodeId: string) => void;
  /** 重命名回调 */
  onRename: (nodeId: string) => void;
  /** 复制节点配置回调 */
  onCopyConfig: (nodeId: string) => void;
  /** 粘贴节点配置回调 */
  onPasteConfig: (nodeId: string) => void;
  /** 是否有可粘贴的兼容配置 */
  hasClipboardConfig: boolean;
}

/** 菜单项定义 */
interface MenuItem {
  /** 唯一标识 */
  key: string;
  /** 显示文本 */
  label: string;
  /** 图标组件 */
  icon: React.ReactNode;
  /** 点击回调 */
  action: () => void;
  /** 是否为危险操作 */
  danger?: boolean;
  /** 是否禁用 */
  disabled?: boolean;
}

/**
 * 节点右键菜单组件
 */
export function NodeContextMenu({
  x,
  y,
  nodeId,
  nodeType,
  nodeName,
  onClose,
  onCopy,
  onDelete,
  onRename,
  onCopyConfig,
  onPasteConfig,
  hasClipboardConfig,
}: NodeContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const { getNodes } = useReactFlow();

  const isEntryType = isEntryNodeType(nodeType);
  const currentFlow = useFlowStore.getState().flow;
  const activeStage = useFlowStore.getState().activeStage;
  const stageNodes = currentFlow ? getStageGraph(currentFlow, activeStage).nodes : [];
  const hasAlternativeEntry = stageNodes.some(
    (n) => n.uuid !== nodeId && isEntryNodeType(n.type)
  );
  // 单例入口节点（entry / http:ingress）仅在当前阶段存在其他可用入口时允许删除，避免删至 0 个入口
  const canDeleteNode = !isEntryType || hasAlternativeEntry;
  const targetNode = stageNodes.find((n) => n.uuid === nodeId);
  const isNodeDisabled = Boolean(targetNode?.disabled || targetNode?.on_error === 'bypass');


  // 构建菜单项
  const LL = getLL();
  const isDebugPaused = useDebugStore((state) => state.status === 'paused');
  const executionStack = useDebugStore((state) => state.executionStack);
  const isNodeInDebugStack = isDebugPaused && executionStack.some((e) => e.node_uuid === nodeId);

  const selectedNodes = getNodes().filter((n) => n.selected);
  const isMultiSelected = selectedNodes.length > 1 && selectedNodes.some((n) => n.id === nodeId);

  const handleTestNode = async () => {
    onClose();
    const flow = useFlowStore.getState().flow;
    if (!flow) {
      toast.error('No flow loaded');
      return;
    }
    await startInteractiveSliceExecution(flow, [nodeId]);
  };

  const handleRunSelectedNodes = async () => {
    onClose();
    const flow = useFlowStore.getState().flow;
    if (!flow) {
      toast.error('No flow loaded');
      return;
    }
    await startInteractiveSliceExecution(
      flow,
      selectedNodes.map((n) => n.id)
    );
  };
  const menuItems: MenuItem[] = [
    ...(isNodeInDebugStack
      ? [
          {
            key: 'run-from-snapshot',
            label: LL.flow.context_run_from_snapshot(),
            icon: <RotateCcw className="w-4 h-4 text-teal-500" />,
            action: () => {
              useDebugStore.getState().runFromSnapshot(nodeId);
              onClose();
            },
          },
        ]
      : []),
    // entry 与 http:ingress 单例节点不显示创建副本
    ...(!isEntryType
      ? [
          {
            key: 'duplicate',
            label: LL.flow.context_duplicate_node(),
            icon: <Copy className="w-4 h-4" />,
            action: () => {
              onCopy(nodeId);
              onClose();
            },
          },
        ]
      : []),
    ...(!isEntryType
      ? [
          {
            key: 'toggle-disabled',
            label: isNodeDisabled ? LL.flow.enable_node() : LL.flow.bypass_node(),
            icon: isNodeDisabled ? <CheckCircle className="w-4 h-4 text-emerald-500" /> : <Ban className="w-4 h-4 text-amber-500" />,
            action: () => {
              const willDisable = !isNodeDisabled;
              useFlowStore.getState().updateNode(nodeId, {
                disabled: willDisable,
                on_error: willDisable
                  ? 'bypass'
                  : (targetNode?.on_error === 'bypass' ? 'route_to_false' : targetNode?.on_error),
              });
              onClose();
            },
          },
        ]
      : []),
    ...(nodeType === 'http:request' && targetNode
      ? [
          {
            key: 'copy-curl',
            label: LL.panels.http.curlCopyButton(),
            icon: <Terminal className="w-4 h-4 text-sky-500" />,
            action: () => {
              const config = targetNode.data as unknown as Record<string, unknown>;
              const url = String(config.url || '');
              const method = String(config.method || 'GET').toUpperCase();
              const headersObj = (config.headers as Record<string, string>) || {};
              const headersArr = Object.entries(headersObj).map(([key, value]) => ({ key, value }));
              const cmd = generateCurlCommand({
                url,
                method,
                headers: headersArr,
                bodyContent: config.body ? String(config.body) : undefined,
                bodyType: (config.body_type as CurlExportOptions['bodyType']) || (config.body ? 'body' : 'none'),
                contentType: (config.content_type as CurlExportOptions['contentType']) || 'json',
              });
              navigator.clipboard.writeText(cmd).then(() => {
                toast.success(LL.panels.http.curlCopySuccess());
              }).catch(() => {
                toast.error(LL.panels.http.curlCopyFailed());
              });
              onClose();
            },
          },
        ]
      : []),
    {
      key: 'copy-config',
      label: LL.flow.context_copy_config(),
      icon: <ClipboardCopy className="w-4 h-4" />,
      action: () => {
        onCopyConfig(nodeId);
        onClose();
      },
    },
    {
      key: 'paste-config',
      label: LL.flow.context_paste_config(),
      icon: <ClipboardPaste className="w-4 h-4" />,
      disabled: !hasClipboardConfig,
      action: () => {
        onPasteConfig(nodeId);
        onClose();
      },
    },
    ...(isMultiSelected
      ? [
          {
            key: 'run-selected',
            label: LL.flow.context_run_selected_nodes({ count: selectedNodes.length }),
            icon: <Play className="w-4 h-4 text-purple-400" />,
            action: () => {
              handleRunSelectedNodes();
            },
          },
        ]
      : []),
    {
      key: 'test',
      label: LL.flow.context_test_node(),
      icon: <Play className="w-4 h-4" />,
      action: () => {
        handleTestNode();
      },
    },
    {
      key: 'rename',
      label: LL.flow.context_rename(),
      icon: <Pencil className="w-4 h-4" />,
      action: () => {
        onRename(nodeId);
        onClose();
      },
    },
    // entry 节点默认不显示删除选项，但若画布中已存在 http:ingress 等替代入口则允许删除
    ...(canDeleteNode
      ? [
          {
            key: 'delete',
            label: LL.flow.context_delete_node(),
            icon: <Trash2 className="w-4 h-4" />,
            action: () => {
              onDelete(nodeId);
              onClose();
            },
            danger: true,
          },
        ]
      : []),
  ];

  // 点击外部关闭菜单
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent | PointerEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as globalThis.Node)) {
        onClose();
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    // 使用 capture 阶段捕获 pointerdown 与 mousedown，防止被 ReactFlow 等子组件的 stopPropagation 拦截
    const timer = setTimeout(() => {
      document.addEventListener('pointerdown', handleClickOutside, { capture: true });
      document.addEventListener('mousedown', handleClickOutside, { capture: true });
      document.addEventListener('contextmenu', handleClickOutside, { capture: true });
      document.addEventListener('keydown', handleKeyDown, { capture: true });
    }, 0);

    return () => {
      clearTimeout(timer);
      document.removeEventListener('pointerdown', handleClickOutside, { capture: true });
      document.removeEventListener('mousedown', handleClickOutside, { capture: true });
      document.removeEventListener('contextmenu', handleClickOutside, { capture: true });
      document.removeEventListener('keydown', handleKeyDown, { capture: true });
    };
  }, [onClose]);
  return (
    <div
      ref={menuRef}
      className={cn(
        'fixed z-50 min-w-[160px]',
        'bg-background',
        'rounded-lg shadow-xl',
        'border border-border/50 dark:border-white/20',
        'py-1',
        'animate-in fade-in-0 zoom-in-95 duration-100',
        'select-none',
      )}
      style={{ left: x, top: y }}
    >
      {/* 节点名称标题 */}
      <div
        className="px-3 py-1.5 text-xs text-muted-foreground border-b border-border/40 dark:border-white/10 truncate max-w-[200px]"
        title={isMultiSelected ? LL.flow.selected_nodes_count({ count: selectedNodes.length }) : nodeName}
      >
        {isMultiSelected ? LL.flow.selected_nodes_count({ count: selectedNodes.length }) : nodeName}
      </div>

      {/* 菜单项列表 */}
      <div className="py-0.5">
        {menuItems.map((item) => (
          <button
            key={item.key}
            type="button"
            onClick={item.action}
            disabled={item.disabled}
            className={cn(
              'flex items-center gap-2.5 w-full px-3 py-1.5',
              'text-sm transition-colors duration-100',
              'outline-none',
              item.disabled
                ? 'text-muted-foreground/40 cursor-not-allowed'
                : 'cursor-pointer',
              !item.disabled && item.danger
                ? 'text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-950/30'
                : !item.disabled && 'text-foreground hover:bg-accent',
            )}
          >
            <span className={cn(
              'flex-shrink-0',
              item.danger
                ? 'text-red-500 dark:text-red-400'
                : 'text-muted-foreground',
            )}>
              {item.icon}
            </span>
            {item.label}
          </button>
        ))}
      </div>
    </div>
  );
}
