/**
 * NodeContextMenu - 节点右键上下文菜单
 * 在鼠标右键点击节点时显示，支持复制、重命名、删除、配置复制/粘贴操作
 */
import { useEffect, useRef, useState } from 'react';
import { Copy, Trash2, Pencil, ClipboardCopy, ClipboardPaste, Play } from 'lucide-react';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import { executeSingleNode } from '@/services/tauri/flow';
import { useFlowStore, useExecutionStore } from '@/stores';
import { getActiveVariables } from '@/types/flow';
import type { FlowExecutionReport } from '@/types';

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
  const [testing, setTesting] = useState(false);

  // 构建菜单项
  const LL = getLL();

  const handleTestNode = async () => {
    setTesting(true);
    try {
      const flow = useFlowStore.getState().flow;
      if (!flow) {
        toast.error('No flow loaded');
        return;
      }
      const node = flow.nodes.find((n) => n.uuid === nodeId);
      if (!node) {
        toast.error('Node not found in current flow');
        return;
      }
      // Pass flow's active variables so {{variable}} references in node config are resolved
      const flowVariables = getActiveVariables(flow);
      const nodeReport = await executeSingleNode(node, flowVariables);

      // Build a lightweight flow report for the dialog (no persistence)
      const now = Date.now();
      const flowReport: FlowExecutionReport = {
        uuid: `test-${nodeId}-${now}`,
        flow_uuid: flow.uuid,
        flow_name: `Test: ${nodeName}`,
        start_time: now - nodeReport.duration_ms,
        end_time: now,
        duration_ms: nodeReport.duration_ms,
        total_nodes: 1,
        executed_count: nodeReport.status === 'success' ? 1 : 0,
        status: nodeReport.status,
        node_reports: [nodeReport],
        statistics: {
          success_count: nodeReport.status === 'success' ? 1 : 0,
          failed_count: nodeReport.status === 'failed' ? 1 : 0,
          avg_node_duration_ms: nodeReport.duration_ms,
          max_node_duration_ms: nodeReport.duration_ms,
          min_node_duration_ms: nodeReport.duration_ms,
        },
        execution_mode: 'normal',
      };

      // Show in report dialog — no save, no history
      useExecutionStore.getState().setReport(flowReport);
      useExecutionStore.getState().openReportDialog(flowReport);
    } catch (error) {
      const msg = error instanceof Error ? error.message : String(error);
      toast.error(`Test failed: ${msg}`);
    } finally {
      setTesting(false);
      onClose();
    }
  };

  const menuItems: MenuItem[] = [
    {
      key: 'copy',
      label: LL.flow.context_copy_node(),
      icon: <Copy className="w-4 h-4" />,
      action: () => {
        onCopy(nodeId);
        onClose();
      },
    },
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
    {
      key: 'test',
      label: LL.flow.context_test_node(),
      icon: <Play className="w-4 h-4" />,
      disabled: testing,
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
    // entry 节点不显示删除选项
    ...(nodeType !== 'entry'
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
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    // 使用 setTimeout 避免当前右键事件立即触发关闭
    const timer = setTimeout(() => {
      document.addEventListener('mousedown', handleClickOutside);
      document.addEventListener('keydown', handleKeyDown);
    }, 0);

    return () => {
      clearTimeout(timer);
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [onClose]);

  return (
    <div
      ref={menuRef}
      className={cn(
        'fixed z-50 min-w-[160px]',
        'bg-background/95 backdrop-blur-xl',
        'rounded-lg shadow-xl',
        'border border-border/50 dark:border-white/20',
        'py-1',
        'animate-in fade-in-0 zoom-in-95 duration-100',
        'select-none',
      )}
      style={{ left: x, top: y }}
    >
      {/* 节点名称标题 */}
      <div className="px-3 py-1.5 text-xs text-muted-foreground border-b border-border/40 dark:border-white/10 truncate max-w-[200px]" title={nodeName}>
        {nodeName}
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
