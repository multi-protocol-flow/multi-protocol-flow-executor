/**
 * NodePalette - 画布底部浮动节点工具条
 * Figma/Excalidraw 风格，分组展示节点类型，支持拖拽到画布和点击添加
 *
 * 动态从后端 NodeRegistry 获取节点分组，回退到静态分组。
 */
import { type LucideIcon } from 'lucide-react';
import {
  Globe,
  GitBranch,
  Wifi,
  Send,
  Plug,
  CheckCircle,
  Database,
  CirclePlay,
  RotateCcw,
  Code,
  Wrench,
  Play,
  Network,
  Puzzle,
  Cable,
  PhoneOutgoing,
  PhoneOff,
  FileSearch,
  Unplug,
} from 'lucide-react';
import * as Popover from '@radix-ui/react-popover';
import { useFlowStore } from '@/stores';
import type { NodeType } from '@/types';
import type { NodeMetadata } from '@/types/nodeRegistry';
import {
  getNodeMetadata,
  groupByCategory,
  CATEGORY_NAMES,
  getCategoryOrder,
} from '@/lib/NodeRegistry';
import { ICON_MAP as NODE_TYPE_ICON_MAP } from '@/lib/nodeMeta';
import { cn } from '@/lib/utils';
import { useEffect, useMemo, useState } from 'react';

/** 节点条目定义 */
export interface NodeItem {
  type: NodeType;
  name: string;
  icon: LucideIcon;
}

/** 节点分组定义 */
export interface NodeGroup {
  /** 分组键 */
  key: string;
  /** 分组显示名称 */
  label: string;
  /** 分组图标 */
  icon: LucideIcon;
  /** 分组下的节点条目；仅 1 个条目时为直接按钮，多个时弹出 Popover */
  items: NodeItem[];
}

/** 后端 category 对应的图标 */
export const CATEGORY_ICONS: Record<string, LucideIcon> = {
  logic: GitBranch,
  http: Globe,
  websocket: Wifi,
  database: Database,
  tools: Wrench,
  script: Code,
  assertion: CheckCircle,
  grpc: Network,
  mcp: Puzzle,
};

/** 后端 icon 字段到 lucide-react 组件的映射 */
export const ICON_MAP: Record<string, LucideIcon> = {
  play: Play,
  globe: Globe,
  'git-branch': GitBranch,
  code: Code,
  'check-circle': CheckCircle,
  wifi: Wifi,
  send: Send,
  plug: Plug,
  database: Database,
  'circle-play': CirclePlay,
  check: CheckCircle,
  'rotate-ccw': RotateCcw,
  wrench: Wrench,
  network: Network,
  puzzle: Puzzle,
  cable: Cable,
  'phone-outgoing': PhoneOutgoing,
  'phone-off': PhoneOff,
  'file-search': FileSearch,
  'file-text': FileSearch,
  unplug: Unplug,
  'x-circle': Unplug,
};

/** 静态回退分组数据 */
export const FALLBACK_NODE_GROUPS: NodeGroup[] = [
  {
    key: 'http',
    label: 'HTTP',
    icon: Globe,
    items: [{ type: 'http', name: 'HTTP 请求', icon: Globe }],
  },
  {
    key: 'tools',
    label: 'Tool',
    icon: Wrench,
    items: [
      { type: 'conditional', name: '条件判断', icon: GitBranch },
      { type: 'script', name: '脚本', icon: Code },
      { type: 'assertion', name: '断言', icon: CheckCircle },
    ],
  },
  {
    key: 'websocket',
    label: 'WS',
    icon: Wifi,
    items: [
      { type: 'ws_connect', name: 'WS 连接', icon: Wifi },
      { type: 'ws_send_collect', name: 'WS 发送采集', icon: Send },
      { type: 'ws_close', name: 'WS 关闭', icon: Plug },
    ],
  },
  {
    key: 'database',
    label: 'DB',
    icon: Database,
    items: [
      { type: 'db_connect', name: 'DB 连接', icon: Database },
      { type: 'db_query', name: 'DB 查询', icon: Database },
      { type: 'db_begin', name: 'DB 开始事务', icon: CirclePlay },
      { type: 'db_commit', name: 'DB 提交', icon: CheckCircle },
      { type: 'db_rollback', name: 'DB 回滚', icon: RotateCcw },
      { type: 'db_close', name: 'DB 关闭', icon: Plug },
    ],
  },
];

/**
 * 从后端元数据构建分组
 *
 * 将后端 NodeMetadata 转换为 NodePalette 所需的 NodeGroup 结构。
 * logic 分类的节点会合并到 tools 分类。
 */
export function buildGroupsFromMetadata(metadata: NodeMetadata[]): NodeGroup[] {
  const grouped = groupByCategory(metadata);

  // 将 logic 分类合并到 tools 分类
  if (grouped['logic']) {
    if (!grouped['tools']) grouped['tools'] = [];
    grouped['tools'] = [...grouped['logic'], ...grouped['tools']];
    delete grouped['logic'];
  }

  const groups: NodeGroup[] = Object.entries(grouped)
    .sort(([a], [b]) => getCategoryOrder(a) - getCategoryOrder(b))
    .map(([category, items]) => {
      // 排除 entry 类型（入口节点自动创建，不需要手动添加）
      const filtered = items.filter((m) => m.type_id !== 'entry');

      if (filtered.length === 0) return null;

      return {
        key: category,
        label: CATEGORY_NAMES[category] ?? category,
        icon: CATEGORY_ICONS[category] ?? Wrench,
        items: filtered.map((m) => ({
          type: m.type_id as NodeType,
          name: m.display_name,
          // 优先使用 nodeMeta.ts 的 type_id 映射（与画布节点一致），回退到后端 icon 字段映射
          icon: NODE_TYPE_ICON_MAP[m.type_id]?.icon ?? ICON_MAP[m.icon] ?? Wrench,
        })),
      };
    })
    .filter((g): g is NodeGroup => g !== null && g.items.length > 0);

  return groups.length > 0 ? groups : FALLBACK_NODE_GROUPS;
}

/**
 * 节点工具条组件
 * 悬浮在画布底部居中，分组展示可用节点类型
 */
export function NodePalette() {
  const setPendingAddNodeType = useFlowStore((s) => s.setPendingAddNodeType);
  const flow = useFlowStore((s) => s.flow);

  // 动态元数据状态
  const [metadata, setMetadata] = useState<NodeMetadata[]>([]);

  // 挂载时获取元数据
  useEffect(() => {
    getNodeMetadata().then(setMetadata);
  }, []);

  // 从元数据构建分组（回退到静态）
  const nodeGroups = useMemo(
    () => buildGroupsFromMetadata(metadata),
    [metadata]
  );

  /** 拖拽开始：设置 dataTransfer */
  const handleDragStart = (e: React.DragEvent, type: NodeType) => {
    e.dataTransfer.setData('application/reactflow/nodeType', type);
    e.dataTransfer.effectAllowed = 'move';
  };

  /** 点击添加节点：设置待添加类型，由 FlowCanvas 在视口中心处理 */
  const handleClick = (type: NodeType) => {
    if (!flow) return;
    setPendingAddNodeType(type);
  };

  return (
    <div
      className={cn(
        'absolute bottom-6 left-1/2 -translate-x-1/2 z-10',
        'flex items-center gap-0',
        'bg-background/90 backdrop-blur-sm rounded-2xl px-1.5 py-1',
        'shadow-lg border border-border/50',
      )}
    >
      {nodeGroups.map((group, groupIndex) => {
        const isDirectAction = group.items.length === 1;
        const item = group.items[0];
        const GroupIcon = group.icon;
        const Separator = groupIndex > 0 && (
          <div className="w-px h-8 bg-border mx-0.5" />
        );

        return (
          <div key={group.key} className="flex items-center">
            {Separator}

            {isDirectAction ? (
              /* 单项分组：直接按钮 */
              <button
                type="button"
                draggable
                onDragStart={(e) => handleDragStart(e, item.type)}
                onClick={() => handleClick(item.type)}
                title={item.name}
                className={cn(
                  'flex flex-col items-center justify-center',
                  'w-12 h-12 rounded-lg',
                  'text-foreground',
                  'hover:bg-accent hover:scale-105',
                  'active:scale-95',
                  'transition-all duration-150 cursor-pointer',
                  'select-none',
                )}
              >
                <GroupIcon className="w-5 h-5" />
                <span className="text-[10px] text-muted-foreground mt-0.5 leading-none">
                  {group.label}
                </span>
              </button>
            ) : (
              /* 多项分组：Popover 弹出 */
              <Popover.Root>
                <Popover.Trigger asChild>
                  <button
                    type="button"
                    title={group.label}
                    className={cn(
                      'flex flex-col items-center justify-center',
                      'w-12 h-12 rounded-lg',
                      'text-foreground',
                      'hover:bg-accent hover:scale-105',
                      'active:scale-95',
                      'transition-all duration-150 cursor-pointer',
                      'select-none',
                      'data-[state=open]:bg-accent',
                    )}
                  >
                    <GroupIcon className="w-5 h-5" />
                    <span className="text-[10px] text-muted-foreground mt-0.5 leading-none">
                      {group.label}
                    </span>
                  </button>
                </Popover.Trigger>

                <Popover.Portal>
                  <Popover.Content
                    side="top"
                    sideOffset={8}
                    align="center"
                    className={cn(
                      'bg-background rounded-lg shadow-xl border border-border',
                      'p-1 min-w-[160px]',
                      'animate-in fade-in-0 zoom-in-95',
                      'data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95',
                      'z-50',
                    )}
                  >
                    {group.items.map((nodeItem) => {
                      const ItemIcon = nodeItem.icon;
                      return (
                        <Popover.Close asChild key={nodeItem.type}>
                          <button
                            type="button"
                            draggable
                            onDragStart={(e) => handleDragStart(e, nodeItem.type)}
                            onClick={() => handleClick(nodeItem.type)}
                            title={nodeItem.name}
                            className={cn(
                              'w-full flex items-center gap-2.5 px-3 py-2 rounded-md',
                              'text-sm text-foreground',
                              'hover:bg-accent',
                              'transition-colors duration-100 cursor-pointer',
                              'select-none',
                            )}
                          >
                            <ItemIcon className="w-4 h-4 text-muted-foreground shrink-0" />
                            <span>{nodeItem.name}</span>
                          </button>
                        </Popover.Close>
                      );
                    })}
                    <Popover.Arrow className="fill-background drop-shadow-sm" />
                  </Popover.Content>
                </Popover.Portal>
              </Popover.Root>
            )}
          </div>
        );
      })}
    </div>
  );
}
