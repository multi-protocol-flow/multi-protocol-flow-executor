/**
 * NodePalette - 节点选择器浮动弹窗
 *
 * 由工具栏按钮或右键画布触发，以浮动弹窗形式展示所有可用节点类型。
 * 支持搜索、超级分类/二级分类 Tab 切换、拖拽/点击添加节点。
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
  X,
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
import { useFlowStore, useUIStore } from '@/stores';
import type { NodeType } from '@/types';
import type { NodeMetadata } from '@/types/nodeRegistry';
import {
  getNodeMetadata,
  groupByCategory,
  CATEGORY_NAMES,
  getCategoryOrder,
  getNodeTypeOrder,
} from '@/lib/NodeRegistry';
import { ICON_MAP as NODE_TYPE_ICON_MAP, getNodeDisplayName } from '@/lib/nodeMeta';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import { useEffect, useMemo, useState, useCallback } from 'react';

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
const LL = getLL();

export const FALLBACK_NODE_GROUPS: NodeGroup[] = [
  {
    key: 'http',
    label: 'HTTP',
    icon: Globe,
    items: [{ type: 'http', name: LL.nodes.http(), icon: Globe }],
  },
  {
    key: 'websocket',
    label: 'WS',
    icon: Wifi,
    items: [
      { type: 'ws_connect', name: LL.nodes.ws_connect(), icon: Wifi },
      { type: 'ws_send_collect', name: LL.nodes.ws_send_collect(), icon: Send },
      { type: 'ws_close', name: LL.nodes.ws_close(), icon: Plug },
    ],
  },
  {
    key: 'grpc',
    label: 'gRPC',
    icon: Network,
    items: [
      { type: 'grpc_connect', name: LL.nodes.grpc_connect(), icon: Cable },
      { type: 'grpc_call', name: LL.nodes.grpc_call(), icon: PhoneOutgoing },
      { type: 'grpc_close', name: LL.nodes.grpc_close(), icon: PhoneOff },
    ],
  },
  {
    key: 'database',
    label: 'DB',
    icon: Database,
    items: [
      { type: 'db_connect', name: LL.nodes.db_connect(), icon: Database },
      { type: 'db_begin', name: LL.nodes.db_begin(), icon: CirclePlay },
      { type: 'db_query', name: LL.nodes.db_query(), icon: Database },
      { type: 'db_commit', name: LL.nodes.db_commit(), icon: CheckCircle },
      { type: 'db_rollback', name: LL.nodes.db_rollback(), icon: RotateCcw },
      { type: 'db_close', name: LL.nodes.db_close(), icon: Plug },
    ],
  },
  {
    key: 'tools',
    label: 'Tool',
    icon: Wrench,
    items: [
      { type: 'conditional', name: LL.nodes.conditional(), icon: GitBranch },
      { type: 'script', name: LL.nodes.script(), icon: Code },
      { type: 'assertion', name: LL.nodes.assertion(), icon: CheckCircle },
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
          // 使用 i18n 翻译名称，回退到后端 display_name
          name: getNodeDisplayName(m.type_id),
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
  /** 包含的二级分类 key */
  categories: string[];
}

/** 将二级分类映射到 5 个一级分类 */
const SUPER_CATEGORIES: SuperCategory[] = [
  {
    key: 'network',
    categories: ['http', 'websocket', 'sse', 'graphql'],
  },
  {
    key: 'services',
    categories: ['grpc', 'mcp'],
  },
  {
    key: 'messaging',
    categories: ['mqtt', 'kafka', 'amqp'],
  },
  {
    key: 'tools',
    categories: ['tools', 'script', 'assertion'],
  },
  {
    key: 'other',
    categories: ['database', 'email', 'tcp', 'udp'],
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

/** 获取超级分类标签（通过 i18n） */
function getSuperCategoryLabel(key: string): string {
  const ll = getLL();
  const labels: Record<string, () => string> = {
    network: () => ll.nodes.superCategory.network(),
    services: () => ll.nodes.superCategory.services(),
    messaging: () => ll.nodes.superCategory.messaging(),
    tools: () => ll.nodes.superCategory.tools(),
    other: () => ll.nodes.superCategory.other(),
  };
  return labels[key]?.() ?? key;
}

/** 面板宽高常量 */
const PALETTE_WIDTH = 400;
const PALETTE_MAX_HEIGHT = 420;

// ============================================================================
// 搜索增强工具
// ============================================================================

/** 关键词别名映射 */
const SEARCH_ALIASES: Record<string, string[]> = {
  rest: ['http'],
  pubsub: ['mqtt', 'kafka'],
  stream: ['kafka_consume', 'kafka_produce', 'sse_listen'],
  queue: ['mqtt_subscribe', 'amqp_consume'],
  trigger: ['mqtt_subscribe', 'sse_listen'],
  sql: ['db_query'],
  nosql: ['db_query'],
  rpc: ['grpc_call'],
  tool: ['mcp_call_tool'],
  email: ['smtp_send', 'imap_fetch'],
  realtime: ['ws_connect', 'sse_listen', 'mqtt_subscribe'],
  schedule: ['mqtt_subscribe', 'kafka_consume'],
  crud: ['http', 'db_query'],
};

/** 别名 → 所有展开的节点类型 */
function expandAliases(query: string): string[] {
  const q = query.toLowerCase().trim();
  return SEARCH_ALIASES[q] ?? [];
}

/** 简单编辑距离（Levenshtein） */
function levenshtein(a: string, b: string): number {
  const m = a.length, n = b.length;
  const dp: number[][] = Array.from({ length: m + 1 }, () => Array(n + 1).fill(0));
  for (let i = 0; i <= m; i++) dp[i][0] = i;
  for (let j = 0; j <= n; j++) dp[0][j] = j;
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      dp[i][j] = a[i - 1] === b[j - 1]
        ? dp[i - 1][j - 1]
        : 1 + Math.min(dp[i - 1][j], dp[i][j - 1], dp[i - 1][j - 1]);
    }
  }
  return dp[m][n];
}

/** 模糊匹配评分（0=不匹配，值越高越匹配） */
function fuzzyScore(query: string, target: string): number {
  const q = query.toLowerCase();
  const t = target.toLowerCase();
  // 完全包含：最高分
  if (t.includes(q)) return 100;
  // 编辑距离 ≤ 2：次高分（距离越小越高）
  const dist = levenshtein(q, t);
  if (dist <= 2) return 50 - dist * 15;
  return 0;
}

// ============================================================================
// 使用频率追踪
// ============================================================================

const USAGE_KEY = 'node-palette-usage';

/** 记录节点使用 */
function recordNodeUsage(type: NodeType) {
  try {
    const raw = localStorage.getItem(USAGE_KEY);
    const counts: Record<string, number> = raw ? JSON.parse(raw) : {};
    counts[type] = (counts[type] || 0) + 1;
    // 只保留最近 50 条，防止无限增长
    const sorted = Object.entries(counts).sort((a, b) => b[1] - a[1]).slice(0, 50);
    localStorage.setItem(USAGE_KEY, JSON.stringify(Object.fromEntries(sorted)));
  } catch { /* ignore */ }
}

/** 获取使用频率排序（高频在前） */
function getUsageOrder(type: NodeType): number {
  try {
    const raw = localStorage.getItem(USAGE_KEY);
    if (!raw) return 0;
    const counts: Record<string, number> = JSON.parse(raw);
    return -(counts[type] || 0);
  } catch {
    return 0;
  }
}

/**
 * 节点选择器浮动弹窗组件
 */
export function NodePalette() {
  const setPendingAddNodeType = useFlowStore((s) => s.setPendingAddNodeType);
  const flow = useFlowStore((s) => s.flow);
  const isOpen = useUIStore((s) => s.isNodePaletteOpen);
  const palettePosition = useUIStore((s) => s.nodePalettePosition);
  const closePalette = useUIStore((s) => s.closeNodePalette);

  // 动态元数据状态
  const [metadata, setMetadata] = useState<NodeMetadata[]>([]);
  // 当前激活的超级分类 key，默认 'network'
  const [activeSuperKey, setActiveSuperKey] = useState<string>('network');
  // 当前激活的二级分类 Tab key
  const [activeTabKey, setActiveTabKey] = useState<string | null>(null);
  // 搜索关键词
  const [searchQuery, setSearchQuery] = useState('');

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
        map['tools'].push(group);
      }
    }
    return map;
  }, [nodeGroups]);

  const currentGroups = useMemo(
    () => (activeSuperKey ? (groupedBySuper[activeSuperKey] ?? []) : []),
    [activeSuperKey, groupedBySuper]
  );

  // 当切换超级分类时，自动选中第一个 Tab
  useEffect(() => {
    if (!activeSuperKey) return;
    const groups = groupedBySuper[activeSuperKey] ?? [];
    setActiveTabKey(groups.length > 0 ? groups[0].key : null);
    setSearchQuery('');
  }, [activeSuperKey, groupedBySuper]);

  // 打开时重置状态
  useEffect(() => {
    if (isOpen) {
      setSearchQuery('');
      setActiveSuperKey('network');
    }
  }, [isOpen]);

  // 当前 Tab 下的节点
  const currentTabItems = useMemo(() => {
    if (!activeTabKey) return [];
    const group = currentGroups.find((g) => g.key === activeTabKey);
    return group?.items ?? [];
  }, [activeTabKey, currentGroups]);

  // 搜索时跨所有分类
  const allItems = useMemo(() => {
    const items: NodeItem[] = [];
    for (const group of nodeGroups) {
      for (const item of group.items) {
        items.push(item);
      }
    }
    return items;
  }, [nodeGroups]);

  const searchResults = useMemo(() => {
    if (!searchQuery.trim()) return [];
    const query = searchQuery.toLowerCase().trim();
    const scored: { item: NodeItem; score: number }[] = [];

    // 1. 别名展开
    const aliasTypes = expandAliases(query);

    for (const item of allItems) {
      let bestScore = 0;

      // 2. 精确包含匹配
      const nameScore = fuzzyScore(query, item.name);
      const typeScore = fuzzyScore(query, item.type);
      bestScore = Math.max(nameScore, typeScore);

      // 3. 别名匹配
      if (aliasTypes.includes(item.type)) {
        bestScore = Math.max(bestScore, 90);
      }

      if (bestScore > 0) {
        scored.push({ item, score: bestScore });
      }
    }

    // 4. 按分数排序（同分时按使用频率排序）
    scored.sort((a, b) => {
      if (b.score !== a.score) return b.score - a.score;
      return getUsageOrder(a.item.type) - getUsageOrder(b.item.type);
    });

    return scored.map((s) => s.item);
  }, [searchQuery, allItems]);

  // 实际展示的节点列表
  const displayItems = searchQuery.trim() ? searchResults : currentTabItems;

  /** 拖拽开始：设置 dataTransfer，延迟关闭弹窗释放画布 */
  const handleDragStart = useCallback((e: React.DragEvent, type: NodeType) => {
    e.dataTransfer.setData('application/reactflow/nodeType', type);
    e.dataTransfer.effectAllowed = 'move';
    // 更新使用频率
    recordNodeUsage(type);
    // 延迟到下一帧关闭弹窗，确保 dragstart 完成后再移除遮罩
    requestAnimationFrame(() => {
      useUIStore.getState().closeNodePalette();
    });
  }, []);

  /** 点击添加节点：设置待添加类型，由 FlowCanvas 在视口中心处理 */
  const handleClick = useCallback((type: NodeType) => {
    if (!flow) return;
    recordNodeUsage(type);
    setPendingAddNodeType(type);
  }, [flow, setPendingAddNodeType]);

  /** Esc 关闭 */
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        closePalette();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, closePalette]);

  // 是否显示搜索（只在有搜索词时隐藏 Tab）
  const isSearching = searchQuery.trim().length > 0;

  // 计算弹窗位置：有指定位置则靠右偏移；否则屏幕居中
  const popupStyle: React.CSSProperties = useMemo(() => {
    if (palettePosition) {
      // 右键触发：弹窗出现在光标右下，避免溢出视口
      const left = Math.min(palettePosition.x + 12, window.innerWidth - PALETTE_WIDTH - 20);
      const top = Math.min(palettePosition.y + 8, window.innerHeight - PALETTE_MAX_HEIGHT - 20);
      return {
        position: 'fixed',
        left: `${Math.max(left, 20)}px`,
        top: `${Math.max(top, 20)}px`,
        width: `${PALETTE_WIDTH}px`,
      };
    }
    // 工具栏按钮触发：屏幕居中
    return {
      position: 'fixed',
      left: '50%',
      top: '50%',
      transform: 'translate(-50%, -50%)',
      width: `${PALETTE_WIDTH}px`,
    };
  }, [palettePosition]);

  if (!isOpen) return null;

  return (
    <>
      {/* 半透明遮罩 */}
      <div
        className="fixed inset-0 z-[60] bg-black/20 cursor-default"
        onClick={closePalette}
      />

      {/* 弹窗 */}
      <div
        style={popupStyle}
        className={cn(
          'z-[70]',
          'bg-background/98 backdrop-blur-md rounded-2xl',
          'shadow-2xl border border-border/50',
          'animate-in fade-in-0 zoom-in-95 duration-150',
          'overflow-hidden',
        )}
        onClick={(e) => e.stopPropagation()}
      >
        {/* 标题栏 */}
        <div className="flex items-center justify-between px-3 pt-3 pb-1">
          <div className="flex items-center gap-1.5">
            <Search className="w-3.5 h-3.5 text-muted-foreground" />
            <span className="text-sm font-medium text-foreground">
              {getLL().nodes.add_node_title()}
            </span>
          </div>
          <button
            type="button"
            onClick={closePalette}
            className="p-1 rounded-md hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
            title={getLL().common.CANCEL()}
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* 搜索框 */}
        <div className="px-3 pt-1 pb-2">
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground pointer-events-none" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={LL.flow.search_placeholder()}
              className={cn(
                'w-full pl-8 pr-3 py-2 text-sm',
                'bg-muted/50 rounded-xl',
                'border border-border/40',
                'placeholder:text-muted-foreground/60',
                'focus:outline-none focus:ring-1 focus:ring-ring/40 focus:border-border',
                'transition-all duration-150',
              )}
              autoFocus
            />
          </div>
        </div>

        {/* 超级分类 Tab 栏 */}
        <div className="px-2 py-1">
          <div className="flex gap-0.5 overflow-x-auto scrollbar-none">
            {SUPER_CATEGORIES.map((sc) => {
              const hasGroups = (groupedBySuper[sc.key] ?? []).length > 0;
              if (!hasGroups) return null;
              const isActive = activeSuperKey === sc.key;
              return (
                <button
                  key={sc.key}
                  type="button"
                  onClick={() => setActiveSuperKey(sc.key)}
                  className={cn(
                    'px-2.5 py-1.5 rounded-lg',
                    'text-xs whitespace-nowrap shrink-0',
                    'transition-all duration-150 cursor-pointer',
                    isActive
                      ? 'bg-accent text-accent-foreground font-medium'
                      : 'text-muted-foreground hover:text-foreground hover:bg-muted/50',
                  )}
                >
                  {getSuperCategoryLabel(sc.key)}
                </button>
              );
            })}
          </div>
        </div>

        {/* 二级分类 Tab 栏（搜索时隐藏） */}
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
              {isSearching ? LL.flow.no_matching_nodes() : LL.nodes.no_nodes_available()}
            </div>
          )}
        </div>

        {/* 右键提示 */}
        {!palettePosition && (
          <div className="px-3 pb-2 pt-0 text-[10px] text-muted-foreground/70 text-center">
            {LL.flow.right_click_hint()}
          </div>
        )}
      </div>
    </>
  );
}