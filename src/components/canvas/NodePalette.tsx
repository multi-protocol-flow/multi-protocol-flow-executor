/**
 * NodePalette - 画布底部浮动节点工具条（二级分组 UI）
 *
 * 收起状态：底部只显示 4 个大类别按钮（协议/数据/工具/邮件）
 * 展开状态：向上弹出面板，搜索框 + Tab 二级分类 + 节点网格
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
  Mail,
  Search,
  Download,
  Eye,
  EyeOff,
  Layers,
  Settings,
  Ear,
  RadioTower,
  Shuffle,
  Trash2,
  List,
  Trash,
  Link2,
  Inbox,
  Braces,
  XCircle,
  Timer,
  Copy,
  FolderPlus,
  FileText,
  MessageSquare,
  ArrowDownLeft,
} from 'lucide-react';
import { useFlowStore } from '@/stores';
import type { NodeType } from '@/types';
import type { NodeMetadata } from '@/types/nodeRegistry';
import {
  getNodeMetadata,
  groupByCategory,
  CATEGORY_NAMES,
  getCategoryOrder,
  getNodeTypeOrder,
} from '@/lib/NodeRegistry';
import { ICON_MAP as NODE_TYPE_ICON_MAP } from '@/lib/nodeMeta';
import { cn } from '@/lib/utils';
import { useEffect, useMemo, useState, useRef, useCallback } from 'react';

// ============================================================================
// 导出的类型和常量（供外部引用，不可删除）
// ============================================================================

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
  mqtt: Wifi,
  kafka: Send,
  sse: Globe,
  graphql: Globe,
  tcp: Cable,
  udp: Send,
  amqp: Plug,
  email: Mail,
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
  'file-text': FileText,
  'file-code': Code,
  unplug: Unplug,
  'x-circle': XCircle,
  download: Download,
  search: Search,
  eye: Eye,
  'eye-off': EyeOff,
  mail: Mail,
  layers: Layers,
  settings: Settings,
  ear: Ear,
  'radio-tower': RadioTower,
  shuffle: Shuffle,
  'trash-2': Trash2,
  list: List,
  trash: Trash,
  link: Link2,
  inbox: Inbox,
  braces: Braces,
  timer: Timer,
  'arrow-down-to-line': ArrowDownLeft,
  undo: RotateCcw,
  x: Unplug,
  'message-square': MessageSquare,
  copy: Copy,
  'folder-plus': FolderPlus,
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
    key: 'grpc',
    label: 'gRPC',
    icon: Network,
    items: [
      { type: 'grpc_connect', name: 'gRPC 连接', icon: Cable },
      { type: 'grpc_call', name: 'gRPC 调用', icon: PhoneOutgoing },
      { type: 'grpc_close', name: 'gRPC 关闭', icon: PhoneOff },
    ],
  },
  {
    key: 'database',
    label: 'DB',
    icon: Database,
    items: [
      { type: 'db_connect', name: 'DB 连接', icon: Database },
      { type: 'db_begin', name: 'DB 开始事务', icon: CirclePlay },
      { type: 'db_query', name: 'DB 查询', icon: Database },
      { type: 'db_commit', name: 'DB 提交', icon: CheckCircle },
      { type: 'db_rollback', name: 'DB 回滚', icon: RotateCcw },
      { type: 'db_close', name: 'DB 关闭', icon: Plug },
    ],
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

      // 分组内排序: 连接 → 调用 → 关闭
      filtered.sort((a, b) => getNodeTypeOrder(a.type_id) - getNodeTypeOrder(b.type_id));

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

// ============================================================================
// 一级超级分类定义
// ============================================================================

/** 超级分类定义 */
interface SuperCategory {
  /** 分类键 */
  key: string;
  /** 显示名称 */
  label: string;
  /** 图标 */
  icon: LucideIcon;
  /** 包含的二级分类 key */
  categories: string[];
}

/** 将 16 个二级分类映射到 4 个一级分类 */
const SUPER_CATEGORIES: SuperCategory[] = [
  {
    key: 'protocol',
    label: '协议',
    icon: Globe,
    categories: ['http', 'websocket', 'grpc', 'mcp', 'mqtt', 'kafka', 'sse', 'graphql', 'tcp', 'udp'],
  },
  {
    key: 'data',
    label: '数据',
    icon: Database,
    categories: ['database', 'amqp'],
  },
  {
    key: 'tools',
    label: '工具',
    icon: Wrench,
    categories: ['tools', 'script', 'assertion'],
  },
  {
    key: 'email',
    label: '邮件',
    icon: Mail,
    categories: ['email'],
  },
];

// ============================================================================
// 子组件
// ============================================================================

/** 节点网格卡片 */
function NodeCard({
  item,
  onDragStart,
  onClick,
}: {
  item: NodeItem;
  onDragStart: (e: React.DragEvent, type: NodeType) => void;
  onClick: (type: NodeType) => void;
}) {
  const Icon = item.icon;
  return (
    <button
      type="button"
      draggable
      onDragStart={(e) => onDragStart(e, item.type)}
      onClick={() => onClick(item.type)}
      title={item.name}
      className={cn(
        'flex flex-col items-center justify-center gap-1.5',
        'px-2 py-3 rounded-xl',
        'text-foreground',
        'hover:bg-accent hover:scale-[1.03]',
        'active:scale-95',
        'transition-all duration-150 cursor-pointer',
        'select-none',
        'border border-transparent hover:border-border/50',
      )}
    >
      <Icon className="w-5 h-5 text-muted-foreground shrink-0" />
      <span className="text-[11px] text-muted-foreground leading-tight text-center line-clamp-2">
        {item.name}
      </span>
    </button>
  );
}

// ============================================================================
// 主组件
// ============================================================================

/**
 * 节点工具条组件
 * 悬浮在画布底部居中，二级分组展示可用节点类型
 */
export function NodePalette() {
  const setPendingAddNodeType = useFlowStore((s) => s.setPendingAddNodeType);
  const flow = useFlowStore((s) => s.flow);

  // 动态元数据状态
  const [metadata, setMetadata] = useState<NodeMetadata[]>([]);
  // 展开状态：当前激活的超级分类 key，null 表示收起
  const [activeSuperKey, setActiveSuperKey] = useState<string | null>(null);
  // 当前激活的二级分类 Tab key
  const [activeTabKey, setActiveTabKey] = useState<string | null>(null);
  // 搜索关键词
  const [searchQuery, setSearchQuery] = useState('');
  // 面板 ref，用于点击外部关闭
  const panelRef = useRef<HTMLDivElement>(null);

  // 挂载时获取元数据
  useEffect(() => {
    getNodeMetadata().then(setMetadata);
  }, []);

  // 从元数据构建分组（回退到静态）
  const nodeGroups = useMemo(
    () => buildGroupsFromMetadata(metadata),
    [metadata]
  );

  // 按超级分类组织分组
  const groupedBySuper = useMemo(() => {
    const map: Record<string, NodeGroup[]> = {};
    for (const sc of SUPER_CATEGORIES) {
      map[sc.key] = [];
    }
    // 未被超级分类覆盖的分组归入"工具"
    for (const group of nodeGroups) {
      let matched = false;
      for (const sc of SUPER_CATEGORIES) {
        if (sc.categories.includes(group.key)) {
          map[sc.key].push(group);
          matched = true;
          break;
        }
      }
      if (!matched) {
        // 未匹配的归入工具分类
        map['tools'].push(group);
      }
    }
    return map;
  }, [nodeGroups]);

  // 当前激活的超级分类下的分组
  const currentGroups = activeSuperKey ? (groupedBySuper[activeSuperKey] ?? []) : [];

  // 当切换超级分类时，自动选中第一个 Tab
  useEffect(() => {
    if (activeSuperKey) {
      const groups = groupedBySuper[activeSuperKey] ?? [];
      setActiveTabKey(groups.length > 0 ? groups[0].key : null);
      setSearchQuery('');
    }
  }, [activeSuperKey, groupedBySuper]);

  // 当前 Tab 下的节点
  const currentTabItems = useMemo(() => {
    if (!activeTabKey) return [];
    const group = currentGroups.find((g) => g.key === activeTabKey);
    return group?.items ?? [];
  }, [activeTabKey, currentGroups]);

  // 搜索过滤结果（跨当前超级分类的所有二级分类）
  const searchResults = useMemo(() => {
    if (!searchQuery.trim()) return [];
    const query = searchQuery.toLowerCase().trim();
    const results: NodeItem[] = [];
    for (const group of currentGroups) {
      for (const item of group.items) {
        if (item.name.toLowerCase().includes(query) || item.type.toLowerCase().includes(query)) {
          results.push(item);
        }
      }
    }
    return results;
  }, [searchQuery, currentGroups]);

  // 实际展示的节点列表
  const displayItems = searchQuery.trim() ? searchResults : currentTabItems;

  /** 拖拽开始：设置 dataTransfer */
  const handleDragStart = useCallback((e: React.DragEvent, type: NodeType) => {
    e.dataTransfer.setData('application/reactflow/nodeType', type);
    e.dataTransfer.effectAllowed = 'move';
  }, []);

  /** 点击添加节点：设置待添加类型，由 FlowCanvas 在视口中心处理 */
  const handleClick = useCallback((type: NodeType) => {
    if (!flow) return;
    setPendingAddNodeType(type);
  }, [flow, setPendingAddNodeType]);

  /** 切换超级分类 */
  const handleSuperClick = (key: string) => {
    setActiveSuperKey((prev) => prev === key ? null : key);
  };

  /** 点击外部关闭面板 */
  useEffect(() => {
    if (!activeSuperKey) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        setActiveSuperKey(null);
      }
    };

    // 延迟绑定，避免本次点击立即触发
    const timer = setTimeout(() => {
      document.addEventListener('mousedown', handleClickOutside);
    }, 0);

    return () => {
      clearTimeout(timer);
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [activeSuperKey]);

  /** Esc 关闭面板 */
  useEffect(() => {
    if (!activeSuperKey) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setActiveSuperKey(null);
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [activeSuperKey]);

  // 是否显示搜索（只在有搜索词时隐藏 Tab）
  const isSearching = searchQuery.trim().length > 0;

  return (
    <div
      ref={panelRef}
      className={cn(
        'absolute bottom-6 left-1/2 -translate-x-1/2 z-10',
        'flex flex-col items-center',
      )}
    >
      {/* 展开面板（向上弹出） */}
      {activeSuperKey && (
        <div
          className={cn(
            'mb-2 w-[360px]',
            'bg-background/95 backdrop-blur-md rounded-2xl',
            'shadow-xl border border-border/50',
            'animate-in fade-in-0 slide-in-from-bottom-2 duration-200',
            'overflow-hidden',
          )}
        >
          {/* 搜索框 */}
          <div className="px-3 pt-3 pb-1">
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground pointer-events-none" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="搜索节点..."
                className={cn(
                  'w-full pl-8 pr-3 py-2 text-sm',
                  'bg-muted/50 rounded-xl',
                  'border border-border/40',
                  'placeholder:text-muted-foreground/60',
                  'focus:outline-none focus:ring-1 focus:ring-ring/40 focus:border-border',
                  'transition-all duration-150',
                )}
              />
            </div>
          </div>

          {/* Tab 栏（搜索时隐藏） */}
          {!isSearching && currentGroups.length > 1 && (
            <div className="px-2 py-1">
              <div className="flex gap-0.5 overflow-x-auto scrollbar-none">
                {currentGroups.map((group) => {
                  const TabIcon = group.icon;
                  const isActive = activeTabKey === group.key;
                  return (
                    <button
                      key={group.key}
                      type="button"
                      onClick={() => setActiveTabKey(group.key)}
                      className={cn(
                        'flex items-center gap-1 px-2.5 py-1.5 rounded-lg',
                        'text-xs whitespace-nowrap shrink-0',
                        'transition-all duration-150 cursor-pointer',
                        isActive
                          ? 'bg-accent text-accent-foreground font-medium'
                          : 'text-muted-foreground hover:text-foreground hover:bg-muted/50',
                      )}
                    >
                      <TabIcon className="w-3 h-3" />
                      {group.label}
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {/* 分隔线 */}
          <div className="mx-3 border-t border-border/30" />

          {/* 节点网格 */}
          <div className="p-2 max-h-[280px] overflow-y-auto">
            {displayItems.length > 0 ? (
              <div
                className="grid gap-1"
                style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(90px, 1fr))' }}
              >
                {displayItems.map((item) => (
                  <NodeCard
                    key={item.type}
                    item={item}
                    onDragStart={handleDragStart}
                    onClick={handleClick}
                  />
                ))}
              </div>
            ) : (
              <div className="py-6 text-center text-xs text-muted-foreground">
                {isSearching ? '没有匹配的节点' : '暂无节点'}
              </div>
            )}
          </div>
        </div>
      )}

      {/* 底部工具条 */}
      <div
        className={cn(
          'flex items-center gap-0',
          'bg-background/90 backdrop-blur-sm rounded-2xl px-1 py-1',
          'shadow-lg border border-border/50',
        )}
      >
        {SUPER_CATEGORIES.map((sc) => {
          const Icon = sc.icon;
          const isActive = activeSuperKey === sc.key;
          // 检查该超级分类下是否有分组数据
          const hasGroups = (groupedBySuper[sc.key] ?? []).length > 0;
          if (!hasGroups) return null;

          return (
            <button
              key={sc.key}
              type="button"
              onClick={() => handleSuperClick(sc.key)}
              title={sc.label}
              className={cn(
                'flex flex-col items-center justify-center',
                'w-14 h-12 rounded-xl',
                'transition-all duration-150 cursor-pointer',
                'select-none',
                isActive
                  ? 'bg-accent text-accent-foreground scale-[1.02]'
                  : 'text-foreground hover:bg-accent/50 hover:scale-105',
                'active:scale-95',
              )}
            >
              <Icon className="w-5 h-5" />
              <span className="text-[10px] text-muted-foreground mt-0.5 leading-none">
                {sc.label}
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
