/**
 * NodeLibrary - 节点库面板
 * 显示可用的节点类型，支持点击添加或拖拽到画布
 *
 * 动态从后端 NodeRegistry 获取节点元数据，按分类分组展示。
 */
import { GripVertical, Sparkles, Wrench } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { useFlowStore } from '@/stores';
import type { NodeType } from '@/types';
import type { NodeMetadata } from '@/types/nodeRegistry';
import {
  getNodeMetadata,
  groupByCategory,
  CATEGORY_NAMES,
  getCategoryOrder,
} from '@/lib/NodeRegistry';
import { ICON_MAP } from '@/lib/nodeMeta';
import { cn } from '@/lib/utils';
import { useEffect, useMemo, useState } from 'react';

/** 节点描述兜底映射 */
const DESCRIPTION_MAP: Record<string, string> = {
  http: '发送 HTTP 请求',
  conditional: '根据条件分支',
  ws_connect: '建立 WebSocket 连接',
  ws_send_collect: '发送 WebSocket 消息并采集响应',
  ws_close: '关闭 WebSocket 连接',
  assertion: '验证执行结果是否符合预期',
  db_connect: '建立数据库连接',
  db_query: '执行数据库查询',
  db_begin: '开始数据库事务',
  db_commit: '提交数据库事务',
  db_rollback: '回滚数据库事务',
  db_close: '关闭数据库连接',
  script: '执行自定义 JavaScript 代码',
};

/** 节点条目 */
interface NodeItem {
  type: NodeType;
  name: string;
  description: string;
  icon: LucideIcon;
}

/** 节点分组 */
interface NodeGroup {
  key: string;
  label: string;
  items: NodeItem[];
}

/**
 * 从后端元数据构建分组
 */
function buildGroupsFromMetadata(metadata: NodeMetadata[]): NodeGroup[] {
  const filtered = metadata.filter((m) => m.type_id !== 'entry');
  const grouped = groupByCategory(filtered);

  const groups: NodeGroup[] = Object.entries(grouped)
    .sort(([a], [b]) => getCategoryOrder(a) - getCategoryOrder(b))
    .map(([category, items]) => ({
      key: category,
      label: CATEGORY_NAMES[category] ?? category,
      items: items.map((m) => ({
        type: m.type_id as NodeType,
        name: m.display_name,
        description: DESCRIPTION_MAP[m.type_id] ?? m.display_name,
        icon: ICON_MAP[m.type_id]?.icon ?? Wrench,
      })),
    }))
    .filter((g) => g.items.length > 0);

  return groups;
}

export function NodeLibrary({ showHeader = true }: { showHeader?: boolean }) {
  const { setPendingAddNodeType, flow } = useFlowStore();
  const [metadata, setMetadata] = useState<NodeMetadata[]>([]);

  useEffect(() => {
    getNodeMetadata().then(setMetadata);
  }, []);

  const nodeGroups = useMemo(
    () => buildGroupsFromMetadata(metadata),
    [metadata]
  );

  const handleDragStart = (e: React.DragEvent, type: NodeType) => {
    e.dataTransfer.setData('application/reactflow/nodeType', type);
    e.dataTransfer.effectAllowed = 'move';
  };

  // 点击添加节点 - 设置待添加类型，由 FlowCanvas 在视口中心处理
  const handleAddNode = (type: NodeType) => {
    if (!flow) return;
    setPendingAddNodeType(type);
  };

  return (
    <div className="flex flex-col h-full">
      {/* 标题栏 */}
      {showHeader && (
        <div className="px-3 py-2 border-b border-border">
          <span className="text-sm font-medium text-foreground">节点库</span>
        </div>
      )}

      {/* 节点列表 */}
      <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden scrollbar-thin p-2 space-y-3">
        {nodeGroups.map((group) => (
          <div key={group.key}>
            {/* 分类标题 */}
            <div className="px-1 mb-1">
              <span className="text-xs font-medium text-muted-foreground">
                {group.label}
              </span>
            </div>
            {/* 分类下节点 */}
            <div className="space-y-1.5">
              {group.items.map((nodeType) => {
                const Icon = nodeType.icon;
                return (
                  <button
                    type="button"
                    key={nodeType.type}
                    onClick={() => handleAddNode(nodeType.type)}
                    draggable
                    onDragStart={(e) => handleDragStart(e, nodeType.type)}
                    className={cn(
                      'w-full flex items-center gap-3 px-3 h-14 rounded-lg',
                      'bg-background border border-border',
                      'hover:bg-accent transition-all duration-150 cursor-pointer',
                      'group text-left'
                    )}
                  >
                    {/* 节点图标 */}
                    <div
                      className={cn(
                        'w-9 h-9 rounded-lg flex items-center justify-center shrink-0',
                        'bg-background border border-border'
                      )}
                    >
                      <Icon className="w-5 h-5 text-foreground" />
                    </div>

                    {/* 节点信息 */}
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium text-foreground truncate">
                        {nodeType.name}
                      </p>
                      <p className="text-xs text-muted-foreground truncate">
                        {nodeType.description}
                      </p>
                    </div>

                    {/* 拖拽手柄 */}
                    <GripVertical
                      className={cn(
                        'w-4 h-4 text-muted-foreground shrink-0',
                        'opacity-0 group-hover:opacity-100 transition-opacity'
                      )}
                    />
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </div>

      {/* 底部提示 */}
      <div className="px-3 py-2.5 border-t border-border bg-muted">
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Sparkles className="w-3.5 h-3.5 text-amber-500" />
          <span>点击添加节点或拖拽到画布</span>
        </div>
      </div>
    </div>
  );
}
